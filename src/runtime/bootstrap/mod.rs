pub mod alpine;
pub mod arch;
pub mod debian;

use anyhow::Result;
use std::path::Path;
use std::process::Command;

use crate::error::DxonError;

pub fn require_tool(tool: &str, install_hint: &str) -> Result<()> {
    let found = Command::new("which")
        .arg(tool)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !found {
        return Err(DxonError::MissingTool {
            tool: tool.to_string(),
            hint: install_hint.to_string(),
        }
        .into());
    }
    Ok(())
}

pub enum Distro {
    Arch,
    Debian,
    Alpine,
}

impl Distro {
    pub fn parse(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "arch"   => Ok(Self::Arch),
            "debian" => Ok(Self::Debian),
            "alpine" => Ok(Self::Alpine),
            _        => Err(DxonError::UnsupportedDistro(s.to_string()).into()),
        }
    }
}

pub fn bootstrap(distro: &Distro, rootfs: &Path) -> Result<()> {
    match distro {
        Distro::Arch   => arch::bootstrap(rootfs),
        Distro::Debian => debian::bootstrap(rootfs),
        Distro::Alpine => alpine::bootstrap(rootfs),
    }
}
