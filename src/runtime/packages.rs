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
