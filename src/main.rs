use std::error::Error;

use btleplug::{ api::Manager as _, platform:: Manager};
use tokio::time::{sleep, Duration};
use brio_bluetooth::BrioSmartTech;

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
    for c in 1..255 {
        train.set_color(c).await?;
        sleep(Duration::from_millis(70)).await;
    }

    train.set_color(0).await?;
    sleep(Duration::from_millis(300)).await;

    println!("Forward");
    for i in 1..8 {
        train.forward(i).await?;
        sleep(Duration::from_secs(1)).await;
    }

    println!("Backward");
    for i in 1..8 {
        train.backward(i).await?;
        sleep(Duration::from_secs(1)).await;
    }

    println!("Stop");
    train.stop().await?;
    sleep(Duration::from_millis(300)).await;

    Ok(())
}
