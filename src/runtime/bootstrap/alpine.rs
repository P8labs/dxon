use anyhow::Result;
use std::path::Path;
use std::process::Command;

use crate::error::DxonError;

pub fn bootstrap(rootfs: &Path) -> Result<()> {
    super::require_tool("curl", "your-package-manager install curl")?;
    super::require_tool("tar",  "your-package-manager install tar")?;

    let apk_arch = match std::env::consts::ARCH {
        "aarch64" => "aarch64",
        _         => "x86_64",
    };

    let url = format!(
        "https://dl-cdn.alpinelinux.org/alpine/latest-stable/releases/{apk_arch}/alpine-minirootfs-latest-{apk_arch}.tar.gz"
    );

    let tarball = rootfs.parent().unwrap().join("alpine-rootfs.tar.gz");

    let dl = Command::new("curl")
        .args(["-fsSL", "-o", tarball.to_str().unwrap(), &url])
        .status()
        .map_err(|e| DxonError::BootstrapFailed {
            distro: "alpine".into(),
            reason: format!("curl download failed: {e}"),
        })?;

    if !dl.success() {
        return Err(DxonError::BootstrapFailed {
            distro: "alpine".into(),
            reason: "failed to download Alpine mini-rootfs tarball".into(),
        }
        .into());
    }

    std::fs::create_dir_all(rootfs)?;

    let extract = Command::new("tar")
        .args(["-xzf", tarball.to_str().unwrap(), "-C", rootfs.to_str().unwrap()])
        .status()
        .map_err(|e| DxonError::BootstrapFailed {
            distro: "alpine".into(),
            reason: format!("tar extraction failed: {e}"),
        })?;

    if !extract.success() {
        return Err(DxonError::BootstrapFailed {
            distro: "alpine".into(),
            reason: "failed to extract Alpine mini-rootfs".into(),
        }
        .into());
    }

    let _ = std::fs::remove_file(&tarball);

    let init = crate::user::privileged_command("systemd-nspawn")
        .args(["-D", rootfs.to_str().unwrap(), "apk", "update"])
        .status();

    if let Ok(s) = init {
        if !s.success() {
            eprintln!("  warning: apk update returned non-zero; container may still be usable");
        }
    }

    Ok(())
}
