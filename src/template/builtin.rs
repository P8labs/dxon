use crate::template::spec::DxTemplate;

pub const BUILTIN_NAMES: &[&str] = &["nodejs", "rust", "go", "cpp", "python"];

const NODEJS: &str = include_str!("../../registry/templates/nodejs.dx");
const RUST: &str   = include_str!("../../registry/templates/rust.dx");
const GO: &str     = include_str!("../../registry/templates/go.dx");
const CPP: &str    = include_str!("../../registry/templates/cpp.dx");
const PYTHON: &str = include_str!("../../registry/templates/python.dx");

pub fn get(name: &str) -> Option<DxTemplate> {
    let src = match name {
        "nodejs" => NODEJS,
        "rust"   => RUST,
        "go"     => GO,
        "cpp"    => CPP,
        "python" => PYTHON,
        _        => return None,
    };
    DxTemplate::from_toml(src).ok()
}

pub fn list_descriptions() -> Vec<(&'static str, &'static str)> {
    vec![
        ("nodejs", "Node.js - choose pnpm / yarn / bun / npm"),
        ("rust",   "Rust - rustup, cargo, clippy, rustfmt"),
        ("go",     "Go - Go toolchain"),
        ("cpp",    "C/C++ - gcc, cmake, ninja, gdb, clang"),
        ("python", "Python - choose pip / poetry / uv"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_builtin_templates_parse_without_error() {
        for name in BUILTIN_NAMES {
            let tmpl = get(name);
            assert!(tmpl.is_some(), "builtin template '{name}' failed to parse");
        }
    }

    #[test]
    fn unknown_template_name_returns_none() {
        assert!(get("doesnotexist").is_none());
        assert!(get("").is_none());
    }

    #[test]
    fn list_descriptions_covers_all_builtin_names() {
        let descs = list_descriptions();
        assert_eq!(descs.len(), BUILTIN_NAMES.len());
        for name in BUILTIN_NAMES {
            assert!(descs.iter().any(|(n, _)| n == name), "missing description for '{name}'");
        }
    }

    #[test]
    fn nodejs_template_has_pkg_manager_prompt_with_npm_default() {
        let t = get("nodejs").unwrap();
        let pm = t.prompts.iter().find(|p| p.id == "pkg_manager").expect("pkg_manager prompt");
        assert!(pm.options.contains(&"npm".to_string()));
        assert!(pm.options.contains(&"pnpm".to_string()));
        assert_eq!(pm.default, Some("npm".into()));
    }

    #[test]
    fn rust_template_has_distro_specific_steps_for_all_three_distros() {
        let t = get("rust").unwrap();
        let distros: Vec<&str> = t.steps.iter().filter_map(|s| s.distro.as_deref()).collect();
        assert!(distros.contains(&"arch"),   "rust template missing arch step");
        assert!(distros.contains(&"debian"), "rust template missing debian step");
        assert!(distros.contains(&"alpine"), "rust template missing alpine step");
    }

    #[test]
    fn cpp_template_uses_logical_tool_names_not_raw_package_names() {
        let t = get("cpp").unwrap();
        let all_tools: Vec<&str> = t
            .steps
            .iter()
            .flat_map(|s| s.tools.iter().map(String::as_str))
            .collect();
        assert!(all_tools.contains(&"cpp-compiler"), "missing cpp-compiler logical name");
        assert!(all_tools.contains(&"c-compiler"),   "missing c-compiler logical name");
        assert!(!all_tools.contains(&"g++"),         "should use logical name, not g++");
        assert!(!all_tools.contains(&"gcc"),         "should use logical name, not gcc");
    }

    #[test]
    fn python_template_has_env_manager_prompt() {
        let t = get("python").unwrap();
        let pm = t.prompts.iter().find(|p| p.id == "env_manager").expect("env_manager prompt");
        assert!(pm.options.contains(&"pip".to_string()));
        assert!(pm.options.contains(&"poetry".to_string()));
        assert!(pm.options.contains(&"uv".to_string()));
    }

    #[test]
    fn go_template_installs_go_toolchain() {
        let t = get("go").unwrap();
        let tools: Vec<&str> = t
            .steps
            .iter()
            .flat_map(|s| s.tools.iter().map(String::as_str))
            .collect();
        assert!(tools.contains(&"go"));
    }
}
