use std::error::Error;
use crate::config::AppConfig;
use crate::spark::scan_and_select_spark;
use crate::midi::{scan_and_select_midi, map_midi_buttons};
use dialoguer::{Select, theme::ColorfulTheme, console::style};

/// Runs the interactive CLI wizard for initial device setup and button mapping.
pub async fn run_configurator() -> Result<(), Box<dyn Error>> {
    let mut config = AppConfig::load();
    
    loop {
        println!("\n{}", style("==========================================").cyan());
        println!("{}", style("  SPARK MIDI BRIDGE  ---  Configurator    ").cyan().bold());
        println!("{}", style("==========================================").cyan());
        
        // --- STEP 1: Spark Amp ---
        println!("\n{}", style("[Step 1/3] Pair with Spark amplifier").yellow().bold());
        let (spark_mac, spark_name) = scan_and_select_spark().await?;
        if spark_mac.is_empty() {
            println!("{}", style("Configuration cancelled.").red());
            return Ok(());
        }
        config.spark_mac = spark_mac.clone();
        config.spark_name = spark_name.clone();
        println!("\n{}", style(format!("Selected Spark: {} - {}", spark_name, spark_mac)).green().bold());
        
        // --- STEP 2: MIDI Pedal ---
        println!("\n{}", style("[Step 2/3] Pair with MIDI pedal (USB/Bluetooth)").yellow().bold());
        let midi_name = scan_and_select_midi()?;
        if midi_name.is_empty() {
            println!("{}", style("Configuration cancelled.").red());
            return Ok(());
        }
        config.midi_name = midi_name.clone();
        println!("\n{}", style(format!("Selected MIDI Pedal: {}", midi_name)).green().bold());
        
        // --- STEP 3: Map Buttons ---
        println!("\n{}", style("[Step 3/3] Map pedal buttons").yellow().bold());
        // Clear previous button mappings for a fresh config run
        config.mappings.retain(|k, _| !k.starts_with("btn"));
        
        map_midi_buttons(&midi_name, &mut config).await?;
        
        // --- SUMMARY & EXIT/RESTART ---
        println!("\n{}", style("==========================================").cyan());
        println!("{}", style("           CONFIGURATION SUMMARY          ").cyan().bold());
        println!("{}", style("==========================================").cyan());
        println!("{}: {} - {}", style("Spark Amp").bold(), config.spark_name, config.spark_mac);
        println!("{}: {}", style("MIDI Pedal").bold(), config.midi_name);
        println!("{}", style("Button mappings:").bold());
        
        let button_mappings = config.get_button_mappings();
        // We want to show mappings for Preset 1, 2, 3, 4
        for preset_num in 1..=4 {
            let preset_name = format!("Preset {}", preset_num);
            let mapped_btn = button_mappings.iter()
                .find(|(_, v)| *v == &preset_name)
                .map(|(k, _)| k.to_string());
                
            match mapped_btn {
                Some(btn_id) => println!("  - Preset {}: {} {}", preset_num, style("Button").green(), style(btn_id).green().bold()),
                None => println!("  - Preset {}: {}", preset_num, style("Not assigned").dim()),
            }
        }
        println!("{}", style("==========================================").cyan());
        
        let options = vec!["Save and exit", "Start over"];
        let choice = tokio::task::spawn_blocking(move || {
            Select::with_theme(&ColorfulTheme::default())
                .with_prompt("Choose an action")
                .items(&options)
                .default(0)
                .interact()
        }).await??;
        
        if choice == 0 {
            if let Err(e) = config.save() {
                println!("{}", style(format!("Error saving configuration: {}", e)).red());
            } else {
                println!("{}", style("Configuration saved successfully to spark_config.json.").green().bold());
            }
            println!("{}", style("Exiting configuration. Start the bridge normally to use it.").green());
            break;
        } else {
            println!("\n{}", style("Restarting configuration wizard...").yellow());
            continue;
        }
    }
    
    Ok(())
}
