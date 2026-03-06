# dxon Template Specification

Templates let you describe a reproducible development environment as a `.dx` file. When you pass `--template <name|path|url>` to `dxon create`, dxon fetches and executes the template during container bootstrap.

`.dx` files are standard TOML. They are versioned, portable, and shareable.

---

## Sections at a glance

| Section     | Required | Purpose                                              |
|-------------|----------|------------------------------------------------------|
| `[meta]`    | yes      | Template identity (name, description, version)       |
| `[base]`    | no       | Base packages installed before any steps             |
| `[runtime]` | no       | Environment variables and post-install commands      |
| `[[prompts]]` | no     | Interactive questions shown during `dxon create`     |
| `[[steps]]` | no       | Ordered installation steps, optionally conditional   |

---

## `[meta]`

```toml
[meta]
name        = "mynodeapp"
description = "Node.js 20 + pnpm"
version     = "1.0.0"
author      = "you@example.com"
```

| Field         | Type   | Required | Description                          |
|---------------|--------|----------|--------------------------------------|
| `name`        | string | yes      | Unique template identifier           |
| `description` | string | no       | Human-readable summary               |
| `version`     | string | no       | Semantic version of this template    |
| `author`      | string | no       | Author name or email                 |

---

## `[base]`

Packages installed by the distro package manager before any `[[steps]]` run.
Useful for common prerequisites like `curl`, `git`, or `ca-certificates`.

```toml
[base]
distros  = ["arch", "debian"]   # restrict to these distros (empty = all)
packages = ["curl", "git", "wget", "ca-certificates"]
```

| Field      | Type            | Required | Description                                  |
|------------|-----------------|----------|----------------------------------------------|
| `distros`  | list of strings | no       | Allowed distros. Empty means all are allowed |
| `packages` | list of strings | no       | Packages to install via the distro PM        |

Package installation uses the correct command automatically:
- **arch** → `pacman -Sy --noconfirm`
- **debian** → `apt-get install -y`
- **alpine** → `apk add --no-cache`

---

## `[runtime]`

```toml
[runtime]
env      = { NODE_ENV = "development", GOPATH = "/root/go" }
commands = ["npm install -g npm@latest"]
```

| Field      | Type              | Required | Description                                           |
|------------|-------------------|----------|-------------------------------------------------------|
| `env`      | map string→string | no       | Environment variables injected into every nspawn call |
| `commands` | list of strings   | no       | Shell commands run after all `[[steps]]` complete     |

---

## `[[prompts]]`

Prompts present an interactive choice to the user during `dxon create`.
Each prompt produces an **answer** which can be referenced in `[[steps]]` via `[steps.when]`.

```toml
[[prompts]]
id       = "pkg_manager"
question = "Which package manager?"
options  = ["npm", "pnpm", "yarn", "bun"]
default  = "npm"
```

| Field      | Type            | Required | Description                                          |
|------------|-----------------|----------|------------------------------------------------------|
| `id`       | string          | yes      | Unique identifier, referenced in `[steps.when]`      |
| `question` | string          | yes      | Question shown to the user                           |
| `options`  | list of strings | yes      | Available choices (at least 2)                       |
| `default`  | string          | no       | Pre-selected option                                  |

Prompts are skipped when the user passes explicit flags. Future versions will
support `--answers key=value` for fully non-interactive execution.

---

## `[[steps]]`

Steps are the core of a template. Each step is an ordered list of shell commands
run inside the container with `systemd-nspawn`.

```toml
[[steps]]
name    = "Install Node.js"
distro  = "arch"
commands = ["pacman -Sy --noconfirm nodejs npm"]

[[steps]]
name    = "Install pnpm"
distro  = "arch"
commands = ["npm install -g pnpm"]

[steps.when]
pkg_manager = "pnpm"
```

| Field      | Type              | Required | Description                                               |
|------------|-------------------|----------|-----------------------------------------------------------|
| `name`     | string            | yes      | Human-readable step label shown during creation           |
| `distro`   | string            | no       | If set, step only runs for this distro                    |
| `commands` | list of strings   | yes      | Shell commands executed in order inside the container     |
| `[steps.when]` | map string→string | no  | Map of `prompt_id → expected_answer` for conditional runs |

A step runs when **all** of these are true:
1. `distro` matches the container's distro (or is unset)
2. Every `when` entry matches the corresponding prompt answer

Commands run as root inside the container. Each command is passed to `/bin/sh -c`.

---

## Version pinning

Pin specific software versions by encoding them in commands:

```toml
[[steps]]
name     = "Install Node.js 20"
distro   = "debian"
commands = [
  "apt-get update -y",
  "apt-get install -y curl",
  "curl -fsSL https://deb.nodesource.com/setup_20.x | bash -",
  "apt-get install -y nodejs",
]
```

For tools installed via rustup, pip, or similar:

```toml
[runtime]
commands = [
  "rustup install 1.78.0",
  "rustup default 1.78.0",
]
```

---

## Full example

```toml
[meta]
name        = "mystack"
description = "Node 20 + Rust + Docker"
version     = "1.0.0"

[base]
packages = ["curl", "git", "ca-certificates"]

[runtime]
env      = { NODE_ENV = "development" }
commands = ["node --version", "cargo --version"]

[[prompts]]
id       = "pkg_manager"
question = "Node.js package manager?"
options  = ["npm", "pnpm", "bun"]
default  = "npm"

[[prompts]]
id       = "docker"
question = "Install Docker inside the container?"
options  = ["no", "yes"]
default  = "no"

# ── Node.js (arch) ──────────────────────────────────────────────
[[steps]]
name     = "Install Node.js"
distro   = "arch"
commands = ["pacman -Sy --noconfirm nodejs npm"]

[[steps]]
name     = "Install pnpm"
distro   = "arch"
commands = ["npm install -g pnpm"]
[steps.when]
pkg_manager = "pnpm"

# ── Node.js (debian) ────────────────────────────────────────────
[[steps]]
name     = "Install Node.js"
distro   = "debian"
commands = ["apt-get update -y", "apt-get install -y nodejs npm"]

# ── Rust (all distros) ──────────────────────────────────────────
[[steps]]
name     = "Install Rust (arch)"
distro   = "arch"
commands = ["pacman -Sy --noconfirm rustup", "rustup default stable"]

[[steps]]
name     = "Install Rust (debian)"
distro   = "debian"
commands = [
  "apt-get install -y curl build-essential",
  "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y",
]

# ── Docker (conditional) ────────────────────────────────────────
[[steps]]
name     = "Install Docker (arch)"
distro   = "arch"
commands = ["pacman -Sy --noconfirm docker docker-compose"]
[steps.when]
docker = "yes"
```

---

## Using a template

```bash
# built-in
dxon create myenv --template nodejs

# local file
dxon create myenv --template ./my-stack.dx

# remote URL
dxon create myenv --template https://example.com/templates/mystack.dx
```

---

## Distribution support matrix

| Distro  | Bootstrap tool | Package manager   |
|---------|----------------|-------------------|
| arch    | `pacstrap`     | `pacman`          |
| debian  | `debootstrap`  | `apt-get`         |
| alpine  | curl + tar     | `apk`             |
