use anyhow::Result;
use colored::Colorize;

use crate::template::{builtin, registry};

pub fn list(registry_url: &str) -> Result<()> {
    let cached = registry::list_cached_names();
    let cached_set: std::collections::HashSet<_> = cached.iter().cloned().collect();

    println!();
    println!("  {} {}", "template registry".bold(), registry_url.dimmed());
    println!("  {}", "─".repeat(60).dimmed());

    match registry::list_templates(registry_url) {
        Ok(entries) => {
            if entries.is_empty() {
                println!("  {}", "no templates found in registry".dimmed());
            } else {
                for entry in &entries {
                    let cached_marker = if cached_set.contains(&entry.name) {
                        " (cached)".dimmed().to_string()
                    } else {
                        String::new()
                    };
                    let distros = if entry.distros.is_empty() {
                        String::new()
                    } else {
                        format!("  [{}]", entry.distros.join(", "))
                            .dimmed()
                            .to_string()
                    };
                    println!(
                        "  {:<12} {}{}{}",
                        entry.name.cyan(),
                        entry.description.dimmed(),
                        distros,
                        cached_marker,
                    );
                }
            }
        }
        Err(e) => {
            eprintln!(
                "{} could not fetch registry index: {e}\n  \
                 Showing locally cached templates only.",
                "warning:".yellow().bold()
            );

            if cached.is_empty() {
                println!(
                    "  {}  (use --template <name> to download one)",
                    "no cached templates".dimmed()
                );
            } else {
                println!("  {}:", "cached".dimmed());
                for name in &cached {
                    println!("  {}", name.cyan());
                }
            }

            println!();
            println!("  {}:", "well-known templates".dimmed());
            for (name, desc) in builtin::list_descriptions() {
                let marker = if cached_set.contains(name) {
                    " (cached)".dimmed().to_string()
                } else {
                    String::new()
                };
                println!("  {:<12} {}{}", name.cyan(), desc.dimmed(), marker);
            }
        }
    }

    println!();
    println!(
        "  {} use {} to create a container from a template",
        "tip:".dimmed(),
        "dxon create --template <name>".bold()
    );
    println!();
    Ok(())
}

pub fn search(keyword: &str, registry_url: &str) -> Result<()> {
    println!(
        "\n  {} searching registry for '{}' …\n",
        "→".cyan(),
        keyword.bold()
    );

    let results = registry::search_templates(keyword, registry_url)?;

    if results.is_empty() {
        println!(
            "  {}",
            format!("no templates matching '{keyword}'").dimmed()
        );
    } else {
        let cached: std::collections::HashSet<_> =
            registry::list_cached_names().into_iter().collect();

        for entry in &results {
            let cached_marker = if cached.contains(&entry.name) {
                " (cached)".dimmed().to_string()
            } else {
                String::new()
            };
            let distros = if entry.distros.is_empty() {
                String::new()
            } else {
                format!("  [{}]", entry.distros.join(", "))
                    .dimmed()
                    .to_string()
            };
            println!(
                "  {:<12} {}{}{}",
                entry.name.cyan(),
                entry.description.dimmed(),
                distros,
                cached_marker,
            );
        }
    }

    println!();
    Ok(())
}

pub fn refresh(registry_url: &str) -> Result<()> {
    println!(
        "\n{} refreshing cached templates from {}\n",
        "→".cyan(),
        registry_url.dimmed()
    );
    registry::refresh(registry_url)
}
