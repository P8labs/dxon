use anyhow::Result;

use crate::error::DxonError;
use crate::template::spec::DxTemplate;

pub fn fetch(url: &str) -> Result<DxTemplate> {
    let response = ureq::get(url)
        .call()
        .map_err(|e| DxonError::RemoteTemplateFetch {
            url: url.to_string(),
            reason: e.to_string(),
        })?;

    let body = response
        .into_string()
        .map_err(|e| DxonError::RemoteTemplateFetch {
            url: url.to_string(),
            reason: e.to_string(),
        })?;

    let ext = url_extension(url);

    match ext {
        "yaml" | "yml" => DxTemplate::from_yaml(&body).map_err(|e| {
            DxonError::InvalidTemplate(format!("remote template '{url}': {e}")).into()
        }),
        "dx" => {
            use colored::Colorize;
            eprintln!(
                "{} remote template '{}' uses the deprecated .dx format.\n  \
                 Ask the template author to migrate to YAML (dxon/v1).",
                "warning:".yellow().bold(),
                url
            );
            DxTemplate::from_toml(&body).map_err(|e| {
                DxonError::InvalidTemplate(format!("remote template '{url}' is invalid TOML: {e}"))
                    .into()
            })
        }
        _ => DxTemplate::from_yaml(&body)
            .or_else(|_| {
                DxTemplate::from_toml(&body).map_err(|e| {
                    DxonError::InvalidTemplate(format!(
                        "remote template '{url}' is not valid YAML or TOML: {e}"
                    ))
                })
            })
            .map_err(Into::into),
    }
}

pub fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

fn url_extension(url: &str) -> &str {
    let path = url.split('?').next().unwrap_or(url);
    path.rsplit('.').next().map(str::trim).unwrap_or("")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_url_detects_http_and_https() {
        assert!(is_url("http://example.com/t.yaml"));
        assert!(is_url("https://example.com/t.yaml"));
        assert!(!is_url("nodejs"));
        assert!(!is_url("./local.yaml"));
    }

    #[test]
    fn url_extension_extracts_yaml() {
        assert_eq!(url_extension("https://example.com/t.yaml"), "yaml");
        assert_eq!(url_extension("https://example.com/t.yml"), "yml");
        assert_eq!(url_extension("https://example.com/t.dx"), "dx");
    }

    #[test]
    fn url_extension_ignores_query_string() {
        assert_eq!(
            url_extension("https://example.com/t.yaml?token=abc"),
            "yaml"
        );
    }
}
