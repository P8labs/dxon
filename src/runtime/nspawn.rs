use anyhow::Result;
use colored::Colorize;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use std::collections::HashMap;
use std::path::Path;
use std::process::Stdio;

use crate::error::DxonError;
use crate::runtime::bootstrap::require_tool;
use crate::runtime::packages::{fallback, pkg_install_cmd};

pub fn require_nspawn() -> Result<()> {
    require_tool(
        "systemd-nspawn",
        "install systemd: your-package-manager install systemd",
    )
}

pub fn run_command(rootfs: &Path, cmd: &str, env: &HashMap<String, String>) -> Result<()> {
    let mut builder = crate::user::privileged_command("systemd-nspawn");
    builder.arg("-D").arg(rootfs);

    for (k, v) in env {
        builder.arg("--setenv").arg(format!("{k}={v}"));
    }

    builder.args(["--", "/bin/sh", "-c", cmd]);

    let status = builder.status().map_err(|e| DxonError::BootstrapFailed {
        distro: "container".into(),
        reason: format!("systemd-nspawn failed to start: {e}"),
    })?;

    if !status.success() {
        return Err(DxonError::BootstrapFailed {
            distro: "container".into(),
            reason: format!("command failed (exit {status}): {cmd}"),
        }
        .into());
    }
    Ok(())
}

pub fn install_packages_with_fallback(
    rootfs: &Path,
    packages: &[String],
    distro: &str,
    env: &HashMap<String, String>,
) -> Result<()> {
    if packages.is_empty() {
        return Ok(());
    }

    let batch_cmd = pkg_install_cmd(distro, packages);
    if run_command(rootfs, &batch_cmd, env).is_ok() {
        return Ok(());
    }

    println!(
        "  {} batch install failed, retrying packages individually…",
        "!".yellow()
    );

    for pkg in packages {
        let single_cmd = pkg_install_cmd(distro, &[pkg.clone()]);
        if run_command(rootfs, &single_cmd, env).is_ok() {
            continue;
        }

        println!(
            "  {} package '{}' not found or failed",
            "!".yellow(),
            pkg.bold()
        );

        match fallback(pkg.as_str(), distro) {
            Some(alts) if !alts.is_empty() => {
                println!(
                    "  {} using known fallback: {}",
                    "→".cyan(),
                    alts.join(", ").bold()
                );
                let alt_cmd = pkg_install_cmd(distro, &alts);
                if let Err(e) = run_command(rootfs, &alt_cmd, env) {
                    println!("  {} fallback also failed: {}", "!".yellow(), e);
                    prompt_package_failure(rootfs, pkg, distro, env)?;
                }
            }
            _ => prompt_package_failure(rootfs, pkg, distro, env)?,
        }
    }

    Ok(())
}

fn prompt_package_failure(
    rootfs: &Path,
    pkg: &str,
    distro: &str,
    env: &HashMap<String, String>,
) -> Result<()> {
    let theme = ColorfulTheme::default();
    let choices = &[
        "skip this package",
        "enter a replacement package name",
        "abort container creation",
    ];

    let idx = Select::with_theme(&theme)
        .with_prompt(format!("How to handle missing package '{pkg}'?"))
        .items(choices)
        .default(0)
        .interact()?;

    match idx {
        0 => {
            println!("  {} skipping '{}'", "→".dimmed(), pkg);
            Ok(())
        }
        1 => {
            let replacement: String = Input::with_theme(&theme)
                .with_prompt("Replacement package name")
                .interact_text()?;
            let cmd = pkg_install_cmd(distro, &[replacement.clone()]);
            run_command(rootfs, &cmd, env).map_err(|_| {
                DxonError::BootstrapFailed {
                    distro: distro.into(),
                    reason: format!("replacement package '{replacement}' also failed"),
                }
                .into()
            })
        }
        _ => Err(DxonError::BootstrapFailed {
            distro: distro.into(),
            reason: format!("aborted after package '{pkg}' failed"),
        }
        .into()),
    }
}

pub fn enter(rootfs: &Path, cmd: &[String]) -> Result<()> {
    require_nspawn()?;

    let mut builder = crate::user::privileged_command("systemd-nspawn");
    builder.arg("-D").arg(rootfs);

    if !cmd.is_empty() {
        builder.arg("--").args(cmd);
    }

    let status = builder
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .map_err(|e| DxonError::BootstrapFailed {
            distro: "container".into(),
            reason: format!("systemd-nspawn failed: {e}"),
        })?;

    if !status.success() {
        eprintln!(
            "container exited with status {}",
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}
