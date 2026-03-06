pub mod builtin;
pub mod registry;
pub mod remote;
pub mod spec;

use anyhow::Result;

use crate::error::DxonError;
use spec::DxTemplate;

pub fn resolve(template_ref: &str) -> Result<DxTemplate> {
    // 1. Remote URL → HTTP fetch
    if remote::is_url(template_ref) {
        return remote::fetch(template_ref);
    }

    // 2. Local file path → read from disk
    if std::path::Path::new(template_ref).exists() {
        let src = std::fs::read_to_string(template_ref)?;
        return DxTemplate::from_toml(&src)
            .map_err(|e| DxonError::InvalidTemplate(format!("{template_ref}: {e}")).into());
    }

    // 3. Registry name → user cache first, then compiled-in builtins
    if let Some(tmpl) = registry::local_template(template_ref) {
        return Ok(tmpl);
    }

    builtin::get(template_ref)
        .ok_or_else(|| DxonError::TemplateNotFound(template_ref.to_string()).into())
}
