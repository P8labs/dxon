use anyhow::Result;
use colored::Colorize;

use crate::config::{config_file_path, Config};

pub fn show(config: &Config) -> Result<()> {
    let file_path = config_file_path()?;
    let resolved_dir = config.containers_dir(None)?;
    let is_default = config
        .containers_dir
        .as_deref()
        .map(|s| s.is_empty())
        .unwrap_or(true);

    println!();
    println!("  {}", "dxon configuration".bold());
    println!("  {}", "─".repeat(50).dimmed());

    println!(
        "  {:<20} {}",
        "config file:".dimmed(),
        file_path.display().to_string().cyan()
    );

    println!(
        "  {:<20} {}{}",
        "containers_dir:".dimmed(),
        resolved_dir.display().to_string().cyan(),
        if is_default {
            "  (default)".dimmed().to_string()
        } else {
            String::new()
        }
    );

    let distro = config
        .default_distro
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("—");
    println!("  {:<20} {}", "default_distro:".dimmed(), distro);

    let tmpl = config
        .default_template
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("—");
    println!("  {:<20} {}", "default_template:".dimmed(), tmpl);

    println!();
    Ok(())
}

pub fn set(config: &mut Config, key: &str, value: &str) -> Result<()> {
    let file_path = config_file_path()?;
    config.set(key, value)?;
    config.save()?;

    if value.is_empty() {
        println!(
            "{} {} {}",
            "✓".green().bold(),
            key.bold(),
            "cleared".dimmed()
        );
    } else {
        println!(
            "{} {} {} {}",
            "✓".green().bold(),
            key.bold(),
            "→".dimmed(),
            value.cyan()
        );
    }
    println!("  saved to {}", file_path.display().to_string().dimmed());
    Ok(())
}
