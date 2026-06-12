use midir::{MidiInput, Ignore};
use tokio::sync::mpsc;
use tokio::time::{sleep, Duration};
use log::{info, warn};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::error::Error;
use std::io::{self, Write};
use crate::config::AppConfig;
use dialoguer::{Select, theme::ColorfulTheme, console::style};
#[cfg(target_os = "linux")]
use tokio::io::{AsyncBufReadExt, BufReader};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MidiEvent {
    ButtonPress(u8),
    ControlChange { cc: u8, value: u8 },
}

pub fn scan_and_select_midi() -> Result<String, Box<dyn Error>> {
    loop {
        let midi_in = MidiInput::new("Spark MIDI Config Scanner")?;
        let ports = midi_in.ports();
        
        let mut inputs = Vec::new();
        for port in &ports {
            let name = midi_in.port_name(port)?;
            inputs.push(name);
        }
        
        if inputs.is_empty() {
            println!("{}", style("No MIDI input devices found.").yellow());
        }
        
        let mut options = inputs.clone();
        options.push("Rescan".to_string());
        options.push("Cancel and exit".to_string());
        
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Choose a MIDI Pedal")
            .items(&options)
            .default(0)
            .interact()?;
            
        if selection < inputs.len() {
            return Ok(inputs[selection].clone());
        } else if selection == inputs.len() {
            continue;
        } else {
            return Ok(String::new());
        }
    }
}

pub async fn map_midi_buttons(midi_name: &str, config: &mut AppConfig) -> Result<(), Box<dyn Error>> {
    if midi_name.is_empty() {
        println!("{}", style("You must first select a MIDI device.").red());
        return Ok(());
    }
    
    let mut midi_in = MidiInput::new("Spark MIDI Config Mapper")?;
    midi_in.ignore(Ignore::None);
    let ports = midi_in.ports();
    let target_port = ports.into_iter().find(|p| {
        let name = midi_in.port_name(p).unwrap_or_default().to_lowercase();
        name.contains(&midi_name.to_lowercase())
    });
    
    let port = match target_port {
        Some(p) => p,
        None => {
            println!("{}", style(format!("Could not find connected MIDI device containing '{}'", midi_name)).red());
            return Ok(());
        }
    };
    
    let name = midi_in.port_name(&port)?;
    println!("\n{}", style(format!("--- MAPPING MODE (Pedal: {}) ---", name)).cyan().bold());
    println!("{}", style("We will sequentially assign buttons for presets 1 to 4.").cyan());
    
    let (tx, mut rx) = mpsc::channel::<u8>(10);
    
    let _conn = midi_in.connect(&port, "SparkMidiMapperConn", move |_, message, _| {
        if message.len() >= 2 {
            let status = message[0];
            let data1 = message[1];
            if status >= 144 && status <= 207 && !(status >= 176 && status <= 191) {
                let _ = tx.try_send(data1);
            }
        }
    }, ())?;
    
    #[cfg(target_os = "linux")]
    let mut stdin_reader = BufReader::new(tokio::io::stdin());
    let presets = ["Preset 1", "Preset 2", "Preset 3", "Preset 4"];
    
    for preset in &presets {
        println!("\n{}", style(format!("Press the button on your pedal for {}.", preset)).yellow().bold());
        #[cfg(target_os = "linux")]
        println!("{}", style("   (Or press Enter on the keyboard to skip)").dim());
        print!("{}", style("Waiting for button press... ").dim());
        io::stdout().flush()?;
        
        // Vaciar cualquier entrada MIDI previa para evitar registrar pulsaciones pasadas accidentalmente
        while rx.try_recv().is_ok() {}
        
        #[cfg(target_os = "linux")]
        {
            let mut input_line = String::new();
            tokio::select! {
                Some(btn_id) = rx.recv() => {
                    let key = format!("btn{}", btn_id);
                    config.mappings.insert(key, preset.to_string());
                    println!("\n{}", style(format!("Button detected! Button ID [{}] assigned to {}", btn_id, preset)).green().bold());
                }
                _ = stdin_reader.read_line(&mut input_line) => {
                    println!("{}", style(format!("{} skipped.", preset)).yellow());
                }
            }
        }
        
        #[cfg(not(target_os = "linux"))]
        {
            if let Some(btn_id) = rx.recv().await {
                let key = format!("btn{}", btn_id);
                config.mappings.insert(key, preset.to_string());
                println!("\n{}", style(format!("Button detected! Button ID [{}] assigned to {}", btn_id, preset)).green().bold());
            }
        }
    }
    
    Ok(())
}

pub async fn map_midi_expression(midi_name: &str, config: &mut AppConfig) -> Result<(), Box<dyn Error>> {
    if midi_name.is_empty() {
        println!("{}", style("You must first select a MIDI device.").red());
        return Ok(());
    }
    
    let mut midi_in = MidiInput::new("Spark MIDI Config Exp Mapper")?;
    midi_in.ignore(Ignore::None);
    let ports = midi_in.ports();
    let target_port = ports.into_iter().find(|p| {
        let name = midi_in.port_name(p).unwrap_or_default().to_lowercase();
        name.contains(&midi_name.to_lowercase())
    });
    
    let port = match target_port {
        Some(p) => p,
        None => {
            println!("{}", style(format!("Could not find connected MIDI device containing '{}'", midi_name)).red());
            return Ok(());
        }
    };
    
    let name = midi_in.port_name(&port)?;
    println!("\n{}", style(format!("--- MAPPING EXPRESSION PEDAL (Pedal: {}) ---", name)).cyan().bold());
    println!("{}", style("Move your expression pedal now to assign it to Volume Control.").yellow().bold());
    #[cfg(target_os = "linux")]
    println!("{}", style("   (Or press Enter on the keyboard to skip)").dim());
    print!("{}", style("Waiting for expression pedal movement... ").dim());
    io::stdout().flush()?;
    
    let (tx, mut rx) = mpsc::channel::<u8>(10);
    
    let _conn = midi_in.connect(&port, "SparkMidiExpMapperConn", move |_, message, _| {
        if message.len() >= 2 {
            let status = message[0];
            let data1 = message[1];
            if status >= 176 && status <= 191 { // CC Messages
                let _ = tx.try_send(data1);
            }
        }
    }, ())?;
    
    #[cfg(target_os = "linux")]
    let mut stdin_reader = BufReader::new(tokio::io::stdin());
    
    // Vaciar cualquier entrada MIDI previa
    while rx.try_recv().is_ok() {}
    
    #[cfg(target_os = "linux")]
    {
        let mut input_line = String::new();
        tokio::select! {
            Some(cc_num) = rx.recv() => {
                let key = format!("cc{}", cc_num);
                // Clear other CC mappings to keep only one active volume expression pedal
                config.mappings.retain(|k, _| !k.starts_with("cc"));
                config.mappings.insert(key, "Volume".to_string());
                println!("\n{}", style(format!("Expression pedal detected! MIDI CC [{}] assigned to Volume Control.", cc_num)).green().bold());
            }
            _ = stdin_reader.read_line(&mut input_line) => {
                println!("{}", style("Expression pedal mapping skipped.").yellow());
            }
        }
    }
    
    #[cfg(not(target_os = "linux"))]
    {
        if let Some(cc_num) = rx.recv().await {
            let key = format!("cc{}", cc_num);
            config.mappings.retain(|k, _| !k.starts_with("cc"));
            config.mappings.insert(key, "Volume".to_string());
            println!("\n{}", style(format!("Expression pedal detected! MIDI CC [{}] assigned to Volume Control.", cc_num)).green().bold());
        }
    }
    
    Ok(())
}

pub async fn midi_connection_loop(
    tx: mpsc::Sender<MidiEvent>,
    target_name: String,
    midi_ready: Arc<AtomicBool>,
) -> Result<(), Box<dyn Error>> {
    loop {
        let mut midi_in_temp = MidiInput::new("Spark MIDI Input")?;
        midi_in_temp.ignore(Ignore::None);
        
        let ports = midi_in_temp.ports();
        let target_port = ports.into_iter().find(|p| {
            let name = midi_in_temp.port_name(p).unwrap_or_default().to_lowercase();
            !name.contains("midi through") && 
            (target_name.is_empty() || name.contains(&target_name) || name.contains("foot") || name.contains("sinco") || name.contains("m-vave"))
        });

        match target_port {
            Some(port) => {
                let name = midi_in_temp.port_name(&port).unwrap_or_default();
                info!("MIDI Pedal hooked: {}", name);
                
                let tx_clone = tx.clone();
                let _conn = midi_in_temp.connect(&port, "SparkMidiIn", move |_, message, _| {
                    info!("MIDI event received: {:?}", message);
                    if message.len() >= 2 {
                        let status = message[0];
                        let data1 = message[1];
                        if status >= 144 && status <= 207 && !(status >= 176 && status <= 191) {
                            info!("Forwarding button ID {} to channel (status={})", data1, status);
                            let _ = tx_clone.try_send(MidiEvent::ButtonPress(data1));
                        } else if status >= 176 && status <= 191 && message.len() >= 3 {
                            let data2 = message[2];
                            info!("Forwarding CC {} value {} to channel (status={})", data1, data2, status);
                            let _ = tx_clone.try_send(MidiEvent::ControlChange { cc: data1, value: data2 });
                        } else {
                            warn!("Ignored MIDI event (status={}) because it is not matching presets or volume", status);
                        }
                    }
                }, ())?;
                
                midi_ready.store(true, Ordering::Relaxed);
                
                loop {
                    sleep(Duration::from_secs(3)).await;
                    if let Ok(midi_check) = MidiInput::new("Spark MIDI Watchdog") {
                        let current_ports = midi_check.ports();
                        let still_connected = current_ports.iter().any(|p| {
                            midi_check.port_name(p).unwrap_or_default() == name
                        });
                        if !still_connected {
                            warn!("MIDI pedal '{}' disconnected.", name);
                            break;
                        }
                    }
                }
                
                midi_ready.store(false, Ordering::Relaxed);
            },
            None => {
                sleep(Duration::from_secs(2)).await;
            }
        }
    }
}
