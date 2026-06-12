use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;

pub const CONFIG_FILE: &str = "spark_config.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub spark_mac: String,
    #[serde(default)]
    pub spark_name: String,
    #[serde(default)]
    pub midi_name: String,
    #[serde(default = "default_led_pin")]
    pub led_pin: u8,
    
    #[serde(flatten)]
    pub mappings: HashMap<String, String>,
}

fn default_led_pin() -> u8 {
    17
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            spark_mac: String::new(),
            spark_name: String::new(),
            midi_name: String::new(),
            led_pin: default_led_pin(),
            mappings: HashMap::new(),
        }
    }
}

impl AppConfig {
    /// Load configuration from CONFIG_FILE. If file doesn't exist or is invalid, returns default configuration.
    pub fn load() -> Self {
        if let Ok(content) = fs::read_to_string(CONFIG_FILE) {
            if let Ok(config) = serde_json::from_str::<AppConfig>(&content) {
                return config;
            }
        }
        AppConfig::default()
    }

    /// Save current configuration to CONFIG_FILE.
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let content = serde_json::to_string_pretty(self)?;
        fs::write(CONFIG_FILE, content)?;
        Ok(())
    }

    /// Check if configuration contains essential settings.
    pub fn is_valid(&self) -> bool {
        !self.spark_mac.is_empty() && !self.midi_name.is_empty()
    }

    /// Extract key-value button mappings (e.g. "btn1" -> "Preset 1") and parse them into a HashMap of button ID -> action.
    pub fn get_button_mappings(&self) -> HashMap<u8, String> {
        let mut map = HashMap::new();
        for (k, v) in &self.mappings {
            if k.starts_with("btn") {
                if let Ok(btn_id) = k[3..].parse::<u8>() {
                    map.insert(btn_id, v.clone());
                }
            }
        }
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.spark_mac, "");
        assert_eq!(config.spark_name, "");
        assert_eq!(config.midi_name, "");
        assert_eq!(config.led_pin, 17);
        assert!(config.mappings.is_empty());
        assert!(!config.is_valid());
    }

    #[test]
    fn test_is_valid() {
        let mut config = AppConfig::default();
        assert!(!config.is_valid());
        config.spark_mac = "00:11:22:33:44:55".to_string();
        assert!(!config.is_valid());
        config.midi_name = "M-Vave Chocolate".to_string();
        assert!(config.is_valid());
    }

    #[test]
    fn test_get_button_mappings() {
        let mut config = AppConfig::default();
        config.mappings.insert("spark_mac".to_string(), "00:11:22:33:44:55".to_string());
        config.mappings.insert("btn12".to_string(), "Preset 1".to_string());
        config.mappings.insert("btn14".to_string(), "Preset 2".to_string());
        config.mappings.insert("random_key".to_string(), "some_value".to_string());

        let btn_map = config.get_button_mappings();
        assert_eq!(btn_map.len(), 2);
        assert_eq!(btn_map.get(&12), Some(&"Preset 1".to_string()));
        assert_eq!(btn_map.get(&14), Some(&"Preset 2".to_string()));
    }
}

