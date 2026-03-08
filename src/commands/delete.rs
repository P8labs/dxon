use anyhow::Result;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Confirm};

use crate::container::store::ContainerStore;

pub fn run(store: &ContainerStore, name: &str, force: bool) -> Result<()> {
    if !store.exists(name) {
        eprintln!(
            "{} container '{}' does not exist",
            "error:".red().bold(),
            name
        );
        std::process::exit(1);
    }

    let confirmed = force || {
        Confirm::with_theme(&ColorfulTheme::default())
            .with_prompt(format!("Delete container '{name}'? This cannot be undone."))
            .default(false)
            .interact()?
    };

    if !confirmed {
        println!("Aborted.");
        return Ok(());
    }

    store.remove(name)?;
    println!(
        "{} Container '{}' deleted.",
        "✓".green().bold(),
        name.cyan()
    );
    Ok(())
}
