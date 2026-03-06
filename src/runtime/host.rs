use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct HostInfo {
    pub pretty_name: String,
}

impl HostInfo {
    pub fn detect() -> Self {
        let fields = parse_os_release();
        let id = fields.get("ID").cloned().unwrap_or_default().to_lowercase();
        let pretty_name = fields.get("PRETTY_NAME").cloned().unwrap_or(id);
        Self { pretty_name }
    }

    pub fn bootstrap_tool_for(&self, target_distro: &str) -> (&'static str, &'static str) {
        match target_distro {
            "arch" => ("pacstrap", "pacman -S arch-install-scripts"),
            "debian" => ("debootstrap", "apt-get install debootstrap"),
            _ => ("curl", "your-package-manager install curl"),
        }
    }
}

fn parse_os_release() -> HashMap<String, String> {
    let content = std::fs::read_to_string("/etc/os-release").unwrap_or_default();
    content
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(k, v)| (k.to_string(), v.trim_matches('"').to_string()))
        .collect()
}
