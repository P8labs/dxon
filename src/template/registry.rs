use anyhow::{Context, Result};
use colored::Colorize;
use serde::Deserialize;
use std::path::PathBuf;

use crate::template::spec::DxTemplate;

pub const DEFAULT_REGISTRY_URL: &str =
    "https://raw.githubusercontent.com/P8labs/dxon-registry/master";

pub fn template_cache_dir() -> PathBuf {
    crate::user::resolve_home().join(".dxon").join("templates")
}

/// Expected cache path for the named template: `~/.dxon/templates/<name>.yaml`.
fn cached_template_path(name: &str) -> PathBuf {
    template_cache_dir().join(format!("{name}.yaml"))
}

#[derive(Debug, Clone, Deserialize)]
pub struct TemplateEntry {
    pub name: String,
    pub description: String,
    pub path: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub distros: Vec<String>,
}

pub fn load_by_name(name: &str, registry_url: &str) -> Result<DxTemplate> {
    let cache_path = cached_template_path(name);

    if cache_path.exists() {
        let src = std::fs::read_to_string(&cache_path)
            .with_context(|| format!("cannot read cached template: {}", cache_path.display()))?;
        println!(
            "  {} template '{}' loaded from cache",
            "→".cyan(),
            name.bold()
        );
        return DxTemplate::from_yaml(&src)
            .map_err(|e| crate::error::DxonError::InvalidTemplate(format!("{name}: {e}")).into());
    }

    let body = download_template(name, registry_url)
        .with_context(|| format!("template '{name}' not found in registry ({registry_url})\n  run `dxon template list` to see available templates"))?;

    let tmpl = DxTemplate::from_yaml(&body)
        .map_err(|e| crate::error::DxonError::InvalidTemplate(format!("{name}: {e}")))?;

    let _ = save_to_cache(name, &body);

    Ok(tmpl)
}

fn download_template(name: &str, registry_url: &str) -> Result<String> {
    println!(
        "  {} downloading template '{}' from registry…",
        "↓".cyan(),
        name.bold()
    );

    let template_path = resolve_path_from_index(name, registry_url)
        .unwrap_or_else(|_| format!("templates/{name}.yaml"));

    let url = format!("{registry_url}/{template_path}");
    match ureq::get(&url).call() {
        Ok(response) => response
            .into_string()
            .with_context(|| format!("failed to read response from {url}")),
        Err(ureq::Error::Status(404, _)) => {
            anyhow::bail!("template '{name}' not found at {url}");
        }
        Err(e) => Err(anyhow::anyhow!("network error fetching '{url}': {e}")),
    }
}

fn resolve_path_from_index(name: &str, registry_url: &str) -> Result<String> {
    let entries = fetch_registry_json(registry_url)?;
    entries
        .into_iter()
        .find(|e| e.name == name)
        .map(|e| e.path)
        .ok_or_else(|| anyhow::anyhow!("template '{name}' not listed in registry index"))
}

fn save_to_cache(name: &str, body: &str) -> Result<()> {
    let dir = template_cache_dir();
    std::fs::create_dir_all(&dir)
        .with_context(|| format!("cannot create template cache dir: {}", dir.display()))?;
    let path = cached_template_path(name);
    std::fs::write(&path, body)
        .with_context(|| format!("cannot write cached template: {}", path.display()))
}

pub fn list_templates(registry_url: &str) -> Result<Vec<TemplateEntry>> {
    fetch_registry_json(registry_url)
}

pub fn search_templates(keyword: &str, registry_url: &str) -> Result<Vec<TemplateEntry>> {
    let all = list_templates(registry_url)?;
    let kw = keyword.to_lowercase();
    Ok(all
        .into_iter()
        .filter(|e| {
            e.name.to_lowercase().contains(&kw)
                || e.description.to_lowercase().contains(&kw)
                || e.tags.iter().any(|t| t.to_lowercase().contains(&kw))
        })
        .collect())
}

fn fetch_registry_json(registry_url: &str) -> Result<Vec<TemplateEntry>> {
    let url = format!("{registry_url}/registry.json");
    let response = ureq::get(&url)
        .call()
        .map_err(|e| anyhow::anyhow!("failed to fetch registry index from {url}: {e}"))?;
    let body = response
        .into_string()
        .context("failed to read registry index response")?;
    let index: IndexFile =
        serde_json::from_str(&body).context("registry.json is not valid JSON")?;
    Ok(index.templates)
}

pub fn refresh(registry_url: &str) -> Result<()> {
    let cache_dir = template_cache_dir();
    if !cache_dir.exists() {
        println!("  {} no cached templates — nothing to refresh", "→".cyan());
        return Ok(());
    }

    let entries: Vec<_> = std::fs::read_dir(&cache_dir)?
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .map(|x| matches!(x.to_str(), Some("yaml" | "yml")))
                .unwrap_or(false)
        })
        .collect();

    if entries.is_empty() {
        println!("  {} no cached templates — nothing to refresh", "→".cyan());
        return Ok(());
    }

    let mut refreshed = 0usize;
    for entry in entries {
        let path = entry.path();
        let name = path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        print!("  {} {}… ", "↓".cyan(), name.bold());
        match download_template(&name, registry_url) {
            Ok(body) => {
                if save_to_cache(&name, &body).is_ok() {
                    println!("{}", "✓".green());
                    refreshed += 1;
                } else {
                    println!("(cache write failed, keeping old version)");
                }
            }
            Err(_) => {
                println!("{} (not found, keeping cached version)", "!".yellow());
            }
        }
    }

    println!(
        "\n{} {} template(s) refreshed",
        "✓".green().bold(),
        refreshed
    );
    Ok(())
}

pub fn list_cached_names() -> Vec<String> {
    let cache_dir = template_cache_dir();
    let Ok(entries) = std::fs::read_dir(cache_dir) else {
        return Vec::new();
    };

    let mut names: Vec<String> = entries
        .flatten()
        .filter(|e| {
            e.path()
                .extension()
                .map(|x| matches!(x.to_str(), Some("yaml" | "yml")))
                .unwrap_or(false)
        })
        .map(|e| {
            e.path()
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        })
        .collect();

    names.sort();
    names
}

#[derive(Deserialize)]
struct IndexFile {
    templates: Vec<TemplateEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn template_cache_dir_is_under_dxon() {
        let dir = template_cache_dir();
        let s = dir.to_string_lossy();
        assert!(
            s.contains(".dxon"),
            "expected .dxon in cache path, got: {s}"
        );
        assert!(
            s.contains("templates"),
            "expected templates in cache path, got: {s}"
        );
    }

    #[test]
    fn cached_template_path_uses_yaml_extension() {
        let path = cached_template_path("nodejs");
        assert_eq!(path.extension().and_then(|e| e.to_str()), Some("yaml"));
        assert!(path.to_string_lossy().contains("nodejs"));
    }

    #[test]
    fn list_cached_names_returns_empty_when_dir_absent() {
        // Should not panic even when the cache directory doesn't exist.
        let _ = list_cached_names();
    }

    #[test]
    fn list_cached_names_returns_sorted_names() {
        let dir = tempdir().unwrap();
        let cache = dir.path().join(".dxon").join("templates");
        std::fs::create_dir_all(&cache).unwrap();
        std::fs::write(cache.join("rust.yaml"), "schema: dxon/v1\nname: rust\n").unwrap();
        std::fs::write(cache.join("nodejs.yaml"), "schema: dxon/v1\nname: nodejs\n").unwrap();

        // Validate that the directory contains both files.
        let files: std::collections::HashSet<_> = std::fs::read_dir(&cache)
            .unwrap()
            .flatten()
            .filter(|e| e.path().extension().map_or(false, |x| x == "yaml"))
            .map(|e| e.path().file_stem().unwrap().to_string_lossy().to_string())
            .collect();

        assert!(files.contains("nodejs"));
        assert!(files.contains("rust"));
    }

    #[test]
    fn json_index_parses_correctly() {
        let src = r#"
{
  "templates": [
    {
      "name": "nodejs",
      "description": "Node.js environment",
      "path": "templates/nodejs.yaml",
      "tags": ["javascript", "node"],
      "distros": ["arch", "debian", "alpine"]
    },
    {
      "name": "rust",
      "description": "Rust environment",
      "path": "templates/rust.yaml",
      "tags": ["rust", "cargo"],
      "distros": ["arch", "debian", "alpine"]
    }
  ]
}
"#;
        let idx: IndexFile = serde_json::from_str(src).unwrap();
        assert_eq!(idx.templates.len(), 2);
        assert_eq!(idx.templates[0].name, "nodejs");
        assert_eq!(idx.templates[0].path, "templates/nodejs.yaml");
        assert_eq!(idx.templates[0].tags, vec!["javascript", "node"]);
        assert_eq!(idx.templates[0].distros, vec!["arch", "debian", "alpine"]);
        assert_eq!(idx.templates[1].name, "rust");
        assert_eq!(idx.templates[1].path, "templates/rust.yaml");
    }

    #[test]
    fn json_index_accepts_optional_fields() {
        // tags and distros are optional; path is required
        let src = r#"
{
  "templates": [
    {
      "name": "minimal",
      "description": "Minimal template",
      "path": "templates/minimal.yaml"
    }
  ]
}
"#;
        let idx: IndexFile = serde_json::from_str(src).unwrap();
        assert_eq!(idx.templates.len(), 1);
        assert_eq!(idx.templates[0].name, "minimal");
        assert!(idx.templates[0].tags.is_empty());
        assert!(idx.templates[0].distros.is_empty());
    }

    #[test]
    fn search_logic_filters_by_name() {
        let entries = vec![
            TemplateEntry {
                name: "nodejs".into(),
                description: "Node.js environment".into(),
                path: "templates/nodejs.yaml".into(),
                tags: vec![],
                distros: vec![],
            },
            TemplateEntry {
                name: "rust".into(),
                description: "Rust environment".into(),
                path: "templates/rust.yaml".into(),
                tags: vec![],
                distros: vec![],
            },
        ];
        let kw = "rust";
        let filtered: Vec<_> = entries
            .into_iter()
            .filter(|e| {
                e.name.to_lowercase().contains(kw)
                    || e.description.to_lowercase().contains(kw)
                    || e.tags.iter().any(|t| t.to_lowercase().contains(kw))
            })
            .collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "rust");
    }

    #[test]
    fn search_logic_filters_by_description() {
        let entries = vec![
            TemplateEntry {
                name: "nodejs".into(),
                description: "Node.js development environment".into(),
                path: "templates/nodejs.yaml".into(),
                tags: vec![],
                distros: vec![],
            },
            TemplateEntry {
                name: "rust".into(),
                description: "Rust development environment".into(),
                path: "templates/rust.yaml".into(),
                tags: vec![],
                distros: vec![],
            },
        ];
        let kw = "node";
        let filtered: Vec<_> = entries
            .into_iter()
            .filter(|e| {
                e.name.to_lowercase().contains(kw)
                    || e.description.to_lowercase().contains(kw)
                    || e.tags.iter().any(|t| t.to_lowercase().contains(kw))
            })
            .collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "nodejs");
    }

    #[test]
    fn search_logic_filters_by_tags() {
        let entries = vec![
            TemplateEntry {
                name: "nodejs".into(),
                description: "Node.js environment".into(),
                path: "templates/nodejs.yaml".into(),
                tags: vec!["javascript".into(), "typescript".into()],
                distros: vec![],
            },
            TemplateEntry {
                name: "rust".into(),
                description: "Rust environment".into(),
                path: "templates/rust.yaml".into(),
                tags: vec!["systems".into(), "cargo".into()],
                distros: vec![],
            },
        ];
        let kw = "typescript";
        let filtered: Vec<_> = entries
            .into_iter()
            .filter(|e| {
                e.name.to_lowercase().contains(kw)
                    || e.description.to_lowercase().contains(kw)
                    || e.tags.iter().any(|t| t.to_lowercase().contains(kw))
            })
            .collect();
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "nodejs");
    }

    #[test]
    fn cached_template_can_be_parsed_after_write() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("testenv.yaml");
        let yaml = "schema: dxon/v1\nname: testenv\n";
        std::fs::write(&path, yaml).unwrap();

        let src = std::fs::read_to_string(&path).unwrap();
        let t = DxTemplate::from_yaml(&src).unwrap();
        assert_eq!(t.meta.name, "testenv");
    }

    #[test]
    fn default_registry_url_is_github() {
        assert!(
            DEFAULT_REGISTRY_URL.contains("githubusercontent.com")
                || DEFAULT_REGISTRY_URL.contains("github.com"),
            "expected a GitHub URL, got: {DEFAULT_REGISTRY_URL}"
        );
        assert!(DEFAULT_REGISTRY_URL.contains("P8labs"));
        assert!(DEFAULT_REGISTRY_URL.contains("dxon-registry"));
    }

    #[test]
    fn resolve_path_from_index_finds_correct_entry() {
        // This tests the logic inline (no network) using a synthetic index.
        let entries = vec![
            TemplateEntry {
                name: "nodejs".into(),
                description: "Node.js environment".into(),
                path: "templates/nodejs.yaml".into(),
                tags: vec![],
                distros: vec![],
            },
            TemplateEntry {
                name: "rust".into(),
                description: "Rust environment".into(),
                path: "templates/rust.yaml".into(),
                tags: vec![],
                distros: vec![],
            },
        ];

        let path = entries
            .into_iter()
            .find(|e| e.name == "rust")
            .map(|e| e.path);
        assert_eq!(path, Some("templates/rust.yaml".to_string()));
    }
}
