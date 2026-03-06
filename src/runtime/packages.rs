pub fn translate(logical: &str, distro: &str) -> Vec<String> {
    match (logical, distro) {
        ("c-compiler", _) => vec!["gcc".into()],
        ("cpp-compiler", "arch") => vec!["gcc".into()],
        ("cpp-compiler", _) => vec!["g++".into()],
        ("build-tools", "arch") => vec!["base-devel".into()],
        ("build-tools", "debian") => vec!["build-essential".into()],
        ("build-tools", "alpine") => vec!["build-base".into()],
        ("cmake", _) => vec!["cmake".into()],
        ("ninja", "debian") => vec!["ninja-build".into()],
        ("ninja", _) => vec!["ninja".into()],
        ("debugger", _) => vec!["gdb".into()],
        ("valgrind", _) => vec!["valgrind".into()],
        ("clang", _) => vec!["clang".into()],
        ("llvm", _) => vec!["llvm".into()],
        ("nodejs", _) => vec!["nodejs".into()],
        ("npm", _) => vec!["npm".into()],
        ("go", "debian") => vec!["golang-go".into()],
        ("go", _) => vec!["go".into()],
        ("python3", "arch") => vec!["python".into()],
        ("python3", _) => vec!["python3".into()],
        ("pip", "arch") => vec!["python-pip".into()],
        ("pip", "debian") => vec!["python3-pip".into(), "python3-venv".into()],
        ("pip", "alpine") => vec!["py3-pip".into()],
        ("docker", "debian") => vec!["docker.io".into()],
        ("docker", _) => vec!["docker".into()],
        ("docker-compose", _) => vec!["docker-compose".into()],
        ("curl", _) => vec!["curl".into()],
        ("git", _) => vec!["git".into()],
        ("wget", _) => vec!["wget".into()],
        ("ca-certificates", _) => vec!["ca-certificates".into()],
        (other, _) => vec![other.into()],
    }
}

pub fn translate_list(inputs: &[String], distro: &str) -> Vec<String> {
    inputs
        .iter()
        .flat_map(|p| translate(p.as_str(), distro))
        .collect()
}

pub fn fallback(package: &str, distro: &str) -> Option<Vec<String>> {
    match (package, distro) {
        ("g++", "arch") => Some(vec!["gcc".into()]),
        ("golang-go", "debian") => Some(vec!["golang".into()]),
        ("ninja-build", "debian") => Some(vec!["ninja".into()]),
        ("docker.io", "debian") => Some(vec!["docker-ce".into()]),
        ("valgrind", "alpine") => None,
        _ => None,
    }
}

pub fn pkg_install_cmd(distro: &str, packages: &[String]) -> String {
    let pkgs = packages.join(" ");
    match distro {
        "arch" => format!("pacman -Sy --noconfirm {pkgs}"),
        "alpine" => format!("apk add --no-cache {pkgs}"),
        _ => format!("DEBIAN_FRONTEND=noninteractive apt-get install -y {pkgs}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpp_compiler_arch_maps_to_gcc() {
        assert_eq!(translate("cpp-compiler", "arch"), vec!["gcc"]);
    }

    #[test]
    fn cpp_compiler_debian_maps_to_gpp() {
        assert_eq!(translate("cpp-compiler", "debian"), vec!["g++"]);
    }

    #[test]
    fn cpp_compiler_alpine_maps_to_gpp() {
        assert_eq!(translate("cpp-compiler", "alpine"), vec!["g++"]);
    }

    #[test]
    fn c_compiler_maps_to_gcc_on_all_distros() {
        for distro in &["arch", "debian", "alpine"] {
            assert_eq!(translate("c-compiler", distro), vec!["gcc"]);
        }
    }

    #[test]
    fn build_tools_maps_per_distro() {
        assert_eq!(translate("build-tools", "arch"), vec!["base-devel"]);
        assert_eq!(translate("build-tools", "debian"), vec!["build-essential"]);
        assert_eq!(translate("build-tools", "alpine"), vec!["build-base"]);
    }

    #[test]
    fn ninja_debian_is_ninja_build_other_is_ninja() {
        assert_eq!(translate("ninja", "debian"), vec!["ninja-build"]);
        assert_eq!(translate("ninja", "arch"), vec!["ninja"]);
        assert_eq!(translate("ninja", "alpine"), vec!["ninja"]);
    }

    #[test]
    fn go_debian_is_golang_go_other_is_go() {
        assert_eq!(translate("go", "debian"), vec!["golang-go"]);
        assert_eq!(translate("go", "arch"), vec!["go"]);
        assert_eq!(translate("go", "alpine"), vec!["go"]);
    }

    #[test]
    fn python3_arch_is_python_other_is_python3() {
        assert_eq!(translate("python3", "arch"), vec!["python"]);
        assert_eq!(translate("python3", "debian"), vec!["python3"]);
        assert_eq!(translate("python3", "alpine"), vec!["python3"]);
    }

    #[test]
    fn pip_expands_differently_per_distro() {
        assert_eq!(translate("pip", "arch"), vec!["python-pip"]);
        assert_eq!(
            translate("pip", "debian"),
            vec!["python3-pip", "python3-venv"]
        );
        assert_eq!(translate("pip", "alpine"), vec!["py3-pip"]);
    }

    #[test]
    fn docker_debian_is_docker_io_other_is_docker() {
        assert_eq!(translate("docker", "debian"), vec!["docker.io"]);
        assert_eq!(translate("docker", "arch"), vec!["docker"]);
    }

    #[test]
    fn unknown_package_passes_through_unchanged() {
        assert_eq!(
            translate("some-custom-pkg", "arch"),
            vec!["some-custom-pkg"]
        );
        assert_eq!(translate("libfoo-dev", "debian"), vec!["libfoo-dev"]);
        assert_eq!(translate("my-tool", "alpine"), vec!["my-tool"]);
    }

    #[test]
    fn translate_list_empty_input_returns_empty() {
        assert!(translate_list(&[], "arch").is_empty());
    }

    #[test]
    fn translate_list_expands_multi_package_logical_name() {
        let result = translate_list(&["pip".to_string()], "debian");
        assert_eq!(result, vec!["python3-pip", "python3-venv"]);
    }

    #[test]
    fn translate_list_passthrough_for_concrete_packages() {
        let inputs = vec!["git".to_string(), "curl".to_string()];
        assert_eq!(translate_list(&inputs, "arch"), vec!["git", "curl"]);
    }

    #[test]
    fn translate_list_mixed_logical_and_concrete() {
        let inputs = vec!["cpp-compiler".to_string(), "git".to_string()];
        let result = translate_list(&inputs, "arch");
        assert_eq!(result, vec!["gcc", "git"]);
    }

    #[test]
    fn fallback_gpp_on_arch_gives_gcc() {
        assert_eq!(fallback("g++", "arch"), Some(vec!["gcc".to_string()]));
    }

    #[test]
    fn fallback_golang_go_on_debian_gives_golang() {
        assert_eq!(
            fallback("golang-go", "debian"),
            Some(vec!["golang".to_string()])
        );
    }

    #[test]
    fn fallback_ninja_build_on_debian_gives_ninja() {
        assert_eq!(
            fallback("ninja-build", "debian"),
            Some(vec!["ninja".to_string()])
        );
    }

    #[test]
    fn fallback_docker_io_on_debian_gives_docker_ce() {
        assert_eq!(
            fallback("docker.io", "debian"),
            Some(vec!["docker-ce".to_string()])
        );
    }

    #[test]
    fn fallback_valgrind_on_alpine_is_none() {
        assert_eq!(fallback("valgrind", "alpine"), None);
    }

    #[test]
    fn fallback_unknown_package_is_none() {
        assert_eq!(fallback("does-not-exist", "arch"), None);
    }

    #[test]
    fn pkg_install_cmd_arch_uses_pacman() {
        let pkgs = vec!["vim".to_string(), "git".to_string()];
        let cmd = pkg_install_cmd("arch", &pkgs);
        assert!(cmd.starts_with("pacman -Sy --noconfirm"));
        assert!(cmd.contains("vim"));
        assert!(cmd.contains("git"));
    }

    #[test]
    fn pkg_install_cmd_alpine_uses_apk() {
        let pkgs = vec!["curl".to_string()];
        let cmd = pkg_install_cmd("alpine", &pkgs);
        assert!(cmd.starts_with("apk add --no-cache"));
        assert!(cmd.contains("curl"));
    }

    #[test]
    fn pkg_install_cmd_debian_uses_apt_get_with_noninteractive() {
        let pkgs = vec!["build-essential".to_string()];
        let cmd = pkg_install_cmd("debian", &pkgs);
        assert!(cmd.contains("apt-get install -y"));
        assert!(cmd.contains("DEBIAN_FRONTEND=noninteractive"));
        assert!(cmd.contains("build-essential"));
    }

    #[test]
    fn pkg_install_cmd_unknown_distro_defaults_to_apt() {
        let pkgs = vec!["foo".to_string()];
        let cmd = pkg_install_cmd("ubuntu", &pkgs);
        assert!(cmd.contains("apt-get install"));
    }
}
