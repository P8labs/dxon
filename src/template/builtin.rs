pub const BUILTIN_NAMES: &[&str] = &["nodejs", "rust", "go", "cpp", "python"];

pub fn list_descriptions() -> Vec<(&'static str, &'static str)> {
    vec![
        ("nodejs", "Node.js - choose pnpm / yarn / bun / npm"),
        ("rust", "Rust - rustup, cargo, clippy, rustfmt"),
        ("go", "Go - Go toolchain"),
        ("cpp", "C/C++ - gcc, cmake, ninja, gdb, clang"),
        ("python", "Python - choose pip / poetry / uv"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_descriptions_covers_all_builtin_names() {
        let descs = list_descriptions();
        assert_eq!(descs.len(), BUILTIN_NAMES.len());
        for name in BUILTIN_NAMES {
            assert!(
                descs.iter().any(|(n, _)| n == name),
                "missing description for '{name}'"
            );
        }
    }

    #[test]
    fn builtin_names_are_non_empty() {
        assert!(!BUILTIN_NAMES.is_empty());
        for name in BUILTIN_NAMES {
            assert!(!name.is_empty(), "builtin name must not be empty");
        }
    }

    #[test]
    fn descriptions_are_non_empty() {
        for (_name, desc) in list_descriptions() {
            assert!(!desc.is_empty(), "description must not be empty");
        }
    }
}
