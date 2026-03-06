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

    DxTemplate::from_toml(&body).map_err(|e| {
        DxonError::InvalidTemplate(format!("template from '{url}' is invalid TOML: {e}")).into()
    })
}

pub fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}
