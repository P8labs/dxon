use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub containers_dir: Option<String>,
    pub default_distro: Option<String>,
    pub default_template: Option<String>,
    pub registry_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub copy_shell_config: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_shell: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_editor: Option<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let path = config_file_path()?;

        if !path.exists() {
            let cfg = Self::default();
            cfg.save()?;
            return Ok(cfg);
        }

        let src = std::fs::read_to_string(&path)
            .with_context(|| format!("cannot read config file: {}", path.display()))?;

        toml::from_str(&src)
            .with_context(|| format!("invalid TOML in config file: {}", path.display()))
    }

    pub fn save(&self) -> Result<()> {
        let path = config_file_path()?;
        let dir = path.parent().expect("config path always has a parent");

        std::fs::create_dir_all(dir).with_context(|| {
            format!(
                "cannot create config directory: {}\n  check that {} is writable",
                dir.display(),
                dir.parent().unwrap_or(dir).display()
            )
        })?;

        let content = toml::to_string_pretty(self).context("failed to serialize config")?;

        std::fs::write(&path, content)
            .with_context(|| format!("cannot write config file: {}", path.display()))?;

        Ok(())
    }

    pub fn containers_dir(&self, cli_override: Option<&str>) -> Result<PathBuf> {
        if let Some(d) = cli_override {
            if !d.is_empty() {
                return Ok(PathBuf::from(d));
            }
        }
        if let Some(ref d) = self.containers_dir {
            if !d.is_empty() {
                return Ok(PathBuf::from(d));
            }
        }
        default_containers_dir()
    }

    pub fn effective_registry_url(&self) -> &str {
        self.registry_url
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(crate::template::registry::DEFAULT_REGISTRY_URL)
    }

    pub fn set(&mut self, key: &str, value: &str) -> Result<()> {
        let opt = if value.is_empty() {
            None
        } else {
            Some(value.to_string())
        };
        match key {
            "containers_dir"    => self.containers_dir    = opt,
            "default_distro"    => self.default_distro    = opt,
            "default_template"  => self.default_template  = opt,
            "registry_url"      => self.registry_url      = opt,
            "copy_shell_config" => self.copy_shell_config = opt,
            "default_shell"     => self.default_shell     = opt,
            "default_editor"    => self.default_editor    = opt,
            _ => anyhow::bail!(
                "unknown config key '{key}'\n  valid keys: containers_dir, default_distro, default_template, registry_url, copy_shell_config, default_shell, default_editor"
            ),
        }
        Ok(())
    }
}

pub fn config_file_path() -> Result<PathBuf> {
    let home = crate::user::resolve_home();
    Ok(home.join(".config").join("dxon").join("config.toml"))
}

pub fn default_containers_dir() -> Result<PathBuf> {
    let home = crate::user::resolve_home();
    Ok(home.join(".dxon").join("containers"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_all_none_fields() {
        let cfg = Config::default();
        assert!(cfg.containers_dir.is_none());
        assert!(cfg.default_distro.is_none());
        assert!(cfg.default_template.is_none());
        assert!(cfg.registry_url.is_none());
    }

    #[test]
    fn set_containers_dir_stores_value() {
        let mut cfg = Config::default();
        cfg.set("containers_dir", "/tmp/test").unwrap();
        assert_eq!(cfg.containers_dir.as_deref(), Some("/tmp/test"));
    }

    #[test]
    fn set_default_distro_stores_value() {
        let mut cfg = Config::default();
        cfg.set("default_distro", "arch").unwrap();
        assert_eq!(cfg.default_distro.as_deref(), Some("arch"));
    }

    #[test]
    fn set_default_template_stores_value() {
        let mut cfg = Config::default();
        cfg.set("default_template", "rust").unwrap();
        assert_eq!(cfg.default_template.as_deref(), Some("rust"));
    }

    #[test]
    fn set_with_empty_string_clears_key_to_none() {
        let mut cfg = Config::default();
        cfg.set("containers_dir", "/tmp/test").unwrap();
        cfg.set("containers_dir", "").unwrap();
        assert!(cfg.containers_dir.is_none());
    }

    #[test]
    fn set_unknown_key_returns_error_with_hint() {
        let mut cfg = Config::default();
        let err = cfg.set("unknown_key", "val").unwrap_err();
        assert!(err.to_string().contains("unknown config key"));
        assert!(err.to_string().contains("unknown_key"));
        assert!(err.to_string().contains("registry_url"));
    }

    #[test]
    fn set_registry_url_stores_value() {
        let mut cfg = Config::default();
        cfg.set("registry_url", "https://example.com/registry")
            .unwrap();
        assert_eq!(
            cfg.registry_url.as_deref(),
            Some("https://example.com/registry")
        );
    }

    #[test]
    fn effective_registry_url_returns_default_when_unset() {
        let cfg = Config::default();
        assert!(!cfg.effective_registry_url().is_empty());
        assert!(cfg.effective_registry_url().starts_with("https://"));
    }

    #[test]
    fn effective_registry_url_returns_configured_value() {
        let cfg = Config {
            registry_url: Some("https://my-registry.example.com".into()),
            ..Default::default()
        };
        assert_eq!(
            cfg.effective_registry_url(),
            "https://my-registry.example.com"
        );
    }

    #[test]
    fn containers_dir_cli_override_takes_highest_priority() {
        let cfg = Config {
            containers_dir: Some("/from/config".into()),
            ..Default::default()
        };
        let result = cfg.containers_dir(Some("/from/cli")).unwrap();
        assert_eq!(result, PathBuf::from("/from/cli"));
    }

    #[test]
    fn containers_dir_config_value_used_when_no_cli_override() {
        let cfg = Config {
            containers_dir: Some("/from/config".into()),
            ..Default::default()
        };
        let result = cfg.containers_dir(None).unwrap();
        assert_eq!(result, PathBuf::from("/from/config"));
    }

    #[test]
    fn containers_dir_empty_cli_override_falls_through_to_config() {
        let cfg = Config {
            containers_dir: Some("/from/config".into()),
            ..Default::default()
        };
        let result = cfg.containers_dir(Some("")).unwrap();
        assert_eq!(result, PathBuf::from("/from/config"));
    }

    #[test]
    fn containers_dir_empty_config_value_falls_through_to_default() {
        let cfg = Config {
            containers_dir: Some("".into()),
            ..Default::default()
        };
        let result = cfg.containers_dir(None).unwrap();
        let s = result.to_string_lossy();
        assert!(s.contains(".dxon") && s.contains("containers"));
    }

    #[test]
    fn containers_dir_default_contains_dxon_containers() {
        let cfg = Config::default();
        let result = cfg.containers_dir(None).unwrap();
        let s = result.to_string_lossy();
        assert!(
            s.contains(".dxon"),
            "expected .dxon in default path, got: {s}"
        );
        assert!(
            s.contains("containers"),
            "expected containers in default path, got: {s}"
        );
    }

    #[test]
    fn config_serializes_and_deserializes_via_toml() {
        let cfg = Config {
            containers_dir: Some("/tmp/containers".into()),
            default_distro: Some("debian".into()),
            default_template: Some("rust".into()),
            ..Default::default()
        };

        let toml_str = toml::to_string_pretty(&cfg).unwrap();
        let restored: Config = toml::from_str(&toml_str).unwrap();

        assert_eq!(restored.containers_dir.as_deref(), Some("/tmp/containers"));
        assert_eq!(restored.default_distro.as_deref(), Some("debian"));
        assert_eq!(restored.default_template.as_deref(), Some("rust"));
    }

    #[test]
    fn all_none_config_toml_roundtrip_stays_none() {
        let cfg = Config::default();
        let toml_str = toml::to_string_pretty(&cfg).unwrap();
        let restored: Config = toml::from_str(&toml_str).unwrap();
        assert!(restored.containers_dir.is_none());
        assert!(restored.default_distro.is_none());
        assert!(restored.default_template.is_none());
        assert!(restored.registry_url.is_none());
    }
}
