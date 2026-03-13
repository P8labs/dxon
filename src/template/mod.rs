pub mod registry;
pub mod remote;
pub mod spec;
pub mod yaml_spec;

use anyhow::Result;
use colored::Colorize;

use crate::error::DxonError;
use spec::DxTemplate;

#[derive(Debug, Clone)]
pub enum TemplateSource {
    Registry,
    RemoteUrl(String),
    LocalFile(String),
}

impl TemplateSource {
    pub fn is_trusted(&self) -> bool {
        matches!(self, TemplateSource::Registry)
    }

    pub fn label(&self) -> &str {
        match self {
            TemplateSource::Registry => "official registry",
            TemplateSource::RemoteUrl(url) => url.as_str(),
            TemplateSource::LocalFile(path) => path.as_str(),
        }
    }

    pub fn kind(&self) -> &str {
        match self {
            TemplateSource::Registry => "registry",
            TemplateSource::RemoteUrl(_) => "remote URL",
            TemplateSource::LocalFile(_) => "local file",
        }
    }
}

pub fn resolve(template_ref: &str, registry_url: &str) -> Result<(DxTemplate, TemplateSource)> {
    if remote::is_url(template_ref) {
        println!(
            "  {} loading template from remote URL {}",
            "→".cyan(),
            template_ref.dimmed()
        );
        let tmpl = remote::fetch(template_ref)?;
        return Ok((tmpl, TemplateSource::RemoteUrl(template_ref.to_string())));
    }

    let path = std::path::Path::new(template_ref);
    if path.exists() {
        println!(
            "  {} loading template from local file {}",
            "→".cyan(),
            template_ref.dimmed()
        );
        let src = std::fs::read_to_string(path)?;
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let tmpl = parse_by_extension(ext, &src, template_ref)?;
        return Ok((tmpl, TemplateSource::LocalFile(template_ref.to_string())));
    }

    for ext in &["yaml", "yml"] {
        let candidate = std::path::PathBuf::from(format!("{template_ref}.{ext}"));
        if candidate.exists() {
            let label = candidate.display().to_string();
            println!(
                "  {} loading template from local file {}",
                "→".cyan(),
                label.dimmed()
            );
            let src = std::fs::read_to_string(&candidate)?;
            let tmpl = DxTemplate::from_yaml(&src)
                .map_err(|e| DxonError::InvalidTemplate(format!("{}: {e}", candidate.display())))?;
            return Ok((tmpl, TemplateSource::LocalFile(label)));
        }
    }

    let tmpl = registry::load_by_name(template_ref, registry_url)
        .map_err(|_| DxonError::TemplateNotFound(template_ref.to_string()))?;
    Ok((tmpl, TemplateSource::Registry))
}

pub fn parse_by_extension(ext: &str, src: &str, label: &str) -> Result<DxTemplate> {
    match ext {
        "yaml" | "yml" => DxTemplate::from_yaml(src)
            .map_err(|e| DxonError::InvalidTemplate(format!("{label}: {e}")).into()),
        _ => DxTemplate::from_yaml(src)
            .or_else(|_| {
                DxTemplate::from_toml(src)
                    .map_err(|e| DxonError::InvalidTemplate(format!("{label}: {e}")))
            })
            .map_err(Into::into),
    }
}
