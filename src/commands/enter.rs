use anyhow::Result;
use colored::Colorize;

use crate::container::store::ContainerStore;
use crate::runtime::nspawn::{ensure_container_user, enter, require_nspawn};

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

    let container_user = meta.config.container_user.as_deref();
    if let (Some(username), Some(uid), Some(gid)) = (
        container_user,
        meta.config.container_uid,
        meta.config.container_gid,
    ) {
        if let Err(e) = ensure_container_user(&rootfs, username, uid, gid) {
            eprintln!(
                "{} could not ensure container user '{}': {e}",
                "warn:".yellow(),
                username
            );
        }
    }

    let effective_cmd: Vec<String> = if cmd.is_empty() {
        let shell = meta.config.shell.as_deref().unwrap_or("bash");
        vec![format!("/bin/{shell}")]
    } else {
        cmd.to_vec()
    };

    let workspace = meta.config.workspace_dir.as_deref();

    enter(
        &rootfs,
        &effective_cmd,
        &meta.config.extra_args,
        container_user,
        workspace,
    )?;
    Ok(())
}
