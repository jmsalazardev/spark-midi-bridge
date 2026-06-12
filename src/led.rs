use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use log::info;

#[cfg(target_os = "linux")]
use std::sync::atomic::Ordering;
#[cfg(target_os = "linux")]
use tokio::time::{sleep, Duration};
#[cfg(target_os = "linux")]
use log::warn;

/// Spawns a background task to manage the physical status LED via GPIO.
/// Blinks slowly (500ms) if one or both connections are down.
/// Solid on (200ms check) if both are connected.
#[cfg(target_os = "linux")]
pub fn spawn_led_task(
    led_pin_num: u8,
    spark_ready: Arc<AtomicBool>,
    midi_ready: Arc<AtomicBool>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        match rppal::gpio::Gpio::new() {
            Ok(gpio) => {
                match gpio.get(led_pin_num).map(|p| p.into_output()) {
                    Ok(mut pin) => {
                        info!("Physical status LED initialized on GPIO pin {}.", led_pin_num);
                        let mut is_on = false;
                        loop {
                            let spark = spark_ready.load(Ordering::Relaxed);
                            let midi = midi_ready.load(Ordering::Relaxed);
                            if spark && midi {
                                pin.set_high();
                                sleep(Duration::from_millis(200)).await;
                            } else {
                                is_on = !is_on;
                                if is_on {
                                    pin.set_high();
                                } else {
                                    pin.set_low();
                                }
                                sleep(Duration::from_millis(500)).await;
                            }
                        }
                    }
                    Err(e) => {
                        warn!("Could not configure GPIO pin {} as output: {}. LED status disabled.", led_pin_num, e);
                    }
                }
            }
            Err(e) => {
                warn!("Could not initialize GPIO interface (rppal): {}. Running without physical LED status.", e);
            }
        }
    })
}

/// Dummy spawn_led_task implementation for non-Linux OS targets where physical GPIO is not available.
#[cfg(not(target_os = "linux"))]
pub fn spawn_led_task(
    _led_pin_num: u8,
    _spark_ready: Arc<AtomicBool>,
    _midi_ready: Arc<AtomicBool>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        info!("Physical status LED is not supported on this platform.");
    })
}
