use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter, WriteType};
use btleplug::platform::{Manager, Peripheral};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use log::{info, warn, error, debug};
use std::collections::HashMap;
use uuid::Uuid;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::error::Error;
use dialoguer::{Select, theme::ColorfulTheme, console::style};
use futures::stream::StreamExt;

pub const SPARK_AMPS: &[&str] = &[
    "RolandJC120", "Twin", "ADClean", "94MatchDCV2", "ODS50", // Clean
    "Bassman", "AC Boost", "Checkmate", "TwoStoneSP50",  // Glassy
    "Deluxe65", "Plexi", "OverDrivenJM45", "OverDrivenLuxVerb", "BluesBoy", // Crunch
    "Bogner", "OrangeAD30", "AmericanHighGain", "SLO100", "YJM100", // Hi Gain
    "Rectifier", "EVH", "SwitchAxeLead", "Invader", "BE101", "Insane6508", // Metal
    "Acoustic", "AcousticAmpV2", "FatAcousticV2", "FlatAcoustic", // Bass/Acoustic
    "GK800", "Sunny3000", "W600", "Hammer500",
    "JH.JTM45", "JH.Super100", "JH.Bassman50", "JH.DualShowman", "JH.Sun100S", "JH.SoundCity100" // Jimi Hendrix Pack
];

pub fn get_amp_user_name(amp_id: &str) -> &str {
    match amp_id {
        "RolandJC120" => "Silver 120",
        "Twin" => "Black Duo",
        "ADClean" => "AD Clean",
        "94MatchDCV2" => "MATCH DC",
        "ODS50" => "ODS 50",
        "Bassman" => "Tweed Bass",
        "AC Boost" => "AC Boost",
        "Checkmate" => "Checkmate",
        "TwoStoneSP50" => "Two Stone SP50",
        "Deluxe65" => "American Deluxe",
        "Plexi" => "Plexiglas",
        "OverDrivenJM45" => "JM45",
        "OverDrivenLuxVerb" => "Lux Verb",
        "BluesBoy" => "Blues Boy",
        "Bogner" => "RB 101",
        "OrangeAD30" => "British 30",
        "AmericanHighGain" => "American High Gain",
        "SLO100" => "SLO 100",
        "YJM100" => "YJM100",
        "Rectifier" => "Treadplate",
        "EVH" => "Insane",
        "SwitchAxeLead" => "Switch Axe",
        "InSwitchAxevader" => "Switch Axe",
        "Invader" => "Rocker V",
        "BE101" => "BE 101",
        "Insane6508" => "Insane 6508",
        "Acoustic" => "Pure Acoustic",
        "AcousticAmpV2" => "Fishboy",
        "FatAcousticV2" => "Jumbo",
        "FlatAcoustic" => "Flat Acoustic",
        "GK800" => "RB-800",
        "Sunny3000" => "Sunny 3000",
        "W600" => "W600",
        "Hammer500" => "Hammer 500",
        "JH.JTM45" => "J.H. 45/100",
        "JH.Super100" => "J.H. Super 100",
        "JH.Bassman50" => "J.H. Bass Master",
        "JH.DualShowman" => "J.H. D-Show Master",
        "JH.Sun100S" => "J.H. Sun 100S",
        "JH.SoundCity100" => "J.H. Tone City 100",
        other => other,
    }
}

pub fn decode_7bit(data7bit: &[u8]) -> Vec<u8> {
    let mut data8bit = Vec::new();
    let chunk_len = data7bit.len();
    let num_seq = (chunk_len + 7) / 8;
    for this_seq in 0..num_seq {
        let seq_start = this_seq * 8;
        if seq_start >= chunk_len {
            break;
        }
        let seq_len = std::cmp::min(8, chunk_len - seq_start);
        if seq_len <= 1 {
            break;
        }
        let bit8 = data7bit[seq_start];
        for ind in 0..(seq_len - 1) {
            let mut dat = data7bit[seq_start + ind + 1];
            if (bit8 & (1 << ind)) != 0 {
                dat |= 0x80;
            }
            data8bit.push(dat);
        }
    }
    data8bit
}

pub struct MsgPackReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> MsgPackReader<'a> {
    pub fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }
    
    pub fn read_byte(&mut self) -> Option<u8> {
        if self.pos < self.data.len() {
            let b = self.data[self.pos];
            self.pos += 1;
            Some(b)
        } else {
            None
        }
    }
    
    pub fn read_string(&mut self) -> Option<String> {
        let b = self.read_byte()?;
        let len = if b == 0xd9 {
            self.read_byte()? as usize
        } else if b >= 0xa0 && b <= 0xbf {
            (b - 0xa0) as usize
        } else {
            let next = self.read_byte()?;
            if next >= 0xa0 && next <= 0xbf {
                (next - 0xa0) as usize
            } else if next == 0xd9 {
                self.read_byte()? as usize
            } else {
                return None;
            }
        };
        
        if self.pos + len <= self.data.len() {
            let s = std::str::from_utf8(&self.data[self.pos..self.pos + len]).ok()?.to_string();
            self.pos += len;
            Some(s)
        } else {
            None
        }
    }
    
    pub fn read_float(&mut self) -> Option<f32> {
        let prefix = self.read_byte()?;
        if prefix != 0xca {
            return None;
        }
        if self.pos + 4 <= self.data.len() {
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(&self.data[self.pos..self.pos + 4]);
            self.pos += 4;
            Some(f32::from_be_bytes(bytes))
        } else {
            None
        }
    }
}

pub fn parse_active_amp_model(data8bit: &[u8]) -> Option<String> {
    let mut r = MsgPackReader::new(data8bit);
    r.read_byte()?; // skip 1 byte
    r.read_byte()?; // skip preset index
    
    let _uuid = r.read_string()?;
    let _name = r.read_string()?;
    let _version = r.read_string()?;
    let _descr = r.read_string()?;
    let _icon = r.read_string()?;
    let _bpm = r.read_float()?;
    
    let num_effects_byte = r.read_byte()?;
    let num_effects = if num_effects_byte >= 0x90 {
        num_effects_byte - 0x90
    } else {
        7
    };
    
    for i in 0..num_effects {
        let pedal_name = r.read_string()?;
        let _onoff = r.read_byte()?;
        
        if i == 3 {
            return Some(pedal_name);
        }
        
        let num_params_byte = r.read_byte()?;
        let num_params = if num_params_byte >= 0x90 {
            num_params_byte - 0x90
        } else {
            0
        };
        
        for _ in 0..num_params {
            r.read_byte()?;
            r.read_byte()?;
            r.read_float()?;
        }
    }
    
    None
}

pub fn extract_strings(data: &[u8]) -> Vec<String> {
    let mut strings = Vec::new();
    let mut i = 0;
    while i < data.len() {
        let b = data[i];
        if b >= 0xa0 && b <= 0xbf {
            let len = (b - 0xa0) as usize;
            if i + 1 + len <= data.len() {
                if let Ok(s) = std::str::from_utf8(&data[i + 1..i + 1 + len]) {
                    strings.push(s.to_string());
                }
                i += 1 + len;
            } else {
                i += 1;
            }
        } else if b == 0xd9 {
            if i + 1 < data.len() {
                let len = data[i + 1] as usize;
                if i + 2 + len <= data.len() {
                    if let Ok(s) = std::str::from_utf8(&data[i + 2..i + 2 + len]) {
                        strings.push(s.to_string());
                    }
                    i += 2 + len;
                } else {
                    i += 2;
                }
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    strings
}

pub fn build_preset_request_payload(preset_num: u8, seq: u8) -> Vec<u8> {
    let mut raw = vec![0x00, preset_num];
    raw.extend(vec![0x00; 30]);
    
    let mut chunk = Vec::new();
    for seq_slice in raw.chunks(7) {
        let mut bit8 = 0u8;
        let mut seq7 = Vec::new();
        for (i, &b) in seq_slice.iter().enumerate() {
            if (b & 0x80) == 0x80 {
                bit8 |= 1 << i;
            }
            seq7.push(b & 0x7f);
        }
        chunk.push(bit8);
        chunk.extend_from_slice(&seq7);
    }
    
    let total_size = 23 + chunk.len();
    let mut packet = Vec::with_capacity(total_size);
    packet.extend_from_slice(&[0x01, 0xfe, 0x00, 0x00, 0x53, 0xfe]);
    packet.push(total_size as u8);
    packet.push(0x00);
    packet.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    packet.extend_from_slice(&[0xf0, 0x01, seq, 0x00]);
    packet.push(0x02); // cmd
    packet.push(0x01); // sub_cmd
    packet.extend_from_slice(&chunk);
    packet.push(0xf7);
    packet
}


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

    #[allow(dead_code)]
    pub fn to_payload(self) -> Vec<u8> {
        vec![
            0x01, 0xfe, 0x00, 0x00, 0x53, 0xfe, 0x1a, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0xf0, 0x01, 0x3a, 0x15, 0x01, 0x38, 0x00, 0x00,
            self as u8, 0xf7,
        ]
    }
}

#[allow(dead_code)]
pub fn get_spark_action(action: &str) -> Option<Vec<u8>> {
    SparkPreset::from_str(action).map(|preset| preset.to_payload())
}

pub fn build_preset_change_payload(preset_num: u8, seq: u8) -> Vec<u8> {
    let chunk = vec![0x00, 0x00, preset_num];
    
    let total_size = 23 + chunk.len();
    let mut packet = Vec::with_capacity(total_size);
    packet.extend_from_slice(&[0x01, 0xfe, 0x00, 0x00, 0x53, 0xfe]);
    packet.push(total_size as u8);
    packet.push(0x00);
    packet.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    packet.extend_from_slice(&[0xf0, 0x01, seq, 0x15]);
    packet.push(0x01); // cmd
    packet.push(0x38); // sub_cmd
    packet.extend_from_slice(&chunk);
    packet.push(0xf7);
    packet
}

pub fn build_parameter_change_payload(pedal: &str, param: u8, val: f32, seq: u8) -> Vec<u8> {
    let mut raw = Vec::new();
    let pedal_len = pedal.len();
    raw.push(pedal_len as u8);
    raw.push((pedal_len + 0xa0) as u8);
    raw.extend_from_slice(pedal.as_bytes());
    
    raw.push(param);
    raw.push(0xca);
    raw.extend_from_slice(&val.to_be_bytes());
    
    let mut chunk = Vec::new();
    for seq_slice in raw.chunks(7) {
        let mut bit8 = 0u8;
        let mut seq7 = Vec::new();
        for (i, &b) in seq_slice.iter().enumerate() {
            if (b & 0x80) == 0x80 {
                bit8 |= 1 << i;
            }
            seq7.push(b & 0x7f);
        }
        chunk.push(bit8);
        chunk.extend_from_slice(&seq7);
    }
    
    let total_size = 23 + chunk.len();
    let mut packet = Vec::with_capacity(total_size);
    packet.extend_from_slice(&[0x01, 0xfe, 0x00, 0x00, 0x53, 0xfe]);
    packet.push(total_size as u8);
    packet.push(0x00);
    packet.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
    packet.extend_from_slice(&[0xf0, 0x01, seq, 0x15]);
    packet.push(0x01); // cmd
    packet.push(0x04); // sub_cmd
    packet.extend_from_slice(&chunk);
    packet.push(0xf7);
    
    packet
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
    rx: &mut mpsc::Receiver<crate::midi::MidiEvent>,
    saved_mac: String,
    button_mapping: HashMap<u8, String>,
    cc_mapping: HashMap<u8, String>,
    _preset_amps: HashMap<String, String>,
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
                        
                        let read_uuid = Uuid::parse_str("0000ffc2-0000-1000-8000-00805f9b34fb")?;
                        let read_char = chars.iter().find(|c| c.uuid == read_uuid);
                        
                        let (notify_tx, mut notify_rx) = mpsc::channel(128);
                        if let Some(rc) = read_char {
                            if let Ok(mut stream) = spark.notifications().await {
                                if spark.subscribe(rc).await.is_ok() {
                                    info!("Subscribed to Spark notifications.");
                                    tokio::spawn(async move {
                                        while let Some(n) = stream.next().await {
                                            if notify_tx.send(n).await.is_err() {
                                                break;
                                            }
                                        }
                                    });
                                } else {
                                    error!("Failed to subscribe to Spark notifications.");
                                }
                            }
                        } else {
                            warn!("Spark read/notification characteristic not found.");
                        }
                        
                        spark_ready.store(true, Ordering::Relaxed);
                        
                        let mut active_amp_model = "RolandJC120".to_string();
                        let mut pending_preset_slot = 0x7fu8;
                        let mut seq = 0u8;
                        let mut check_interval = tokio::time::interval(Duration::from_secs(2));
                        let mut read_buf = Vec::new();
                        let mut preset_concat_buf = Vec::new();
                        
                        // Request active preset details on startup to get the active amp model
                        let req_payload = build_preset_request_payload(0x7f, seq);
                        seq = seq.wrapping_add(1);
                        if let Err(e) = spark.write(c, &req_payload, WriteType::WithResponse).await {
                            error!("Error requesting active preset: {}", e);
                        } else {
                            info!("Requested active preset details on startup.");
                        }
                        
                        loop {
                            tokio::select! {
                                Some(notification) = notify_rx.recv() => {
                                    debug!("Received BLE notification, len={}, uuid={}", notification.value.len(), notification.uuid);
                                    debug!("BLE notification bytes: {:02x?}", notification.value);
                                    if notification.value.len() >= 16 {
                                        read_buf.extend_from_slice(&notification.value[16..]);
                                    }
                                    
                                    while let Some(start_pos) = read_buf.windows(2).position(|w| w == [0xf0, 0x01]) {
                                        if start_pos > 0 {
                                            read_buf.drain(0..start_pos);
                                        }
                                        
                                        if let Some(f7_pos) = read_buf.iter().position(|&b| b == 0xf7) {
                                            let chunk = read_buf.drain(0..=f7_pos).collect::<Vec<u8>>();
                                            debug!("Framed Spark chunk: {:02x?}", chunk);
                                            if chunk.len() > 6 {
                                                let cmd = chunk[4];
                                                let sub_cmd = chunk[5];
                                                let data7bit = &chunk[6..chunk.len() - 1]; // Exclude f7
                                                
                                                let data8bit = decode_7bit(data7bit);
                                                
                                                if cmd == 0x03 && sub_cmd == 0x01 {
                                                    if data8bit.len() > 3 {
                                                        let num_chunks = data8bit[0] as usize;
                                                        let this_chunk = data8bit[1] as usize;
                                                        let chunk_payload = &data8bit[3..];
                                                        
                                                        if this_chunk == 0 {
                                                            preset_concat_buf.clear();
                                                            pending_preset_slot = data8bit[2];
                                                        }
                                                        preset_concat_buf.extend_from_slice(chunk_payload);
                                                        
                                                        if this_chunk == num_chunks - 1 {
                                                            let mut resolved_amp = parse_active_amp_model(&preset_concat_buf);
                                                            if resolved_amp.is_none() {
                                                                let strings = extract_strings(&preset_concat_buf);
                                                                for s in &strings {
                                                                    if SPARK_AMPS.contains(&s.as_str()) {
                                                                        resolved_amp = Some(s.clone());
                                                                        break;
                                                                    }
                                                                }
                                                            }
                                                            if let Some(amp) = resolved_amp {
                                                                let preset_str = if pending_preset_slot == 0x7f {
                                                                    "Active Preset (Temporary)".to_string()
                                                                } else {
                                                                    format!("Preset {}", pending_preset_slot + 1)
                                                                };
                                                                info!("Dynamically resolved active amp model: '{}' ({}) for {}", amp, get_amp_user_name(&amp), preset_str);
                                                                active_amp_model = amp;
                                                            }
                                                            preset_concat_buf.clear();
                                                        }
                                                    } else if data8bit.len() == 3 {
                                                        let num_chunks = data8bit[0];
                                                        let this_chunk = data8bit[1];
                                                        let active_slot = data8bit[2];
                                                        if num_chunks == 0 && this_chunk == 0x7f {
                                                            if active_slot < 4 {
                                                                info!("Active preset slot on startup is {} (Preset {}), requesting details...", active_slot, active_slot + 1);
                                                                let req_payload = build_preset_request_payload(active_slot, seq);
                                                                seq = seq.wrapping_add(1);
                                                                if let Err(e) = spark.write(c, &req_payload, WriteType::WithResponse).await {
                                                                    error!("Error requesting active preset details: {}", e);
                                                                }
                                                            }
                                                        }
                                                    }
                                                } else if cmd == 0x03 && sub_cmd == 0x38 {
                                                    if data8bit.len() > 1 {
                                                        let preset_num = data8bit[1];
                                                        info!("Spark hardware preset changed to {} (Preset {})", preset_num, preset_num + 1);
                                                        let req_payload = build_preset_request_payload(preset_num, seq);
                                                        seq = seq.wrapping_add(1);
                                                        if let Err(e) = spark.write(c, &req_payload, WriteType::WithResponse).await {
                                                            error!("Error requesting preset info: {}", e);
                                                        }
                                                    }
                                                }
                                            }
                                        } else {
                                            break;
                                        }
                                    }
                                }
                                Some(event) = rx.recv() => {
                                    if !spark.is_connected().await.unwrap_or(false) {
                                        warn!("Spark disconnected while trying to send command.");
                                        break; 
                                    }
                                    
                                    match event {
                                        crate::midi::MidiEvent::ButtonPress(btn_id) => {
                                            info!("Received button press event: {}", btn_id);
                                            if let Some(action) = button_mapping.get(&btn_id) {
                                                info!("Pedal: {} -> Spark: {}", btn_id, action);
                                                if let Some(preset) = SparkPreset::from_str(action) {
                                                    let payload = build_preset_change_payload(preset as u8, seq);
                                                    seq = seq.wrapping_add(1);
                                                    if let Err(e) = spark.write(c, &payload, WriteType::WithResponse).await {
                                                        error!("Error sending preset change to Spark: {}", e);
                                                    } else {
                                                        info!("BLE preset change command sent successfully to Spark. Active preset: {}", action);
                                                        
                                                        // Request details for this preset to update the amp model
                                                        let req_payload = build_preset_request_payload(preset as u8, seq);
                                                        seq = seq.wrapping_add(1);
                                                        if let Err(e) = spark.write(c, &req_payload, WriteType::WithResponse).await {
                                                            error!("Error requesting preset info after change: {}", e);
                                                        }
                                                    }
                                                }
                                            } else {
                                                warn!("Button {} is not mapped in config. Check your spark_config.json mappings.", btn_id);
                                            }
                                        }
                                        crate::midi::MidiEvent::ControlChange { cc, value } => {
                                            info!("Received CC event: cc={}, value={}", cc, value);
                                            if let Some(target) = cc_mapping.get(&cc) {
                                                if target == "Volume" {
                                                    let val = (value as f32) / 127.0;
                                                    let payload = build_parameter_change_payload(&active_amp_model, 4, val, seq);
                                                    seq = seq.wrapping_add(1);
                                                    if let Err(e) = spark.write(c, &payload, WriteType::WithResponse).await {
                                                        error!("Error sending parameter change to Spark: {}", e);
                                                    } else {
                                                        info!("Sent volume CC change (value={}) to Spark for amp '{}'", value, active_amp_model);
                                                    }
                                                }
                                            }
                                        }
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

    #[test]
    fn test_build_preset_change_payload() {
        let payload = build_preset_change_payload(0, 0x15);
        assert_eq!(payload.len(), 26);
        // block size
        assert_eq!(payload[6], 26);
        // seq
        assert_eq!(payload[18], 0x15);
        // cmd / sub_cmd
        assert_eq!(payload[20], 0x01);
        assert_eq!(payload[21], 0x38);
        // chunk payload
        assert_eq!(payload[22], 0x00); // bit8 header
        assert_eq!(payload[23], 0x00);
        assert_eq!(payload[24], 0); // preset index
        assert_eq!(payload[25], 0xf7);
    }

    #[test]
    fn test_build_parameter_change_payload() {
        // Test with "Twin" (4 bytes), param = 0, val = 0.0 (4 bytes float)
        let payload = build_parameter_change_payload("Twin", 0, 0.0, 0x15);
        // Twin is 4 bytes. Raw data length: 1 (len) + 1 (len+a0) + 4 (bytes) + 1 (param) + 1 (float marker 0xca) + 4 (float bytes) = 12 bytes
        // 7-bit conversion of 12 bytes:
        // seq 1 (7 bytes): 0x04, 0xa4 (has 0x80 set!), 0x54, 0x77, 0x69, 0x6e, 0x00
        //   bit8 = 1 << 1 = 2. output: 0x02, 0x04, 0x24, 0x54, 0x77, 0x69, 0x6e, 0x00 (8 bytes)
        // seq 2 (5 bytes): 0xca (has 0x80 set!), 0x00, 0x00, 0x00, 0x00
        //   bit8 = 1 << 0 = 1. output: 0x01, 0x4a, 0x00, 0x00, 0x00, 0x00 (6 bytes)
        // Total chunk = 8 + 6 = 14 bytes
        // Total size = 23 + 14 = 37 bytes
        assert_eq!(payload.len(), 37);
        assert_eq!(payload[6], 37); // block size
        assert_eq!(payload[18], 0x15); // seq
        assert_eq!(payload[20], 0x01); // cmd
        assert_eq!(payload[21], 0x04); // sub_cmd
        assert_eq!(payload[36], 0xf7); // trailer
    }

    #[test]
    fn test_decode_7bit() {
        // raw data chunk 7-bit: 02 04 24 54 77 69 6e 00 15 4a 3e 30 20 45
        let data7bit = vec![0x02, 0x04, 0x24, 0x54, 0x77, 0x69, 0x6e, 0x00, 0x15, 0x4a, 0x3e, 0x30, 0x20, 0x45];
        let decoded = decode_7bit(&data7bit);
        // Expected original bytes:
        // [0x04, 0xa4, 0x54, 0x77, 0x69, 0x6e, 0x00, 0xca, 0x3e, 0xb0, 0x20, 0xc5]
        assert_eq!(decoded, vec![0x04, 0xa4, 0x54, 0x77, 0x69, 0x6e, 0x00, 0xca, 0x3e, 0xb0, 0x20, 0xc5]);
    }

    #[test]
    fn test_extract_strings() {
        let data = vec![0x04, 0xa4, 0x54, 0x77, 0x69, 0x6e, 0x00, 0xca, 0x3e, 0xb0, 0x20, 0xc5];
        let strings = extract_strings(&data);
        assert_eq!(strings, vec!["Twin".to_string()]);
    }

    #[test]
    fn test_build_preset_request_payload() {
        let payload = build_preset_request_payload(0x7f, 0x04);
        assert_eq!(payload.len(), 60);
        assert_eq!(payload[6], 60); // block size
        assert_eq!(payload[18], 0x04); // seq
        assert_eq!(payload[19], 0x00); // checksum
        assert_eq!(payload[20], 0x02); // cmd: Request Preset
        assert_eq!(payload[21], 0x01); // sub_cmd
        assert_eq!(payload[payload.len() - 1], 0xf7); // trailer
    }
}

