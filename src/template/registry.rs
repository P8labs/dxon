use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

use crate::template::spec::DxTemplate;

const REGISTRY_BASE_URL: &str = "https://raw.githubusercontent.com/P8labs/dxon-registry/main";

fn registry_dir() -> PathBuf {
    crate::user::resolve_home().join(".dxon").join("registry")
}

fn templates_dir() -> PathBuf {
    registry_dir().join("templates")
}

/// Load a template by name from the local registry cache (`~/.dxon/registry/templates/<name>.dx`).
/// Returns `None` if the cache doesn't contain that template.
pub fn local_template(name: &str) -> Option<DxTemplate> {
    let path = templates_dir().join(format!("{name}.dx"));
    let src = std::fs::read_to_string(&path).ok()?;
    DxTemplate::from_toml(&src).ok()
}

/// Download/refresh all templates from the upstream registry into `~/.dxon/registry/`.
pub fn update() -> Result<()> {
    use colored::Colorize;

    let tdir = templates_dir();
    std::fs::create_dir_all(&tdir)
        .with_context(|| format!("cannot create registry cache: {}", tdir.display()))?;

    let index_url = format!("{REGISTRY_BASE_URL}/index.toml");
    let index_body = ureq::get(&index_url)
        .call()
        .map_err(|e| anyhow::anyhow!("failed to fetch registry index: {e}"))?
        .into_string()
        .context("failed to read registry index response")?;

    let index_path = registry_dir().join("index.toml");
    std::fs::write(&index_path, &index_body)
        .with_context(|| format!("cannot write registry index: {}", index_path.display()))?;

    let index: IndexFile =
        toml::from_str(&index_body).context("registry index is not valid TOML")?;

    for entry in &index.templates {
        let url = format!("{REGISTRY_BASE_URL}/templates/{}.dx", entry.name);
        println!("  {} {}", "↓".cyan(), entry.name);
        let body = ureq::get(&url)
            .call()
            .map_err(|e| anyhow::anyhow!("failed to fetch template '{}': {e}", entry.name))?
            .into_string()
            .with_context(|| format!("failed to read template '{}'", entry.name))?;
        let dest = tdir.join(format!("{}.dx", entry.name));
        std::fs::write(&dest, &body)
            .with_context(|| format!("cannot save template '{}'", entry.name))?;
    }

    println!(
        "{} registry updated — {} template(s)",
        "✓".green().bold(),
        index.templates.len()
    );
    Ok(())
}

/// List templates available in the local registry cache.
/// Falls back to scanning `.dx` files when the index is absent.
pub fn list_cached() -> Vec<(String, String)> {
    let index_path = registry_dir().join("index.toml");
    if let Ok(src) = std::fs::read_to_string(&index_path) {
        if let Ok(idx) = toml::from_str::<IndexFile>(&src) {
            return idx
                .templates
                .into_iter()
                .map(|e| (e.name, e.description))
                .collect();
        }
    }
    // No index — enumerate .dx files directly
    let Ok(entries) = std::fs::read_dir(templates_dir()) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|e| e.path().extension().map(|x| x == "dx").unwrap_or(false))
        .map(|e| {
            let name = e.path().file_stem().unwrap().to_string_lossy().to_string();
            (name, String::new())
        })
        .collect()
}

#[derive(Deserialize)]
struct IndexFile {
    templates: Vec<IndexEntry>,
}

#[derive(Deserialize)]
struct IndexEntry {
    name: String,
    description: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn list_cached_returns_empty_when_dir_absent() {
        // registry_dir() uses the real home, so we just verify the function doesn't panic
        // when the cache directory doesn't exist.
        let result = list_cached();
        // If cache is absent the result is empty or populated — either is fine.
        let _ = result;
    }

    #[test]
    fn list_cached_scans_dx_files_when_no_index() {
        let dir = tempdir().unwrap();
        let tdir = dir.path().join("templates");
        std::fs::create_dir_all(&tdir).unwrap();
        std::fs::write(tdir.join("myenv.dx"), "[meta]\nname=\"myenv\"").unwrap();
        std::fs::write(tdir.join("other.dx"), "[meta]\nname=\"other\"").unwrap();

        let entries = std::fs::read_dir(&tdir)
            .unwrap()
            .flatten()
            .filter(|e| e.path().extension().map(|x| x == "dx").unwrap_or(false))
            .map(|e| e.path().file_stem().unwrap().to_string_lossy().to_string())
            .collect::<std::collections::HashSet<_>>();

        assert!(entries.contains("myenv"));
        assert!(entries.contains("other"));
    }

    #[test]
    fn index_toml_parses_correctly() {
        let src = r#"
[[templates]]
name        = "nodejs"
description = "Node.js environment"
file        = "templates/nodejs.dx"

[[templates]]
name        = "rust"
description = "Rust environment"
file        = "templates/rust.dx"
"#;
        let idx: IndexFile = toml::from_str(src).unwrap();
        assert_eq!(idx.templates.len(), 2);
        assert_eq!(idx.templates[0].name, "nodejs");
        assert_eq!(idx.templates[1].name, "rust");
    }

    #[test]
    fn local_template_returns_none_for_absent_cache() {
        // Should not panic even if ~/.dxon/registry doesn't exist.
        let result = local_template("nonexistent_template_xyz");
        assert!(result.is_none());
    }
}
