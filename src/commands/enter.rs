use anyhow::Result;
use colored::Colorize;

use crate::container::store::ContainerStore;
use crate::runtime::nspawn::{enter, require_nspawn};

pub fn run(store: &ContainerStore, name: &str, cmd: &[String]) -> Result<()> {
    require_nspawn()?;

    let meta = store.load_meta(name)?;
    let rootfs = std::path::PathBuf::from(&meta.rootfs_path);

    if !rootfs.exists() {
        eprintln!(
            "{} rootfs directory not found: {}",
            "error:".red().bold(),
            rootfs.display()
        );
        std::process::exit(1);
    }

    println!("{} entering {}…", "→".cyan(), name.bold());
    enter(&rootfs, cmd)?;
    Ok(())
}
