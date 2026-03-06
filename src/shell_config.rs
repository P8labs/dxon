use anyhow::Result;
use colored::Colorize;
use std::path::{Path, PathBuf};

use crate::user;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellConfigMode {
    Copy,
    Bind,
}

impl ShellConfigMode {
    pub fn parse(s: &str) -> anyhow::Result<Self> {
        match s.to_ascii_lowercase().as_str() {
            "copy" => Ok(Self::Copy),
            "bind" => Ok(Self::Bind),
            other => {
                anyhow::bail!("invalid --shell-config mode '{other}'\n  valid modes: copy, bind")
            }
        }
    }
}

fn config_files_for(shell: &str, host_home: &Path) -> Vec<(PathBuf, PathBuf)> {
    let mut files: Vec<(PathBuf, PathBuf)> = Vec::new();

    match shell {
        "bash" => {
            for name in &[
                ".profile",
                ".bash_profile",
                ".bash_login",
                ".bashrc",
                ".bash_aliases",
                ".bash_logout",
                ".inputrc",
            ] {
                let p = PathBuf::from(name);
                files.push((p.clone(), p));
            }
        }
        "zsh" => {
            for name in &[".zshenv", ".inputrc"] {
                let p = PathBuf::from(name);
                files.push((p.clone(), p));
            }

            let zdotdir = std::env::var("ZDOTDIR")
                .ok()
                .filter(|s| !s.is_empty())
                .map(PathBuf::from);

            if let Some(ref zd) = zdotdir {
                if let Ok(rel) = zd.strip_prefix(host_home) {
                    for name in &[".zshrc", ".zprofile", ".zlogin", ".zlogout"] {
                        let file_rel = rel.join(name);
                        files.push((file_rel.clone(), file_rel));
                    }
                } else {
                    for name in &[".zshrc", ".zprofile", ".zlogin", ".zlogout"] {
                        let p = PathBuf::from(name);
                        files.push((p.clone(), p));
                    }
                }
            } else {
                for name in &[".zshrc", ".zprofile", ".zlogin", ".zlogout"] {
                    let p = PathBuf::from(name);
                    files.push((p.clone(), p));
                }
            }
        }
        "fish" => {
            let fish_dir = std::env::var("XDG_CONFIG_HOME")
                .ok()
                .filter(|s| !s.is_empty())
                .map(|xdg| PathBuf::from(xdg).join("fish"))
                .unwrap_or_else(|| host_home.join(".config/fish"));

            let host_rel = if let Ok(rel) = fish_dir.strip_prefix(host_home) {
                rel.to_path_buf()
            } else {
                PathBuf::from(".config/fish")
            };

            let container_rel = PathBuf::from(".config/fish");
            files.push((host_rel, container_rel));
        }
        _ => {
            for name in &[".profile", ".inputrc"] {
                let p = PathBuf::from(name);
                files.push((p.clone(), p));
            }
        }
    }

    let mut seen = std::collections::HashSet::new();
    files.retain(|(host_rel, _)| seen.insert(host_rel.clone()));

    files
        .into_iter()
        .filter(|(host_rel, _)| host_home.join(host_rel).exists())
        .collect()
}

pub fn apply_copy(rootfs: &Path, host_home: &Path, shell: &str) -> Result<()> {
    let container_home = rootfs.join("root");

    let files = config_files_for(shell, host_home);
    if files.is_empty() {
        println!(
            "  {} no shell config files found for {} in {}",
            "↳".dimmed(),
            shell.bold(),
            host_home.display()
        );
        return Ok(());
    }

    println!(
        "{} copying {} shell config into container…",
        "→".cyan(),
        shell.bold()
    );

    for (host_rel, container_rel) in &files {
        let src = host_home.join(host_rel);
        let dst = container_home.join(container_rel);

        if let Some(parent) = dst.parent() {
            user::privileged_mkdir(parent)?;
        }

        if src.is_dir() {
            copy_dir_all(&src, &dst, host_home, &PathBuf::from("/root"))?;
        } else {
            copy_file_with_path_sub(&src, &dst, host_home, &PathBuf::from("/root"))?;
        }

        println!("  {} {}", "↳".dimmed(), host_rel.display());
    }

    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path, host_home: &Path, container_home: &Path) -> Result<()> {
    user::privileged_mkdir(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_all(&src_path, &dst_path, host_home, container_home)?;
        } else {
            copy_file_with_path_sub(&src_path, &dst_path, host_home, container_home)?;
        }
    }
    Ok(())
}

fn copy_file_with_path_sub(
    src: &Path,
    dst: &Path,
    host_home: &Path,
    container_home: &Path,
) -> Result<()> {
    let bytes = std::fs::read(src)?;

    let content = match std::str::from_utf8(&bytes) {
        Ok(text) => {
            let host_str = host_home.to_string_lossy();
            let container_str = container_home.to_string_lossy();
            if text.contains(host_str.as_ref()) {
                text.replace(host_str.as_ref(), container_str.as_ref())
                    .into_bytes()
            } else {
                bytes
            }
        }
        Err(_) => bytes,
    };

    user::privileged_write(dst, &content)
}

pub fn bind_args(host_home: &Path, shell: &str) -> Vec<String> {
    let files = config_files_for(shell, host_home);

    files
        .into_iter()
        .map(|(host_rel, container_rel)| {
            let host_path = host_home.join(&host_rel);
            let container_path = PathBuf::from("/root").join(&container_rel);
            format!(
                "--bind={}:{}",
                host_path.display(),
                container_path.display()
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_files(dir: &Path, names: &[&str]) {
        for name in names {
            let path = dir.join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).unwrap();
            }
            fs::write(&path, format!("# {name}")).unwrap();
        }
    }

    #[test]
    fn parse_copy_and_bind() {
        assert_eq!(
            ShellConfigMode::parse("copy").unwrap(),
            ShellConfigMode::Copy
        );
        assert_eq!(
            ShellConfigMode::parse("bind").unwrap(),
            ShellConfigMode::Bind
        );
        assert_eq!(
            ShellConfigMode::parse("COPY").unwrap(),
            ShellConfigMode::Copy
        );
    }

    #[test]
    fn parse_invalid_mode_returns_error() {
        assert!(ShellConfigMode::parse("mount").is_err());
        assert!(ShellConfigMode::parse("").is_err());
    }

    #[test]
    fn bash_picks_up_standard_files() {
        let home = TempDir::new().unwrap();
        make_files(
            home.path(),
            &[
                ".bashrc",
                ".bash_profile",
                ".bash_aliases",
                ".bash_logout",
                ".inputrc",
            ],
        );
        let files = config_files_for("bash", home.path());
        let names: Vec<_> = files.iter().map(|(h, _)| h.to_str().unwrap()).collect();
        assert!(names.contains(&".bashrc"));
        assert!(names.contains(&".bash_profile"));
        assert!(names.contains(&".bash_aliases"));
        assert!(names.contains(&".bash_logout"));
        assert!(names.contains(&".inputrc"));
    }

    #[test]
    fn bash_includes_profile_and_bash_login() {
        let home = TempDir::new().unwrap();
        make_files(home.path(), &[".profile", ".bash_login"]);
        let files = config_files_for("bash", home.path());
        let names: Vec<_> = files.iter().map(|(h, _)| h.to_str().unwrap()).collect();
        assert!(names.contains(&".profile"));
        assert!(names.contains(&".bash_login"));
    }

    #[test]
    fn bash_skips_missing_files() {
        let home = TempDir::new().unwrap();
        make_files(home.path(), &[".bashrc"]);
        let files = config_files_for("bash", home.path());
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].0, PathBuf::from(".bashrc"));
    }

    #[test]
    fn zsh_picks_up_standard_files() {
        let home = TempDir::new().unwrap();
        make_files(
            home.path(),
            &[
                ".zshenv",
                ".zshrc",
                ".zprofile",
                ".zlogin",
                ".zlogout",
                ".inputrc",
            ],
        );
        let files = config_files_for("zsh", home.path());
        let names: Vec<_> = files.iter().map(|(h, _)| h.to_str().unwrap()).collect();
        assert!(names.contains(&".zshenv"));
        assert!(names.contains(&".zshrc"));
        assert!(names.contains(&".zprofile"));
        assert!(names.contains(&".zlogin"));
        assert!(names.contains(&".zlogout"));
        assert!(names.contains(&".inputrc"));
    }

    #[test]
    fn zsh_zdotdir_under_home_includes_config_files() {
        let home = TempDir::new().unwrap();
        let zdotdir = home.path().join(".config/zsh");
        fs::create_dir_all(&zdotdir).unwrap();
        make_files(&zdotdir, &[".zshrc", ".zshenv"]);
        make_files(home.path(), &[".zshenv"]);

        std::env::set_var("ZDOTDIR", zdotdir.to_str().unwrap());
        let files = config_files_for("zsh", home.path());
        std::env::remove_var("ZDOTDIR");

        let host_rels: Vec<_> = files.iter().map(|(h, _)| h.clone()).collect();
        assert!(host_rels
            .iter()
            .any(|p| p == &PathBuf::from(".config/zsh/.zshrc")));
        assert!(host_rels.iter().any(|p| p == &PathBuf::from(".zshenv")));
        assert!(!host_rels.iter().any(|p| p == &PathBuf::from(".config/zsh")));
    }

    #[test]
    fn zsh_no_zdotdir_uses_standard_dotfiles() {
        let home = TempDir::new().unwrap();
        make_files(home.path(), &[".zshrc", ".zshenv"]);
        std::env::remove_var("ZDOTDIR");
        let files = config_files_for("zsh", home.path());
        let names: Vec<_> = files.iter().map(|(h, _)| h.to_str().unwrap()).collect();
        assert!(names.contains(&".zshrc"));
        assert!(names.contains(&".zshenv"));
    }

    #[test]
    fn fish_default_config_dir() {
        let home = TempDir::new().unwrap();
        let fish_dir = home.path().join(".config/fish");
        fs::create_dir_all(&fish_dir).unwrap();
        make_files(&fish_dir, &["config.fish"]);

        std::env::remove_var("XDG_CONFIG_HOME");
        let files = config_files_for("fish", home.path());
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].0, PathBuf::from(".config/fish"));
        assert_eq!(files[0].1, PathBuf::from(".config/fish"));
    }

    #[test]
    fn fish_respects_xdg_config_home_under_home() {
        let home = TempDir::new().unwrap();
        let xdg = home.path().join(".local/config");
        let fish_dir = xdg.join("fish");
        fs::create_dir_all(&fish_dir).unwrap();
        make_files(&fish_dir, &["config.fish"]);

        std::env::set_var("XDG_CONFIG_HOME", xdg.to_str().unwrap());
        let files = config_files_for("fish", home.path());
        std::env::remove_var("XDG_CONFIG_HOME");

        assert_eq!(files.len(), 1);
        assert_eq!(files[0].0, PathBuf::from(".local/config/fish"));
        assert_eq!(files[0].1, PathBuf::from(".config/fish"));
    }

    #[test]
    fn fish_absent_config_dir_returns_empty() {
        let home = TempDir::new().unwrap();
        std::env::remove_var("XDG_CONFIG_HOME");
        let files = config_files_for("fish", home.path());
        assert!(files.is_empty());
    }

    #[test]
    fn copy_file_rewrites_home_paths() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();

        let host_home = src_dir.path();
        let container_home = PathBuf::from("/root");

        let src = src_dir.path().join(".bashrc");
        let content = format!("export PATH={}/bin:$PATH", host_home.display());
        fs::write(&src, &content).unwrap();

        let dst = dst_dir.path().join(".bashrc");
        copy_file_with_path_sub(&src, &dst, host_home, &container_home).unwrap();

        let result = fs::read_to_string(&dst).unwrap();
        assert!(result.contains("/root/bin"));
        assert!(!result.contains(&host_home.display().to_string()));
    }

    #[test]
    fn copy_file_leaves_binary_files_intact() {
        let src_dir = TempDir::new().unwrap();
        let dst_dir = TempDir::new().unwrap();

        let src = src_dir.path().join("binary");
        let binary: Vec<u8> = vec![0xFF, 0xFE, 0x00, 0x01];
        fs::write(&src, &binary).unwrap();

        let dst = dst_dir.path().join("binary");
        copy_file_with_path_sub(&src, &dst, src_dir.path(), &PathBuf::from("/root")).unwrap();

        assert_eq!(fs::read(&dst).unwrap(), binary);
    }

    #[test]
    fn bind_args_format_for_bash() {
        let home = TempDir::new().unwrap();
        make_files(home.path(), &[".bashrc"]);
        std::env::remove_var("ZDOTDIR");
        std::env::remove_var("XDG_CONFIG_HOME");

        let files = config_files_for("bash", home.path());
        assert!(!files.is_empty());

        let args: Vec<String> = files
            .into_iter()
            .map(|(host_rel, container_rel)| {
                let host_path = home.path().join(&host_rel);
                let container_path = PathBuf::from("/root").join(&container_rel);
                format!(
                    "--bind={}:{}",
                    host_path.display(),
                    container_path.display()
                )
            })
            .collect();

        assert!(args[0].starts_with("--bind="));
        assert!(args[0].contains(":/root/"));
    }
}
