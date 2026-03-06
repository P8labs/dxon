use anyhow::Result;
use colored::Colorize;

use crate::template::{builtin, registry};

pub fn list() -> Result<()> {
    let cached = registry::list_cached();
    let builtin_list = builtin::list_descriptions();

    println!();
    println!("  {}", "template registry".bold());
    println!("  {}", "─".repeat(50).dimmed());

    if cached.is_empty() {
        println!(
            "  {}  (run {} to download)",
            "no cached templates".dimmed(),
            "dxon registry update".bold()
        );
    } else {
        println!("  {}:", "cached".dimmed());
        for (name, desc) in &cached {
            let d = if desc.is_empty() {
                "—"
            } else {
                desc.as_str()
            };
            println!("  {:<12} {}", name.cyan(), d.dimmed());
        }
    }

    println!();
    println!("  {}:", "built-in".dimmed());
    for (name, desc) in &builtin_list {
        println!("  {:<12} {}", name.cyan(), desc.dimmed());
    }

    println!();
    println!(
        "  registry source: {}",
        "github.com/P8labs/dxon-registry".dimmed()
    );
    println!();
    Ok(())
}

pub fn update() -> Result<()> {
    println!(
        "{} syncing registry from {}",
        "→".cyan(),
        "github.com/P8labs/dxon-registry".dimmed()
    );
    registry::update()
}
