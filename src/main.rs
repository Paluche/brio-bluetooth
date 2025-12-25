use std::error::Error;

use btleplug::{ api::Manager as _, platform:: Manager};
use tokio::time::{sleep, Duration};
use brio_bluetooth::{BrioSmartTech, Color};
use strum::IntoEnumIterator;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Initializing BLE manager");
    let manager = Manager::new().await.unwrap();
    // Get the first bluetooth adapter
    let adapters = manager.adapters().await.unwrap();
    let central= adapters.first().unwrap();

    println!("Searching for train");
    let train = BrioSmartTech::new(central).await?.expect("device not found");

    println!("Sending different colors");
    for c in Color::iter() {
        println!("Color {c:?}");
        for i in 0..16 {
            train.set_color(c, i).await?;
            sleep(Duration::from_millis(100)).await;
        }
    }

    train.set_color(Color::White, 15).await?;
    sleep(Duration::from_millis(300)).await;

    println!("Forward");
    train.forward(7).await?;
    sleep(Duration::from_secs(10)).await;

    println!("Backward");
    train.backward(7).await?;
    sleep(Duration::from_secs(1)).await;

    println!("Stop");
    train.stop().await?;
    sleep(Duration::from_millis(300)).await;

    Ok(())
}
