use std::env;
use std::path::PathBuf;
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
