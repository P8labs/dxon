use anyhow::Result;
use colored::Colorize;

use crate::container::store::ContainerStore;

pub fn run(store: &ContainerStore, name: &str) -> Result<()> {
    let meta = store.load_meta(name)?;
    let container_dir = store.container_dir(name);

    println!();
    println!("  {}", meta.name.cyan().bold());
    println!("  {}", "─".repeat(50).dimmed());

    println!("  {:<18} {}", "distro:".dimmed(), meta.distro);
    println!(
        "  {:<18} {}",
        "template:".dimmed(),
        meta.template.as_deref().unwrap_or("—")
    );
    println!(
        "  {:<18} {}",
        "created:".dimmed(),
        meta.created_at.format("%Y-%m-%d %H:%M UTC")
    );
    println!(
        "  {:<18} {}",
        "container dir:".dimmed(),
        container_dir.display().to_string().cyan()
    );
    println!("  {:<18} {}", "rootfs:".dimmed(), meta.rootfs_path.cyan());

    if let Some(ref repo) = meta.repo {
        println!("  {:<18} {}", "repo:".dimmed(), repo);
    }

    if !meta.packages.is_empty() {
        println!(
            "  {:<18} {}",
            "packages:".dimmed(),
            meta.packages.join(", ")
        );
    }

    if !meta.config.env.is_empty() {
        println!("  env:");
        let mut pairs: Vec<_> = meta.config.env.iter().collect();
        pairs.sort_by_key(|(k, _)| k.as_str());
        for (k, v) in pairs {
            println!("    {} = {}", k.dimmed(), v);
        }
    }

    println!();
    Ok(())
}
