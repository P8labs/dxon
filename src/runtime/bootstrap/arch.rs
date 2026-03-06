use anyhow::Result;
use std::path::Path;

use crate::error::DxonError;

pub fn bootstrap(rootfs: &Path) -> Result<()> {
    super::require_tool("pacstrap", "pacman -S arch-install-scripts")?;

    let status = crate::user::privileged_command("pacstrap")
        .args(["-c", rootfs.to_str().unwrap(), "base", "base-devel", "git", "curl"])
        .status()
        .map_err(|e| DxonError::BootstrapFailed {
            distro: "arch".into(),
            reason: e.to_string(),
        })?;

    if !status.success() {
        return Err(DxonError::BootstrapFailed {
            distro: "arch".into(),
            reason: format!("pacstrap exited with {status}"),
        }
        .into());
    }
    Ok(())
}
