use serde::Deserialize;
use std::collections::HashMap;

use crate::template::spec::{BaseConfig, DxTemplate, Prompt, RuntimeConfig, Step, TemplateMeta};

pub const SCHEMA_VERSION: &str = "dxon/v1";

#[derive(Debug, Deserialize)]
pub struct YamlTemplate {
    pub schema: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub base: Option<String>,
    #[serde(default)]
    pub packages: HashMap<String, Vec<String>>,
    #[serde(default)]
    pub env: HashMap<String, String>,
    #[serde(default)]
    pub run: Vec<String>,
    #[serde(default)]
    pub options: Vec<YamlOption>,
    #[serde(default)]
    pub steps: Vec<YamlStep>,
}

#[derive(Debug, Deserialize)]
pub struct YamlOption {
    pub id: String,
    pub prompt: String,
    pub choices: Vec<String>,
    #[serde(default)]
    pub default: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct YamlStep {
    pub name: String,
    #[serde(default)]
    pub distro: Option<String>,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_run")]
    pub run: Vec<String>,
    #[serde(default)]
    pub when: HashMap<String, String>,
}

fn deserialize_run<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::{self, Visitor};

    struct RunVisitor;

    impl<'de> Visitor<'de> for RunVisitor {
        type Value = Vec<String>;

        fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "a string or list of strings")
        }

        fn visit_str<E: de::Error>(self, v: &str) -> Result<Self::Value, E> {
            Ok(vec![v.to_string()])
        }

        fn visit_string<E: de::Error>(self, v: String) -> Result<Self::Value, E> {
            Ok(vec![v])
        }

        fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
            let mut out = Vec::new();
            while let Some(s) = seq.next_element::<String>()? {
                out.push(s);
            }
            Ok(out)
        }

        fn visit_none<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(Vec::new())
        }

        fn visit_unit<E: de::Error>(self) -> Result<Self::Value, E> {
            Ok(Vec::new())
        }
    }

    deserializer.deserialize_any(RunVisitor)
}

impl YamlTemplate {
    pub fn from_yaml(source: &str) -> Result<Self, serde_yaml::Error> {
        serde_yaml::from_str(source)
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.schema != SCHEMA_VERSION {
            return Err(format!(
                "unsupported schema '{}' — expected '{SCHEMA_VERSION}'",
                self.schema
            ));
        }
        if self.name.is_empty() {
            return Err("template 'name' field is required and must not be empty".to_string());
        }
        for (i, opt) in self.options.iter().enumerate() {
            if opt.id.is_empty() {
                return Err(format!("options[{i}]: 'id' is required"));
            }
            if opt.choices.is_empty() {
                return Err(format!(
                    "options[{}] '{}': 'choices' must not be empty",
                    i, opt.id
                ));
            }
            if let Some(ref d) = opt.default {
                if !opt.choices.contains(d) {
                    return Err(format!(
                        "options[{}] '{}': default '{}' is not in choices {:?}",
                        i, opt.id, d, opt.choices
                    ));
                }
            }
        }
        for (i, step) in self.steps.iter().enumerate() {
            if step.name.is_empty() {
                return Err(format!("steps[{i}]: 'name' is required"));
            }
        }
        Ok(())
    }

    pub fn into_dx_template(self) -> DxTemplate {
        let meta = TemplateMeta {
            name: self.name,
            description: self.description,
            version: String::new(),
            author: String::new(),
        };

        let base = BaseConfig {
            distro: self.base.clone(),
            distros: self.base.into_iter().collect(),
            packages: Vec::new(),
            packages_by_distro: self.packages,
        };

        let runtime = RuntimeConfig {
            env: self.env,
            commands: self.run,
        };

        let prompts: Vec<Prompt> = self
            .options
            .into_iter()
            .map(|o| Prompt {
                id: o.id,
                question: o.prompt,
                options: o.choices,
                default: o.default,
            })
            .collect();

        let steps: Vec<Step> = self
            .steps
            .into_iter()
            .map(|s| Step {
                name: s.name,
                distro: s.distro,
                tools: s.tools,
                commands: s.run,
                when: s.when,
            })
            .collect();

        DxTemplate {
            meta,
            base,
            runtime,
            prompts,
            steps,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_yaml() -> &'static str {
        "schema: dxon/v1\nname: test\n"
    }

    #[test]
    fn parse_minimal_template() {
        let t = YamlTemplate::from_yaml(minimal_yaml()).unwrap();
        assert_eq!(t.schema, "dxon/v1");
        assert_eq!(t.name, "test");
        assert!(t.packages.is_empty());
        assert!(t.options.is_empty());
        assert!(t.steps.is_empty());
    }

    #[test]
    fn validate_passes_for_valid_template() {
        let t = YamlTemplate::from_yaml(minimal_yaml()).unwrap();
        assert!(t.validate().is_ok());
    }

    #[test]
    fn validate_rejects_wrong_schema() {
        let src = "schema: dxon/v2\nname: test\n";
        let t = YamlTemplate::from_yaml(src).unwrap();
        assert!(t.validate().is_err());
    }

    #[test]
    fn validate_rejects_empty_name() {
        let src = "schema: dxon/v1\nname: \"\"\n";
        let t = YamlTemplate::from_yaml(src).unwrap();
        let err = t.validate();
        assert!(err.is_err());
        assert!(err.unwrap_err().contains("name"));
    }

    #[test]
    fn validate_rejects_default_not_in_choices() {
        let src = r#"
schema: dxon/v1
name: test
options:
  - id: pm
    prompt: "Pick one"
    choices: [npm, yarn]
    default: pnpm
"#;
        let t = YamlTemplate::from_yaml(src).unwrap();
        assert!(t.validate().is_err());
    }

    #[test]
    fn parse_full_template() {
        let src = r#"
schema: dxon/v1
name: nodejs
description: Node.js development environment

packages:
  arch:   [curl, git, nodejs, npm]
  debian: [curl, git, ca-certificates, nodejs, npm]
  alpine: [curl, git, ca-certificates, nodejs, npm]

env:
  NODE_ENV: development

run:
  - echo "setup complete"

options:
  - id: pkg_manager
    prompt: "Which package manager?"
    choices: [npm, pnpm, yarn, bun]
    default: npm

steps:
  - name: Install pnpm
    run:
      - npm install -g pnpm
    when:
      pkg_manager: pnpm

  - name: Install rustup
    distro: arch
    tools: [build-tools]
    run:
      - pacman -Sy --noconfirm rustup
"#;
        let t = YamlTemplate::from_yaml(src).unwrap();
        assert_eq!(t.name, "nodejs");
        assert_eq!(t.packages["arch"], vec!["curl", "git", "nodejs", "npm"]);
        assert_eq!(t.env["NODE_ENV"], "development");
        assert_eq!(t.options[0].id, "pkg_manager");
        assert_eq!(t.options[0].default, Some("npm".to_string()));
        assert_eq!(t.steps[0].run, vec!["npm install -g pnpm"]);
        assert_eq!(t.steps[0].when["pkg_manager"], "pnpm");
        assert_eq!(t.steps[1].distro, Some("arch".to_string()));
    }

    #[test]
    fn run_field_accepts_single_string() {
        let src = r#"
schema: dxon/v1
name: test
steps:
  - name: single cmd
    run: "echo hello"
"#;
        let t = YamlTemplate::from_yaml(src).unwrap();
        assert_eq!(t.steps[0].run, vec!["echo hello"]);
    }

    #[test]
    fn into_dx_template_maps_fields_correctly() {
        let src = r#"
schema: dxon/v1
name: myenv
description: My env

packages:
  arch: [git]
  debian: [git]

env:
  FOO: bar

options:
  - id: pm
    prompt: "Which?"
    choices: [npm, yarn]
    default: npm

steps:
  - name: Step one
    tools: [nodejs]
    run:
      - node --version
    when:
      pm: npm
"#;
        let yaml = YamlTemplate::from_yaml(src).unwrap();
        let dx = yaml.into_dx_template();
        assert_eq!(dx.meta.name, "myenv");
        assert_eq!(dx.base.packages_by_distro["arch"], vec!["git"]);
        assert!(dx.base.packages.is_empty());
        assert_eq!(dx.runtime.env["FOO"], "bar");
        assert_eq!(dx.prompts[0].id, "pm");
        assert_eq!(dx.prompts[0].options, vec!["npm", "yarn"]);
        assert_eq!(dx.steps[0].tools, vec!["nodejs"]);
        assert_eq!(dx.steps[0].commands, vec!["node --version"]);
        assert_eq!(dx.steps[0].when["pm"], "npm");
    }

    #[test]
    fn into_dx_template_maps_base_distro() {
        let src = r#"
schema: dxon/v1
name: myenv
base: debian
"#;
        let yaml = YamlTemplate::from_yaml(src).unwrap();
        let dx = yaml.into_dx_template();
        assert_eq!(dx.base.distro, Some("debian".to_string()));
        assert_eq!(dx.pinned_distro(), Some("debian"));
    }
}
