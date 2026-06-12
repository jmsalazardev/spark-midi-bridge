mod config;
mod spark;
mod midi;
mod led;
mod configurator;

use config::AppConfig;
use std::error::Error;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::mpsc;
use log::{info, error};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = std::env::args().collect();
    let config = AppConfig::load();

    if args.contains(&"--configure".to_string()) 
        || args.contains(&"-c".to_string()) 
        || !config.is_valid() 
    {
        if let Err(e) = configurator::run_configurator().await {
            eprintln!("Error running configurator: {}", e);
        }
        return Ok(());
    }

    println!("=== DEBUG: main start ===");
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();
    info!("=========================================");
    info!("STARTING SPARK MIDI HEADLESS            ");
    info!("=========================================");

    let spark_mac = config.spark_mac.clone();
    let midi_name = config.midi_name.clone();
    let mapping = config.get_button_mappings();
    let led_pin_num = config.led_pin;

    let (tx, mut rx) = mpsc::channel::<u8>(100);

    let spark_ready = Arc::new(AtomicBool::new(false));
    let midi_ready = Arc::new(AtomicBool::new(false));

    // Start physical status LED task
    let _led_task = led::spawn_led_task(led_pin_num, spark_ready.clone(), midi_ready.clone());

    // Start Bluetooth BLE engine task
    let spark_ready_task = spark_ready.clone();
    let spark_task = tokio::spawn(async move {
        if let Err(e) = spark::spark_connection_loop(&mut rx, spark_mac, mapping, spark_ready_task).await {
            error!("BLE Engine error: {}", e);
        }
    });

    // Start MIDI engine task
    let midi_ready_task = midi_ready.clone();
    let midi_task = tokio::spawn(async move {
        if let Err(e) = midi::midi_connection_loop(tx, midi_name, midi_ready_task).await {
            error!("MIDI Engine error: {}", e);
        }
    });

    let _ = tokio::join!(spark_task, midi_task);
    Ok(())
}
