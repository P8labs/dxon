use anyhow::{bail, Result};
use colored::Colorize;
use serde::{Deserialize, Serialize};
use std::env;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::Config;
use crate::container::store::ContainerStore;
use crate::runtime::ipc;
use crate::user;

const EDITOR_CANDIDATES: &[&str] = &["code", "cursor", "zed"];

const VSCODE_COMPAT: &[&str] = &["code", "cursor"];

pub fn run(
    store: &ContainerStore,
    cfg: &mut Config,
    target: &str,
    editor_override: Option<&str>,
) -> Result<()> {
    if is_container_client_mode() {
        return proxy_open_to_host(target, editor_override);
    }

    let (container, folder) = parse_open_target(target)?;
    run_host_open_for_workspace_subpath(store, cfg, &container, &folder, editor_override)
}

pub fn run_from_rpc(
    store: &ContainerStore,
    cfg: &mut Config,
    container_name: &str,
    container_path: &str,
    editor_override: Option<&str>,
) -> Result<()> {
    let folder = workspace_subpath_from_container_path(container_path)?;
    run_host_open_for_workspace_subpath(store, cfg, container_name, &folder, editor_override)
}

fn run_host_open_for_workspace_subpath(
    store: &ContainerStore,
    cfg: &mut Config,
    container_name: &str,
    folder: &Path,
    editor_override: Option<&str>,
) -> Result<()> {
    let meta = store.load_meta(container_name)?;
    let rootfs = PathBuf::from(&meta.rootfs_path);

    if !rootfs.exists() {
        bail!(
            "rootfs directory not found: {}\n  has the container been created?",
            rootfs.display()
        );
    }

    let workspace = rootfs.join("workspace");
    if !workspace.exists() {
        bail!(
            "workspace directory not found: {}\n  container '{}' has no /workspace",
            workspace.display(),
            container_name
        );
    }

    let open_path = workspace.join(folder);
    if !open_path.exists() {
        bail!(
            "target directory not found in container workspace: {}\n  expected host path: {}",
            folder.display(),
            open_path.display()
        );
    }

    if !open_path.is_dir() {
        bail!(
            "target is not a directory: {}\n  use a folder path under /workspace",
            open_path.display()
        );
    }

    let editor = if let Some(e) = editor_override {
        e.to_string()
    } else if let Some(ref e) = cfg.default_editor.clone().filter(|s| !s.is_empty()) {
        e.clone()
    } else {
        let detected = detect_editor()?;
        cfg.default_editor = Some(detected.clone());
        if let Err(e) = cfg.save() {
            eprintln!("{} could not save editor preference: {e}", "warn:".yellow());
        }
        detected
    };

    let editor_bin = editor.split_whitespace().next().unwrap_or("");

    if VSCODE_COMPAT.contains(&editor_bin) {
        let enter_target = if folder.as_os_str().is_empty() {
            format!("{}/.", container_name)
        } else {
            format!("{}/{}", container_name, folder.display())
        };
        ensure_vscode_terminal_profile(&open_path, &enter_target)?;
    }

    println!(
        "{} opening {} in {}…",
        "→".cyan(),
        open_path.display().to_string().bold(),
        editor.bold()
    );

    if editor_bin == "zed" {
        println!(
            "  {} Zed: terminal integration is not yet configured automatically",
            "note:".dimmed()
        );
        println!(
            "  {}   use `dxon enter {}` to enter the container from a host terminal",
            " ".dimmed(),
            container_name
        );
    }

    let mut cmd = host_user_command(&editor);
    cmd.arg(&open_path);

    let status = cmd
        .status()
        .map_err(|e| anyhow::anyhow!("failed to launch editor '{}': {}", editor, e))?;

    if !status.success() {
        eprintln!(
            "{} editor '{}' exited with status {}",
            "warn:".yellow(),
            editor,
            status.code().unwrap_or(-1)
        );
    }

    Ok(())
}

fn parse_open_target(target: &str) -> Result<(String, PathBuf)> {
    let (container, raw_folder) = target.split_once('/').ok_or_else(|| {
        anyhow::anyhow!("invalid target '{}': expected <container>/<folder>", target)
    })?;

    if container.is_empty() {
        bail!("invalid target '{}': missing container name", target);
    }

    let folder = sanitize_workspace_subpath(raw_folder)?;
    Ok((container.to_string(), folder))
}

fn sanitize_workspace_subpath(raw: &str) -> Result<PathBuf> {
    let mut out = PathBuf::new();
    let input = Path::new(raw);

    if input.is_absolute() {
        bail!("workspace folder must be relative, got absolute path '{raw}'");
    }

    for comp in input.components() {
        match comp {
            std::path::Component::CurDir => {}
            std::path::Component::Normal(part) => out.push(part),
            std::path::Component::ParentDir => {
                bail!("workspace folder must not contain '..': '{raw}'")
            }
            _ => bail!("invalid workspace folder path: '{raw}'"),
        }
    }

    Ok(out)
}

fn workspace_subpath_from_container_path(container_path: &str) -> Result<PathBuf> {
    let path = Path::new(container_path);
    if !path.is_absolute() {
        bail!(
            "invalid container path '{}': expected an absolute path",
            container_path
        );
    }

    let rel = path
        .strip_prefix("/workspace")
        .map_err(|_| anyhow::anyhow!("path '{}' is outside /workspace", container_path))?;

    sanitize_workspace_subpath(rel.to_string_lossy().as_ref())
}

fn is_container_client_mode() -> bool {
    Path::new(ipc::CONTAINER_SOCKET_PATH).exists()
        && env::var("DXON_CONTAINER")
            .ok()
            .is_some_and(|v| !v.trim().is_empty())
}

fn proxy_open_to_host(path_arg: &str, editor_override: Option<&str>) -> Result<()> {
    let container = env::var("DXON_CONTAINER")
        .map_err(|_| anyhow::anyhow!("DXON_CONTAINER not set in container session"))?;

    let current_dir = env::current_dir()?;
    let requested = Path::new(path_arg);
    let abs = if requested.is_absolute() {
        requested.to_path_buf()
    } else {
        current_dir.join(requested)
    };

    let req = IpcRequest {
        method: "open".to_string(),
        container,
        path: abs.to_string_lossy().into_owned(),
        editor: editor_override.map(ToOwned::to_owned),
    };

    let response: IpcResponse = ipc::send_request(Path::new(ipc::CONTAINER_SOCKET_PATH), &req)?;
    if !response.ok {
        bail!(response.message);
    }

    println!("{}", response.message);
    Ok(())
}

fn detect_editor() -> Result<String> {
    for &candidate in EDITOR_CANDIDATES {
        if user::command_available(candidate) {
            return Ok(candidate.to_string());
        }
    }

    bail!(
        "no supported editor found on PATH\n  checked: {}\n  install one or use: dxon open <container> --editor <binary>",
        EDITOR_CANDIDATES.join(", ")
    )
}

fn ensure_vscode_terminal_profile(workspace: &Path, enter_target: &str) -> Result<()> {
    let vscode_dir = workspace.join(".vscode");

    user::privileged_mkdir(&vscode_dir)?;

    let settings_path = vscode_dir.join("settings.json");

    let mut settings: serde_json::Value = if settings_path.exists() {
        let raw = user::privileged_read(&settings_path).unwrap_or_default();
        let stripped = strip_json_comments(&raw);
        serde_json::from_str(&stripped).unwrap_or(serde_json::Value::Object(Default::default()))
    } else {
        serde_json::Value::Object(Default::default())
    };

    let obj = settings
        .as_object_mut()
        .expect("settings root must be a JSON object");

    let profile_name = "dXon";
    let profile_entry = serde_json::json!({
        "path": "dxon",
        "args": ["enter", enter_target],
        "icon": "terminal-linux"
    });

    let profiles_key = "terminal.integrated.profiles.linux";
    let profiles = obj
        .entry(profiles_key)
        .or_insert_with(|| serde_json::Value::Object(Default::default()));

    if let Some(map) = profiles.as_object_mut() {
        map.insert(profile_name.to_string(), profile_entry);
    }

    obj.insert(
        "terminal.integrated.defaultProfile.linux".to_string(),
        serde_json::Value::String(profile_name.to_string()),
    );

    let json_str = serde_json::to_string_pretty(&settings)?;
    user::privileged_write(&settings_path, json_str.as_bytes())?;

    println!(
        "  {} wrote .vscode/settings.json (dXon terminal profile → dxon enter {})",
        "✓".green(),
        enter_target
    );

    Ok(())
}

fn strip_json_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut in_string = false;
    let mut chars = src.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                in_string = !in_string;
                out.push(ch);
            }
            '/' if !in_string => {
                if chars.peek() == Some(&'/') {
                    for c in chars.by_ref() {
                        if c == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                } else {
                    out.push(ch);
                }
            }
            '\\' if in_string => {
                out.push(ch);
                if let Some(next) = chars.next() {
                    out.push(next);
                }
            }
            _ => out.push(ch),
        }
    }
    out
}

fn host_user_command(prog: &str) -> Command {
    if user::is_root() {
        if let Ok(sudo_user) = std::env::var("SUDO_USER") {
            if !sudo_user.is_empty() {
                let mut cmd = Command::new("sudo");
                cmd.args(["--user", &sudo_user, "--", prog]);
                return cmd;
            }
        }
    }
    Command::new(prog)
}

#[derive(Debug, Serialize, Deserialize)]
struct IpcRequest {
    method: String,
    container: String,
    path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    editor: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct IpcResponse {
    ok: bool,
    message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn workspace() -> TempDir {
        TempDir::new().unwrap()
    }

    #[test]
    fn strips_single_line_comments() {
        let input = r#"{ // a comment
    "key": "value" // trailing
}"#;
        let result = strip_json_comments(input);
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["key"], "value");
    }

    #[test]
    fn leaves_url_strings_intact() {
        let input = r#"{ "url": "https://example.com" }"#;
        let result = strip_json_comments(input);
        let v: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(v["url"], "https://example.com");
    }

    #[test]
    fn creates_vscode_dir_and_settings_when_absent() {
        let ws = workspace();
        ensure_vscode_terminal_profile(ws.path(), "myenv/.").unwrap();

        let settings_path = ws.path().join(".vscode/settings.json");
        assert!(settings_path.exists());

        let v: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&settings_path).unwrap()).unwrap();

        assert_eq!(v["terminal.integrated.defaultProfile.linux"], "dXon");
        let profile = &v["terminal.integrated.profiles.linux"]["dXon"];
        assert_eq!(profile["path"], "dxon");
        assert_eq!(profile["args"][0], "enter");
        assert_eq!(profile["args"][1], "myenv/.");
    }

    #[test]
    fn merges_into_existing_settings_without_overwriting_other_keys() {
        let ws = workspace();
        let vscode = ws.path().join(".vscode");
        fs::create_dir_all(&vscode).unwrap();

        let existing = serde_json::json!({
            "editor.fontSize": 14,
            "terminal.integrated.profiles.linux": {
                "bash": { "path": "bash" }
            }
        });
        fs::write(
            vscode.join("settings.json"),
            serde_json::to_string_pretty(&existing).unwrap(),
        )
        .unwrap();

        ensure_vscode_terminal_profile(ws.path(), "devbox/src").unwrap();

        let v: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(vscode.join("settings.json")).unwrap())
                .unwrap();

        assert_eq!(v["editor.fontSize"], 14);
        assert_eq!(
            v["terminal.integrated.profiles.linux"]["bash"]["path"],
            "bash"
        );
        assert_eq!(
            v["terminal.integrated.profiles.linux"]["dXon"]["path"],
            "dxon"
        );
        assert_eq!(v["terminal.integrated.defaultProfile.linux"], "dXon");
    }

    #[test]
    fn updates_existing_dxon_profile_with_new_container_name() {
        let ws = workspace();
        let vscode = ws.path().join(".vscode");
        fs::create_dir_all(&vscode).unwrap();

        let old = serde_json::json!({
            "terminal.integrated.profiles.linux": {
                "dXon": { "path": "dxon", "args": ["enter", "old"] }
            },
            "terminal.integrated.defaultProfile.linux": "dXon"
        });
        fs::write(
            vscode.join("settings.json"),
            serde_json::to_string_pretty(&old).unwrap(),
        )
        .unwrap();

        ensure_vscode_terminal_profile(ws.path(), "new/pkg").unwrap();

        let v: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(vscode.join("settings.json")).unwrap())
                .unwrap();

        assert_eq!(
            v["terminal.integrated.profiles.linux"]["dXon"]["args"][1],
            "new/pkg"
        );
    }

    #[test]
    fn parses_jsonc_settings_file() {
        let ws = workspace();
        let vscode = ws.path().join(".vscode");
        fs::create_dir_all(&vscode).unwrap();

        let jsonc = r#"{
    // User preference
    "editor.tabSize": 4
}"#;
        fs::write(vscode.join("settings.json"), jsonc).unwrap();

        ensure_vscode_terminal_profile(ws.path(), "ctr/.").unwrap();

        let v: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(vscode.join("settings.json")).unwrap())
                .unwrap();

        assert_eq!(v["editor.tabSize"], 4);
        assert_eq!(v["terminal.integrated.defaultProfile.linux"], "dXon");
    }

    #[test]
    fn parse_open_target_requires_container_and_folder() {
        assert!(parse_open_target("devbox").is_err());
        let (name, folder) = parse_open_target("devbox/src").unwrap();
        assert_eq!(name, "devbox");
        assert_eq!(folder, PathBuf::from("src"));
    }

    #[test]
    fn sanitize_workspace_subpath_rejects_parent_traversal() {
        assert!(sanitize_workspace_subpath("../etc").is_err());
        assert!(sanitize_workspace_subpath("a/../../b").is_err());
    }

    #[test]
    fn workspace_subpath_from_container_path_requires_workspace_prefix() {
        assert!(workspace_subpath_from_container_path("/tmp").is_err());
        assert_eq!(
            workspace_subpath_from_container_path("/workspace/project").unwrap(),
            PathBuf::from("project")
        );
    }
}
