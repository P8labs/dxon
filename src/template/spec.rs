use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
        let yaml = crate::template::yaml_spec::YamlTemplate::from_yaml(source)
            .map_err(|e| format!("YAML parse error: {e}"))?;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_template_only_meta_name() {
        let t = DxTemplate::from_toml("[meta]\nname = \"test\"").unwrap();
        assert_eq!(t.meta.name, "test");
        assert!(t.base.packages.is_empty());
        assert!(t.steps.is_empty());
        assert!(t.prompts.is_empty());
    }

    #[test]
    fn parse_full_template() {
        let src = r#"
[meta]
name        = "myenv"
description = "Test env"
version     = "1.0.0"
author      = "tester"

[base]
packages = ["git", "curl"]

[runtime]
env      = { FOO = "bar" }
commands = ["echo hello"]

[[prompts]]
id       = "pm"
question = "Which?"
options  = ["npm", "yarn"]
default  = "npm"

[[steps]]
name     = "Step one"
tools    = ["nodejs"]
commands = ["node --version"]

[[steps]]
name   = "Step two"
distro = "arch"
tools  = ["gcc"]
[steps.when]
pm = "yarn"
"#;
        let t = DxTemplate::from_toml(src).unwrap();
        assert_eq!(t.meta.name, "myenv");
        assert_eq!(t.base.packages, vec!["git", "curl"]);
        assert_eq!(t.runtime.env.get("FOO"), Some(&"bar".to_string()));
        assert_eq!(t.runtime.commands, vec!["echo hello"]);
        assert_eq!(t.prompts.len(), 1);
        assert_eq!(t.prompts[0].id, "pm");
        assert_eq!(t.prompts[0].options, vec!["npm", "yarn"]);
        assert_eq!(t.prompts[0].default, Some("npm".into()));
        assert_eq!(t.steps.len(), 2);
        assert_eq!(t.steps[1].distro, Some("arch".into()));
        assert_eq!(t.steps[1].when.get("pm"), Some(&"yarn".to_string()));
    }

    #[test]
    fn parse_invalid_toml_returns_error() {
        assert!(DxTemplate::from_toml("[[[ not valid toml").is_err());
    }

    #[test]
    fn missing_required_meta_name_returns_parse_error() {
        let src = "[meta]\ndescription = \"no name\"";
        assert!(
            DxTemplate::from_toml(src).is_err(),
            "expected parse error when required meta.name is absent"
        );
    }

    #[test]
    fn step_with_no_tools_or_commands_or_when() {
        let src = "[meta]\nname = \"x\"\n[[steps]]\nname = \"empty step\"";
        let t = DxTemplate::from_toml(src).unwrap();
        assert_eq!(t.steps.len(), 1);
        assert!(t.steps[0].tools.is_empty());
        assert!(t.steps[0].commands.is_empty());
        assert!(t.steps[0].when.is_empty());
        assert!(t.steps[0].distro.is_none());
    }

    #[test]
    fn prompt_with_no_default_parses_as_none() {
        let src = r#"
[meta]
name = "x"

[[prompts]]
id       = "q"
question = "?"
options  = ["a", "b"]
"#;
        let t = DxTemplate::from_toml(src).unwrap();
        assert!(t.prompts[0].default.is_none());
    }

    #[test]
    fn multiple_steps_with_when_conditions() {
        let src = r#"
[meta]
name = "x"

[[steps]]
name = "step-a"
[steps.when]
flag = "yes"

[[steps]]
name = "step-b"
[steps.when]
flag = "no"
other = "val"
"#;
        let t = DxTemplate::from_toml(src).unwrap();
        assert_eq!(t.steps[0].when.get("flag"), Some(&"yes".to_string()));
        assert_eq!(t.steps[1].when.len(), 2);
    }

    #[test]
    fn runtime_env_multiple_vars() {
        let src = r#"
[meta]
name = "x"

[runtime]
env = { HOME = "/root", PATH = "/usr/bin" }
"#;
        let t = DxTemplate::from_toml(src).unwrap();
        assert_eq!(t.runtime.env.get("HOME"), Some(&"/root".to_string()));
        assert_eq!(t.runtime.env.get("PATH"), Some(&"/usr/bin".to_string()));
    }

    #[test]
    fn pinned_distro_prefers_explicit_base_distro() {
        let src = r#"
[meta]
name = "x"

[base]
distro = "debian"
distros = ["arch"]
"#;
        let t = DxTemplate::from_toml(src).unwrap();
        assert_eq!(t.pinned_distro(), Some("debian"));
    }

    #[test]
    fn pinned_distro_falls_back_to_first_distros_entry() {
        let src = r#"
[meta]
name = "x"

[base]
distros = ["alpine"]
"#;
        let t = DxTemplate::from_toml(src).unwrap();
        assert_eq!(t.pinned_distro(), Some("alpine"));
    }
}
