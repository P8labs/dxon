use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerMeta {
    pub name: String,
    pub distro: String,
    pub created_at: DateTime<Utc>,
    pub template: Option<String>,
    pub packages: Vec<String>,
    pub repo: Option<String>,
    pub rootfs_path: String,
    pub config: ContainerConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ContainerConfig {
    pub env: HashMap<String, String>,
    pub extra_args: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shell_config_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_user: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_uid: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub container_gid: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace_dir: Option<String>,
}

impl ContainerMeta {
    pub fn new(name: &str, distro: &str, rootfs_path: &str) -> Self {
        Self {
            name: name.to_string(),
            distro: distro.to_string(),
            created_at: Utc::now(),
            template: None,
            packages: Vec::new(),
            repo: None,
            rootfs_path: rootfs_path.to_string(),
            config: ContainerConfig::default(),
        }
    }
}
