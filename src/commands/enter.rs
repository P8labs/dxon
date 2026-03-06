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

    let effective_cmd: Vec<String> = if cmd.is_empty() {
        let shell = meta.config.shell.as_deref().unwrap_or("bash");
        vec![format!("/bin/{shell}")]
    } else {
        cmd.to_vec()
    };

    enter(&rootfs, &effective_cmd, &meta.config.extra_args)?;
    Ok(())
}
