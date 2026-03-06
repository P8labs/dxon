use anyhow::Result;
use colored::Colorize;

use crate::container::store::ContainerStore;

pub fn run(store: &ContainerStore) -> Result<()> {
    let containers = store.list()?;

    if containers.is_empty() {
        println!("No containers found. Create one with {}", "dxon create".bold());
        println!("  storage: {}", store.base_dir.display().to_string().dimmed());
        return Ok(());
    }

    println!(
        "{:<20} {:<10} {:<14} {:<18} {}",
        "NAME".bold(),
        "DISTRO".bold(),
        "TEMPLATE".bold(),
        "CREATED".bold(),
        "PATH".bold()
    );
    println!("{}", "─".repeat(90).dimmed());

    let home = dirs::home_dir();

    for c in &containers {
        let container_dir = store.container_dir(&c.name);
        let display_path = match &home {
            Some(h) if container_dir.starts_with(h) => {
                format!("~/{}", container_dir.strip_prefix(h).unwrap().display())
            }
            _ => container_dir.display().to_string(),
        };

        println!(
            "{:<20} {:<10} {:<14} {:<18} {}",
            c.name.cyan(),
            c.distro,
            c.template.as_deref().unwrap_or("—"),
            c.created_at.format("%Y-%m-%d %H:%M").to_string(),
            display_path.dimmed()
        );
    }
    Ok(())
}
