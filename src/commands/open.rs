use anyhow::{bail, Result};
use colored::Colorize;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::container::store::ContainerStore;
use crate::user;

const EDITOR_CANDIDATES: &[&str] = &["code", "cursor", "zed"];

const VSCODE_COMPAT: &[&str] = &["code", "cursor"];

pub fn run(store: &ContainerStore, name: &str, editor_override: Option<&str>) -> Result<()> {
    let meta = store.load_meta(name)?;
    let rootfs = PathBuf::from(&meta.rootfs_path);

    if !rootfs.exists() {
        bail!(
            "rootfs directory not found: {}\n  has the container been created?",
            rootfs.display()
        );
    }

    let workspace = rootfs.join("workspace");
    let open_path = if workspace.exists() {
        workspace
    } else {
        rootfs.clone()
    };

    let editor = if let Some(e) = editor_override {
        e.to_string()
    } else {
        detect_editor()?
    };

    let editor_bin = editor.split_whitespace().next().unwrap_or("");

    if VSCODE_COMPAT.contains(&editor_bin) {
        ensure_vscode_terminal_profile(&open_path, name)?;
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
            name
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

fn ensure_vscode_terminal_profile(workspace: &Path, container_name: &str) -> Result<()> {
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
        "args": ["enter", container_name],
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
        container_name
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
        ensure_vscode_terminal_profile(ws.path(), "myenv").unwrap();

        let settings_path = ws.path().join(".vscode/settings.json");
        assert!(settings_path.exists());

        let v: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&settings_path).unwrap()).unwrap();

        assert_eq!(v["terminal.integrated.defaultProfile.linux"], "dXon");
        let profile = &v["terminal.integrated.profiles.linux"]["dXon"];
        assert_eq!(profile["path"], "dxon");
        assert_eq!(profile["args"][0], "enter");
        assert_eq!(profile["args"][1], "myenv");
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

        ensure_vscode_terminal_profile(ws.path(), "devbox").unwrap();

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

        ensure_vscode_terminal_profile(ws.path(), "new").unwrap();

        let v: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(vscode.join("settings.json")).unwrap())
                .unwrap();

        assert_eq!(
            v["terminal.integrated.profiles.linux"]["dXon"]["args"][1],
            "new"
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

        ensure_vscode_terminal_profile(ws.path(), "ctr").unwrap();

        let v: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(vscode.join("settings.json")).unwrap())
                .unwrap();

        assert_eq!(v["editor.tabSize"], 4);
        assert_eq!(v["terminal.integrated.defaultProfile.linux"], "dXon");
    }
}
