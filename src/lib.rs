use std::{error::Error, sync::Arc};

use btleplug::{
    api::{
        Central, CharPropFlags, Characteristic, Peripheral as _, ScanFilter,
        WriteType,
    },
    platform::{Adapter, Peripheral},
};
use futures::stream::StreamExt;
use strum::EnumIter;
use tokio::{
    sync::Mutex,
    task,
    time::{Duration, sleep},
};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, EnumIter)]
pub enum Color {
    Off,
    Yellow,
    Orange,
    Red,
    Pink,
    Purple,
    Blue,
    LightBlue,
    Cyan,
    Green,
    White,
    RedBackward,
}

impl Color {
    fn get_command_value(&self, intensity: u8) -> u8 {
        assert!(intensity < 16);
        (match self {
            Self::Off => 0,
            Self::Yellow => 1,
            Self::Orange => 2,
            Self::Red => 3,
            Self::Pink => 4,
            Self::Purple => 5,
            Self::Blue => 6,
            Self::LightBlue => 7,
            Self::Cyan => 8,
            Self::Green => 9,
            Self::White => 10,
            Self::RedBackward => 11,
        } as u8)
            * 16
            + intensity
    }

    pub fn next(&self) -> Self {
        match self {
            Self::Off => Self::Yellow,
            Self::Yellow => Self::Orange,
            Self::Orange => Self::Red,
            Self::Red => Self::Pink,
            Self::Pink => Self::Purple,
            Self::Purple => Self::Blue,
            Self::Blue => Self::LightBlue,
            Self::LightBlue => Self::Cyan,
            Self::Cyan => Self::Green,
            Self::Green => Self::White,
            Self::White => Self::RedBackward,
            Self::RedBackward => Self::Off,
        }
    }
}

#[derive(Debug, Clone, Copy, EnumIter)]
pub enum SoundTheme {
    Honk,
    Whistle,
    Horn,
    Spaceship,
}

impl SoundTheme {
    pub fn get_command_value(&self) -> u8 {
        0xf0 + match self {
            Self::Honk => 0,
            Self::Whistle => 1,
            Self::Horn => 2,
            Self::Spaceship => 3,
        }
    }

    // offsets:
    // - eff1 0x15
    // - eff2 0x1d
    // - eff3 0x11
    // - eff4 0x19
    pub fn from_u8(value: u8, offset: u8) -> Option<Self> {
        match value - offset {
            0 => Some(Self::Honk),
            1 => Some(Self::Whistle),
            2 => Some(Self::Honk),
            3 => Some(Self::Spaceship),
            _ => None,
        }
    }
}

pub struct BrioSmartTech {
    peripheral: Peripheral,
    cmd_char: Characteristic,
}

async fn find_peripheral(central: &Adapter) -> Option<Peripheral> {
    for p in central.peripherals().await.unwrap() {
        if p.properties()
            .await
            .unwrap()
            .unwrap()
            .local_name
            .iter()
            .any(|name| name.contains("Smart 2.0"))
        {
            return Some(p);
        }
    }
    None
}

fn compute_checksum(payload: &[u8]) -> u8 {
    let sum: u16 = (payload.len() as u16)
        + payload.iter().map(|x| u16::from(*x)).sum::<u16>();
    ((0x100 - (sum & 0xFF)) & 0xFF) as u8
}

fn command_data(payload: Vec<u8>) -> Vec<u8> {
    // Insert first a byte indicating the number of bytes the payload has.
    // This byte enters in the computation of the checksum.
    let mut data = vec![0xaa, payload.len().try_into().unwrap()];
    let checksum = compute_checksum(&payload);
    data.extend(payload);
    data.push(checksum);

    data
}

async fn notification_watcher(brio_smart_tech: Arc<Mutex<BrioSmartTech>>) {
    // Print the first 4 notifications received.
    let mut notification_stream = brio_smart_tech
        .lock()
        .await
        .peripheral
        .notifications()
        .await
        .unwrap();
    // Process while the BLE connection is not broken or stopped.
    while let Some(data) = notification_stream.next().await {
        let mut bytes = data.value.iter();

        assert_eq!(*bytes.next().unwrap(), 0xaa);
        let len = *bytes.next().unwrap() as usize;
        let payload: Vec<u8> = bytes.by_ref().take(len).copied().collect();
        let checksum = *bytes.next().unwrap();

        assert_eq!(checksum, compute_checksum(&payload));
        assert_eq!(bytes.next(), None);

        println!("Data notified {payload:?}");
    }
}

impl BrioSmartTech {
    /// Instantiate the communication with a Brio Smart Tech device.
    pub async fn new(
        central: &Adapter,
    ) -> Result<Arc<Mutex<Self>>, Box<dyn Error>> {
        // service and characteristic have the same uuid for the brio smart 2.0
        let service_uuid =
            Uuid::parse_str("B11B0001-BF9B-4A20-BA07-9218FEC577D7").unwrap();
        let control_point_uuid =
            Uuid::parse_str("B11B0002-BF9B-4A20-BA07-9218FEC577D7").unwrap();
        let notification_uuid =
            Uuid::parse_str("B11B0002-BF9B-4A20-BA07-9218FEC577D7").unwrap();

        println!("Scanning for devices with service UUID: {service_uuid}");
        central
            .start_scan(ScanFilter {
                services: vec![service_uuid],
            })
            .await
            .unwrap();

        // Wait a bit to collect some peripherals.
        sleep(Duration::from_secs(2)).await;

        let peripheral;

        loop {
            if let Some(p) = find_peripheral(central).await {
                peripheral = p;
                break;
            }
            sleep(Duration::from_millis(500)).await;
        }

        central.stop_scan().await?;
        peripheral.connect().await?;
        peripheral.discover_services().await?;

        let mut cmd_char: Option<Characteristic> = None;

        for char in peripheral.characteristics() {
            if char.uuid == control_point_uuid {
                cmd_char = Some(char);
            } else if char.uuid == notification_uuid {
                assert!(
                    char.properties.contains(CharPropFlags::NOTIFY),
                    "Unexpected non-notify characteristic"
                );
                peripheral.subscribe(&char).await?;
            } else {
                continue;
            }
            break;
        }

        let ret = Arc::new(Mutex::new(Self {
            peripheral,
            cmd_char: cmd_char.expect("Could not find command characteristic"),
        }));

        task::spawn(notification_watcher(ret.clone()));

        Ok(ret)
    }

    pub async fn is_connected(&self) -> Result<bool, btleplug::Error> {
        self.peripheral.is_connected().await
    }

    async fn write_command(
        &self,
        payload: Vec<u8>,
    ) -> Result<(), Box<dyn Error>> {
        self.peripheral
            .write(
                &self.cmd_char,
                &command_data(payload),
                WriteType::WithoutResponse,
            )
            .await?;
        Ok(())
    }

    pub async fn set_speed(&self, speed: u8) -> Result<(), Box<dyn Error>> {
        self.write_command(vec![0x01, speed]).await
    }

    pub async fn forward(&self, speed: u8) -> Result<(), Box<dyn Error>> {
        assert!(speed != 0 && speed <= 7);
        self.set_speed(speed + 1).await
    }

    pub async fn backward(&self, speed: u8) -> Result<(), Box<dyn Error>> {
        assert!(speed != 0 && speed <= 7);
        self.set_speed(speed + 0x11).await
    }

    pub async fn stop(&self) -> Result<(), Box<dyn Error>> {
        self.set_speed(0).await
    }

    pub async fn set_color(
        &self,
        color: Color,
        intensity: u8,
    ) -> Result<(), Box<dyn Error>> {
        self.write_command(vec![0x02, color.get_command_value(intensity)])
            .await
    }

    pub async fn set_sound_theme(
        &self,
        sound_theme: SoundTheme,
    ) -> Result<(), Box<dyn Error>> {
        self.write_command(vec![0x56, 0xaa, sound_theme.get_command_value()])
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chksum() {
        assert_eq!(
            command_data(vec![0x02, Color::Blue.get_command_value(15)]),
            vec![0xaa, 0x02, 0x02, 0x6f, 0x8d]
        );
        assert_eq!(
            command_data(vec![0x02, Color::LightBlue.get_command_value(15)]),
            vec![0xaa, 0x02, 0x02, 0x7f, 0x7d]
        );

        assert_eq!(
            command_data(vec![0x01, 0x00]),
            vec![0xaa, 0x02, 0x01, 0x00, 0xfd]
        );
    }
}
