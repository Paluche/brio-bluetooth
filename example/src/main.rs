use std::error::Error;

use brio_smart_tech::{BrioSmartTech, Color};
use btleplug::{api::Manager as _, platform::Manager};
use strum::IntoEnumIterator;
use tokio::time::{Duration, sleep};

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Initializing BLE manager");
    let manager = Manager::new().await.unwrap();
    // Get the first bluetooth adapter
    let adapters = manager.adapters().await.unwrap();
    let central = adapters.first().unwrap();

    println!("Searching for train");
    let train = BrioSmartTech::new(central)
        .await?;

    println!("Sending different colors");
    for c in Color::iter() {
        println!("Color {c:?}");
        for i in 0..16 {
            train.lock().await.set_color(c, i).await?;
            sleep(Duration::from_millis(100)).await;
        }
    }

    train.lock().await.set_color(Color::White, 15).await?;
    sleep(Duration::from_millis(300)).await;

    println!("Forward");
    train.lock().await.forward(7).await?;
    sleep(Duration::from_secs(10)).await;

    println!("Backward");
    train.lock().await.backward(7).await?;
    sleep(Duration::from_secs(1)).await;

    println!("Stop");
    train.lock().await.stop().await?;
    sleep(Duration::from_millis(300)).await;

    Ok(())
}
