use thiserror::Error;

#[derive(Error, Debug)]
pub enum DxonError {
    #[error("container '{0}' already exists")]
    ContainerExists(String),

    #[error("container '{0}' not found")]
    ContainerNotFound(String),

    #[error("missing required tool: {tool}\n  hint: {hint}")]
    MissingTool { tool: String, hint: String },

    #[error("bootstrap failed for '{distro}': {reason}")]
    BootstrapFailed { distro: String, reason: String },

    #[error("template '{0}' not found")]
    TemplateNotFound(String),

    #[error("invalid template: {0}")]
    InvalidTemplate(String),

    #[error("failed to fetch template from '{url}': {reason}")]
    RemoteTemplateFetch { url: String, reason: String },

    #[error("git clone failed: {0}")]
    GitCloneFailed(String),

    #[error("unsupported distribution: '{0}' (supported: arch, debian, alpine)")]
    UnsupportedDistro(String),

    #[error("{0}")]
    Io(#[from] std::io::Error),
}
