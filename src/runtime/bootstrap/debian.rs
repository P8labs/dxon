use anyhow::Result;
use std::path::Path;

use crate::error::DxonError;

pub fn bootstrap(rootfs: &Path) -> Result<()> {
    super::require_tool("debootstrap", "apt-get install debootstrap")?;

    let status = crate::user::privileged_command("debootstrap")
        .args(["stable", rootfs.to_str().unwrap()])
        .status()
        .map_err(|e| DxonError::BootstrapFailed {
            distro: "debian".into(),
            reason: e.to_string(),
        })?;

    if !status.success() {
        return Err(DxonError::BootstrapFailed {
            distro: "debian".into(),
            reason: format!("debootstrap exited with {status}"),
        }
        .into());
    }
    Ok(())
}
