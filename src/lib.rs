use std::error::Error;
use uuid::Uuid;
use tokio::time::{sleep, Duration};
use btleplug::{api::{Central, Characteristic, Peripheral as _, ScanFilter, WriteType}, platform::{Adapter, Peripheral}};
use strum::EnumIter;

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
    fn get_command_value(&self, intensity:u8) -> u8 {
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
        } as u8) * 16 + intensity
    }
}

pub struct BrioSmartTech {
    device: Peripheral,
    cmd_char: Characteristic,
}

async fn find_device(central: &Adapter) -> Option<Peripheral> {
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

impl BrioSmartTech {
    pub async fn new(central: &Adapter) -> Result<Option<Self>, Box<dyn Error>> {
        // service and characteristic have the same uuid for the brio smart 2.0
        let service_id = Uuid::parse_str(
            "B11B0002-BF9B-4A20-BA07-9218FEC577D7"
        ).unwrap();

        //println!("Scanning for devices with service ID: {service_id}");
        central.start_scan(ScanFilter::default()).await.unwrap();

        // Wait a bit to collect some devices
        sleep(Duration::from_secs(2)).await;

        let timeout = Duration::from_secs(30);
        let start = std::time::Instant::now();

        let mut device  = None;

        while start.elapsed() < timeout {
            if let Some(d) = find_device(central).await {
                device = Some(d);
                break;
            }
            sleep(Duration::from_millis(500)).await;
        }

        if device.is_none() {
            return Ok(None)
        }
        let device = device.unwrap();
        device.connect().await?;
        device.discover_services().await?;

        let cmd_char = device.characteristics().iter().
            find(|c| c.uuid == service_id).expect("Could not find command characteristic").to_owned();

        Ok(Some(Self{
            device,
            cmd_char,
        }))
    }

    async fn write_command(&self, mut data: Vec<u8>) -> Result<(), Box<dyn Error>> {
        let sum: u16 = data.iter().map(|x| u16::from(*x)).sum();
        data.insert(0, 0xAA);
        data.push(((0x100 - (sum & 0xFF)) & 0xFF) as u8);

        self.device.write(
            &self.cmd_char,
            &data,
            WriteType::WithoutResponse
        ).await?;
        Ok(())
    }

    pub async fn set_speed(&self, speed: u8) -> Result<(), Box<dyn Error>> {
        self.write_command(vec![0x02, 0x01, speed]).await
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

    pub async fn set_color(&self, color: Color, intensity:u8) -> Result<(), Box<dyn Error>> {
        assert!(intensity <= 16);
        self.write_command(vec![0x02, 0x02, color.get_command_value(intensity)]).await
    }
}
