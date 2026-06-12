use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Manager, Peripheral};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use log::{info, warn, error};
use std::collections::HashMap;
use uuid::Uuid;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::error::Error;
use dialoguer::{Select, theme::ColorfulTheme, console::style};

pub const SPARK_MAC_PREFIX: &str = "F7:EB:ED";
pub const WRITE_UUID: &str = "0000ffc1-0000-1000-8000-00805f9b34fb";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SparkPreset {
    Preset1 = 0,
    Preset2 = 1,
    Preset3 = 2,
    Preset4 = 3,
}

impl SparkPreset {
    pub fn from_str(action: &str) -> Option<Self> {
        match action {
            "Preset 1" => Some(SparkPreset::Preset1),
            "Preset 2" => Some(SparkPreset::Preset2),
            "Preset 3" => Some(SparkPreset::Preset3),
            "Preset 4" => Some(SparkPreset::Preset4),
            _ => None,
        }
    }

    pub fn to_payload(self) -> Vec<u8> {
        vec![
            0x01, 0xfe, 0x00, 0x00, 0x53, 0xfe, 0x1a, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0xf0, 0x01, 0x3a, 0x15, 0x01, 0x38, 0x00, 0x00,
            self as u8, 0xf7,
        ]
    }
}

pub fn get_spark_action(action: &str) -> Option<Vec<u8>> {
    SparkPreset::from_str(action).map(|preset| preset.to_payload())
}

pub async fn get_spark_peripheral(manager: &Manager, saved_mac: &str) -> Option<Peripheral> {
    if let Ok(adapters) = manager.adapters().await {
        if let Some(adapter) = adapters.into_iter().nth(0) {
            let _ = adapter.start_scan(ScanFilter::default()).await;
            sleep(Duration::from_secs(5)).await;
            
            if let Ok(peripherals) = adapter.peripherals().await {
                for p in peripherals {
                    if let Ok(Some(props)) = p.properties().await {
                        let addr = p.address().to_string().to_uppercase();
                        let name = props.local_name.unwrap_or_default().to_lowercase();
                        
                        if (!saved_mac.is_empty() && addr == saved_mac.to_uppercase()) 
                            || (name.contains("spark") && !name.contains("audio")) 
                            || addr.starts_with(SPARK_MAC_PREFIX) {
                            return Some(p);
                        }
                    }
                }
            }
        }
    }
    None
}

pub async fn scan_and_select_spark() -> Result<(String, String), Box<dyn Error>> {
    let manager = Manager::new().await?;
    let adapters = manager.adapters().await?;
    if adapters.is_empty() {
        println!("{}", style("No Bluetooth adapters found.").red());
        return Ok((String::new(), String::new()));
    }
    let adapter = &adapters[0];

    loop {
        println!("\n{}", style("Scanning for Spark amplifiers (Bluetooth BLE)... (please wait ~5 seconds)").cyan());
        let _ = adapter.start_scan(ScanFilter::default()).await;
        sleep(Duration::from_secs(5)).await;
        
        let peripherals = adapter.peripherals().await?;
        let mut sparks = Vec::new();
        for p in peripherals {
            if let Ok(Some(props)) = p.properties().await {
                let addr = p.address().to_string();
                let name = props.local_name.unwrap_or_default();
                if (name.to_lowercase().contains("spark") && !name.to_lowercase().contains("audio")) 
                    || addr.to_uppercase().starts_with(SPARK_MAC_PREFIX) {
                    sparks.push((addr, name));
                }
            }
        }
        
        if sparks.is_empty() {
            println!("{}", style("No Spark amplifiers found nearby.").yellow());
        }
        
        let mut options = Vec::new();
        for (addr, name) in &sparks {
            options.push(format!("{} ({})", name, addr));
        }
        options.push("Rescan".to_string());
        options.push("Cancel and exit".to_string());

        let selection = tokio::task::spawn_blocking(move || {
            Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Choose a Spark amplifier")
                .items(&options)
                .default(0)
                .interact()
        }).await??;
        
        if selection < sparks.len() {
            return Ok((sparks[selection].0.clone(), sparks[selection].1.clone()));
        } else if selection == sparks.len() {
            continue;
        } else {
            return Ok((String::new(), String::new()));
        }
    }
}

pub async fn spark_connection_loop(
    rx: &mut mpsc::Receiver<u8>,
    saved_mac: String,
    mapping: HashMap<u8, String>,
    spark_ready: Arc<AtomicBool>,
) -> Result<(), Box<dyn Error>> {
    let manager = Manager::new().await?;
    let target_uuid = Uuid::parse_str(WRITE_UUID)?;
    
    loop {
        info!("Scanning Bluetooth for Spark...");
        if let Some(spark) = get_spark_peripheral(&manager, &saved_mac).await {
            let address = spark.address();
            info!("Connecting to Spark: {}", address);
            
            if spark.connect().await.is_ok() {
                sleep(Duration::from_secs(2)).await; 
                
                if spark.discover_services().await.is_ok() {
                    let chars = spark.characteristics();
                    let target_char = chars.iter().find(|c| c.uuid == target_uuid);
                    
                    if let Some(c) = target_char {
                        info!("Connection to Spark successful and ready.");
                        spark_ready.store(true, Ordering::Relaxed);
                        
                        let mut check_interval = tokio::time::interval(Duration::from_secs(2));
                        loop {
                            tokio::select! {
                                Some(btn_id) = rx.recv() => {
                                    info!("Received button press event: {}", btn_id);
                                    if !spark.is_connected().await.unwrap_or(false) {
                                        warn!("Spark disconnected while trying to send command.");
                                        break; 
                                    }
                                    
                                    if let Some(action) = mapping.get(&btn_id) {
                                        info!("Pedal: {} -> Spark: {}", btn_id, action);
                                        if let Some(payload) = get_spark_action(action) {
                                            if let Err(e) = spark.write(c, &payload, WriteType::WithResponse).await {
                                                error!("Error sending to Spark: {}", e);
                                            } else {
                                                info!("BLE command sent successfully to Spark.");
                                            }
                                        }
                                    } else {
                                        warn!("Button {} is not mapped in config. Check your spark_config.json mappings.", btn_id);
                                    }
                                }
                                _ = check_interval.tick() => {
                                    if !spark.is_connected().await.unwrap_or(false) {
                                        warn!("Spark connection lost.");
                                        break;
                                    }
                                }
                            }
                        }
                        spark_ready.store(false, Ordering::Relaxed);
                    }
                }
            } else {
                warn!("Failed to connect to Spark.");
            }
        }
        sleep(Duration::from_secs(5)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preset_from_str() {
        assert_eq!(SparkPreset::from_str("Preset 1"), Some(SparkPreset::Preset1));
        assert_eq!(SparkPreset::from_str("Preset 2"), Some(SparkPreset::Preset2));
        assert_eq!(SparkPreset::from_str("Preset 3"), Some(SparkPreset::Preset3));
        assert_eq!(SparkPreset::from_str("Preset 4"), Some(SparkPreset::Preset4));
        assert_eq!(SparkPreset::from_str("Preset 5"), None);
        assert_eq!(SparkPreset::from_str("invalid"), None);
    }

    #[test]
    fn test_preset_to_payload() {
        let preset1_payload = SparkPreset::Preset1.to_payload();
        assert_eq!(preset1_payload.len(), 26);
        assert_eq!(preset1_payload[24], 0); // preset index
        assert_eq!(preset1_payload[25], 0xf7); // Sysex end byte

        let preset4_payload = SparkPreset::Preset4.to_payload();
        assert_eq!(preset4_payload.len(), 26);
        assert_eq!(preset4_payload[24], 3); // preset index
        assert_eq!(preset4_payload[25], 0xf7); // Sysex end byte
    }

    #[test]
    fn test_get_spark_action() {
        assert!(get_spark_action("Preset 1").is_some());
        assert!(get_spark_action("invalid").is_none());
    }
}

