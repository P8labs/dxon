use anyhow::Result;
use colored::Colorize;
use std::path::{Path, PathBuf};
use std::process::Stdio;

use crate::container::store::ContainerStore;
use crate::runtime::ipc;
use crate::runtime::nspawn::{ensure_container_user, enter, require_nspawn};

pub fn run(store: &ContainerStore, target: &str, cmd: &[String]) -> Result<()> {
    require_nspawn()?;

    let (name, subpath) = parse_enter_target(target)?;
    let meta = store.load_meta(&name)?;
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

    let machine_name = format!("dxon-{}", sanitize_machine_name(&name));

    let container_user = meta.config.container_user.as_deref();

    let effective_cmd: Vec<String> = if cmd.is_empty() {
        let shell = meta.config.shell.as_deref().unwrap_or("bash");
        vec![format!("/bin/{shell}")]
    } else {
        cmd.to_vec()
    };

    let enter_dir = enter_dir_from_meta(&meta, &subpath);

    if is_machine_running(&machine_name) {
        return attach_to_running_machine(
            &machine_name,
            container_user,
            enter_dir.as_deref(),
            &effective_cmd,
        );
    }

    cleanup_stale_nspawn_machine(&machine_name);

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

    let _ipc_server = ipc::HostSocketServer::start(store.base_dir.clone())?;

    let mut effective_args = meta.config.extra_args.clone();

    let socket_path = ipc::host_socket_path_from_containers_base(&store.base_dir);
    push_if_missing(
        &mut effective_args,
        format!(
            "--bind={}:{}",
            socket_path.display(),
            ipc::CONTAINER_SOCKET_PATH
        ),
    );

    if let Ok(host_exe) = std::env::current_exe() {
        push_if_missing(
            &mut effective_args,
            format!("--bind-ro={}:/usr/bin/dxon", host_exe.display()),
        );
    }

    push_if_missing(
        &mut effective_args,
        format!("--setenv=DXON_CONTAINER={}", name),
    );

    if !has_arg_prefix(&effective_args, "--machine=") {
        effective_args.push(format!("--machine={machine_name}"));
    }

    enter(
        &rootfs,
        &effective_cmd,
        &effective_args,
        container_user,
        enter_dir.as_deref(),
    )?;
    Ok(())
}

fn enter_dir_from_meta(
    meta: &crate::container::meta::ContainerMeta,
    subpath: &Option<PathBuf>,
) -> Option<String> {
    let workspace_base = meta.config.workspace_dir.as_deref().unwrap_or("/workspace");
    match subpath {
        Some(rel) if !rel.as_os_str().is_empty() => {
            Some(format!("{}/{}", workspace_base, rel.display()))
        }
        _ => Some(workspace_base.to_string()),
    }
}

fn attach_to_running_machine(
    machine_name: &str,
    user: Option<&str>,
    chdir: Option<&str>,
    cmd: &[String],
) -> Result<()> {
    println!(
        "{} attaching to running machine {}…",
        "→".cyan(),
        machine_name.bold()
    );

    let mut shell_line = String::new();
    if let Some(dir) = chdir {
        shell_line.push_str("cd ");
        shell_line.push_str(&shell_quote(dir));
        shell_line.push_str(" && ");
    }

    if cmd.is_empty() {
        shell_line.push_str("exec /bin/sh");
    } else {
        shell_line.push_str("exec");
        for part in cmd {
            shell_line.push(' ');
            shell_line.push_str(&shell_quote(part));
        }
    }

    let leader = machine_leader_pid(machine_name).ok_or_else(|| {
        anyhow::anyhow!("could not determine leader PID for machine '{machine_name}'")
    })?;

    let wrapper = if let Some(u) = user {
        format!(
            "if command -v su >/dev/null 2>&1; then exec su -s /bin/sh - {} -c {}; else exec /bin/sh -lc {}; fi",
            shell_quote(u),
            shell_quote(&shell_line),
            shell_quote(&shell_line)
        )
    } else {
        format!("exec /bin/sh -lc {}", shell_quote(&shell_line))
    };

    let status = crate::user::privileged_command("nsenter")
        .arg("--target")
        .arg(leader.to_string())
        .arg("--mount")
        .arg("--uts")
        .arg("--ipc")
        .arg("--net")
        .arg("--pid")
        .arg("--")
        .arg("/bin/sh")
        .arg("-lc")
        .arg(wrapper)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()?;

    if !status.success() {
        eprintln!(
            "container exited with status {}",
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}

fn is_machine_running(machine_name: &str) -> bool {
    if !crate::user::command_available("machinectl") {
        return false;
    }

    let output = crate::user::privileged_command("machinectl")
        .arg("show")
        .arg(machine_name)
        .arg("--property=State")
        .arg("--value")
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let state = String::from_utf8_lossy(&out.stdout).trim().to_string();
            matches!(state.as_str(), "running" | "degraded")
        }
        _ => false,
    }
}

fn machine_leader_pid(machine_name: &str) -> Option<u32> {
    if !crate::user::command_available("machinectl") {
        return None;
    }

    let output = crate::user::privileged_command("machinectl")
        .arg("show")
        .arg(machine_name)
        .arg("--property=Leader")
        .arg("--value")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let raw = String::from_utf8_lossy(&output.stdout).trim().to_string();
    raw.parse::<u32>().ok().filter(|pid| *pid > 0)
}

fn shell_quote(input: &str) -> String {
    if input.is_empty() {
        return "''".to_string();
    }
    let escaped = input.replace('\'', "'\\''");
    format!("'{escaped}'")
}

fn push_if_missing(args: &mut Vec<String>, value: String) {
    if !args.iter().any(|existing| existing == &value) {
        args.push(value);
    }
}

fn has_arg_prefix(args: &[String], prefix: &str) -> bool {
    args.iter().any(|arg| arg.starts_with(prefix))
}

fn sanitize_machine_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' {
            out.push(ch.to_ascii_lowercase());
        } else {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

fn parse_enter_target(raw: &str) -> Result<(String, Option<PathBuf>)> {
    if let Some((name, subpath)) = raw.split_once('/') {
        if name.is_empty() {
            anyhow::bail!("invalid enter target '{}': missing container name", raw);
        }
        let rel = sanitize_workspace_subpath(subpath)?;
        return Ok((name.to_string(), Some(rel)));
    }

    Ok((raw.to_string(), None))
}

fn sanitize_workspace_subpath(raw: &str) -> Result<PathBuf> {
    let path = Path::new(raw);
    if path.is_absolute() {
        anyhow::bail!("workspace path must be relative, got '{raw}'");
    }

    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            std::path::Component::CurDir => {}
            std::path::Component::Normal(seg) => out.push(seg),
            std::path::Component::ParentDir => {
                anyhow::bail!("workspace path must not contain '..': '{raw}'")
            }
            _ => anyhow::bail!("invalid workspace path: '{raw}'"),
        }
    }
    Ok(out)
}

fn cleanup_stale_nspawn_machine(machine_name: &str) {
    let socket_path = format!("/run/systemd/nspawn/unix-export/{machine_name}");

    if !std::path::Path::new(&socket_path).exists() {
        return;
    }

    if crate::user::command_available("machinectl") {
        let _ = crate::user::privileged_command("machinectl")
            .arg("terminate")
            .arg(machine_name)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    if std::path::Path::new(&socket_path).exists() {
        let _ = crate::user::privileged_command("umount")
            .arg("--lazy")
            .arg(&socket_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }

    if std::path::Path::new(&socket_path).exists() {
        let _ = crate::user::privileged_command("rm")
            .arg("-f")
            .arg(&socket_path)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
    }
}
