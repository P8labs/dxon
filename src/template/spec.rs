use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::template::yaml_spec::YamlTemplate;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DxTemplate {
    pub meta: TemplateMeta,
    #[serde(default)]
    pub base: BaseConfig,
    #[serde(default)]
    pub runtime: RuntimeConfig,
    #[serde(default)]
    pub prompts: Vec<Prompt>,
    #[serde(default)]
    pub steps: Vec<Step>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TemplateMeta {
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BaseConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub distro: Option<String>,
    #[serde(default)]
    pub distros: Vec<String>,
    #[serde(default)]
    pub packages: Vec<String>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub packages_by_distro: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RuntimeConfig {
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prompt {
    pub id: String,
    pub question: String,
    pub options: Vec<String>,
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step {
    pub name: String,
    #[serde(default)]
    pub distro: Option<String>,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub commands: Vec<String>,
    #[serde(default)]
    pub when: HashMap<String, String>,
}

impl DxTemplate {
    pub fn from_toml(source: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(source)
    }

    pub fn from_yaml(source: &str) -> Result<Self, String> {
        let yaml = YamlTemplate::from_yaml(source).map_err(|e| format!("YAML parse error: {e}"))?;
        yaml.validate()?;
        Ok(yaml.into_dx_template())
    }

    pub fn pinned_distro(&self) -> Option<&str> {
        self.base
            .distro
            .as_deref()
            .or_else(|| self.base.distros.first().map(|s| s.as_str()))
    }
}
