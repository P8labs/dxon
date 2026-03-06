use anyhow::Result;
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;
use users::{get_current_uid, get_user_by_name, os::unix::UserExt};

pub fn is_root() -> bool {
    get_current_uid() == 0
}

pub fn resolve_home() -> PathBuf {
    static WARNED: OnceLock<()> = OnceLock::new();

    if is_root() {
        if let Ok(sudo_user) = env::var("SUDO_USER") {
            if !sudo_user.is_empty() {
                if let Some(user) = get_user_by_name(sudo_user.as_str()) {
                    return user.home_dir().to_path_buf();
                }
            }
        }
        WARNED.get_or_init(|| {
            eprintln!(
                "dxon warning: running as root without sudo.\nConfiguration will be stored in /root.\nThis is usually not intended."
            );
        });
    }

    dirs::home_dir().expect("cannot determine home directory")
}

pub fn privileged_command(prog: &str) -> Command {
    if is_root() {
        Command::new(prog)
    } else {
        let mut cmd = Command::new("sudo");
        cmd.arg(prog);
        cmd
    }
}

pub fn command_available(name: &str) -> bool {
    Command::new("which")
        .arg(name)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn privileged_mkdir(dir: &Path) -> Result<()> {
    match std::fs::create_dir_all(dir) {
        Ok(()) => return Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {}
        Err(e) => return Err(e.into()),
    }
    let status = Command::new("sudo")
        .args(["mkdir", "-p", &dir.to_string_lossy()])
        .status()?;
    if !status.success() {
        anyhow::bail!("sudo mkdir -p {} failed", dir.display());
    }
    Ok(())
}

pub fn privileged_read(path: &Path) -> Result<String> {
    match std::fs::read_to_string(path) {
        Ok(s) => return Ok(s),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {}
        Err(e) => return Err(e.into()),
    }
    let out = Command::new("sudo")
        .args(["cat", &path.to_string_lossy().as_ref()])
        .output()?;
    if !out.status.success() {
        anyhow::bail!("sudo cat {} failed", path.display());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

pub fn privileged_write(path: &Path, content: &[u8]) -> Result<()> {
    match std::fs::write(path, content) {
        Ok(()) => return Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => {}
        Err(e) => return Err(e.into()),
    }
    let mut child = Command::new("sudo")
        .args(["tee", &path.to_string_lossy()])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::null())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin.write_all(content)?;
    }
    let status = child.wait()?;
    if !status.success() {
        anyhow::bail!("sudo tee {} failed", path.display());
    }
    Ok(())
}
