# Using dXon with AI Agents

This page is written for AI coding agents, LLM-powered tools, and automated pipelines. It covers everything needed to drive dXon programmatically: command shapes, flags, exit codes, non-interactive patterns, and common task recipes.

A machine-readable condensed version of this information is also available at [`llm.txt`](https://raw.githubusercontent.com/P8labs/dxon/master/llm.txt) in the repository root.

---

## Key facts

| Item                 | Value                                                           |
| -------------------- | --------------------------------------------------------------- |
| Binary               | `dxon`                                                          |
| Supported OS         | Linux only (requires `systemd-nspawn`)                          |
| Container storage    | `~/.dxon/containers/`                                           |
| Config file          | `~/.config/dxon/config.toml` (TOML)                             |
| Exit code on success | `0`                                                             |
| Exit code on error   | `1` (message on stderr, prefixed `error:`)                      |
| Non-interactive flag | All `create` prompts are bypassed by supplying flags explicitly |

---

## Installation from an agent

```sh
# latest release
curl -sSfL https://raw.githubusercontent.com/P8labs/dxon/master/install.sh | sh

# pin a version
DXON_VERSION=v0.2.0 curl -sSfL https://raw.githubusercontent.com/P8labs/dxon/master/install.sh | sh

# custom install directory (no sudo needed)
DXON_INSTALL_DIR=~/.local/bin curl -sSfL https://raw.githubusercontent.com/P8labs/dxon/master/install.sh | sh

# upgrade an existing install
curl -sSfL https://raw.githubusercontent.com/P8labs/dxon/master/upgrade.sh | sh
```

Verify:

```sh
dxon --version
```

---

## Fully non-interactive container creation

Provide all flags explicitly to prevent any interactive prompt:

```sh
dxon create <name> \
  --distro  <arch|debian|alpine> \
  --template <name|path|url> \
  --shell   <bash|zsh|fish> \
  --trust
```

`--trust` (`-y`) skips the security confirmation for templates loaded from local paths or third-party URLs. Omit it when using official registry templates (they are trusted by default).

### Minimal headless example

```sh
dxon create ci-env --distro debian --template rust --trust
```

### With a repository clone

```sh
dxon create myproject \
  --distro arch \
  --template rust \
  --repo https://github.com/user/myproject \
  --shell bash \
  --trust
```

The repository is cloned into `/workspace` inside the container after setup.

### With extra packages (no template)

```sh
dxon create devbox \
  --distro arch \
  --packages git neovim tmux ripgrep fd \
  --shell zsh
```

---

## Running commands inside a container

Use `--` to pass a command directly without entering an interactive shell:

```sh
dxon enter <name> -- <command> [args...]
```

```sh
# build
dxon enter myproject -- cargo build --release

# test
dxon enter myproject -- cargo test

# arbitrary shell pipeline
dxon enter myproject -- bash -c "npm install && npm run build"

# run in a subdirectory of /workspace
dxon enter myproject/src -- ls -la
```

The `dxon enter` command exits with the same exit code as the command you ran, so it integrates cleanly with CI pipelines.

---

## Inspecting containers

```sh
# list all containers (name, distro, creation date)
dxon list

# detailed info for one container
dxon info <name>
```

`dxon info` prints: distro, template used, shell, shell-config mode, creation timestamp, environment variables, and the on-disk path.

---

## Deleting containers

```sh
# interactive (asks for confirmation)
dxon delete <name>

# non-interactive (no prompt)
dxon delete <name> --force
```

Deletion is permanent and irreversible.

---

## Template management

```sh
# list all templates in the configured registry
dxon template list

# search by keyword
dxon template search nodejs

# refresh the cached registry index
dxon template refresh
```

### Official templates

| Name     | Description                                                    |
| -------- | -------------------------------------------------------------- |
| `nodejs` | Node.js; prompts for package manager (npm / pnpm / yarn / bun) |
| `python` | Python 3 + pip                                                 |
| `rust`   | Rust via rustup + clippy + rustfmt                             |
| `go`     | Go development environment                                     |
| `cpp`    | C/C++ build tools + optional cmake / ninja                     |

---

## Configuration

Read current config:

```sh
dxon config show
```

Set a value:

```sh
dxon config set <key> <value>
```

| Key                 | Default                    | Description                                    |
| ------------------- | -------------------------- | ---------------------------------------------- |
| `containers_dir`    | `~/.dxon/containers`       | Where container rootfs trees are stored        |
| `registry_url`      | Official registry JSON URL | URL of the registry index                      |
| `default_distro`    | _(none)_                   | Pre-selects distro in interactive prompt       |
| `default_shell`     | _(none)_                   | Pre-selects shell in interactive prompt        |
| `default_template`  | _(none)_                   | Pre-selects template in interactive prompt     |
| `copy_shell_config` | _(none)_                   | Pre-selects shell-config mode (`copy`\|`bind`) |
| `default_editor`    | _(none)_                   | Editor binary for `dxon open`                  |

Example — redirect container storage to a separate drive:

```sh
dxon config set containers_dir /data/dxon/containers
```

The `--dir` flag (or `DXON_DIR` env var) overrides `containers_dir` for a single invocation without changing the config file.

---

## Writing a template

Templates are YAML files using the `dxon/v1` schema. An agent can generate and pass them directly:

```yaml
schema: dxon/v1
name: myenv
description: Custom Node.js environment

base: arch

packages:
  arch: [git, curl, nodejs, npm]
  debian: [git, curl, ca-certificates, nodejs, npm]
  alpine: [git, curl, ca-certificates, nodejs, npm]

env:
  NODE_ENV: development

options:
  - id: pkg_manager
    prompt: "Which package manager?"
    choices: [npm, pnpm, yarn, bun]
    default: npm

steps:
  - name: Enable corepack
    run: corepack enable
    when:
      pkg_manager: npm

  - name: Install pnpm
    tools: [pnpm]
    when:
      pkg_manager: pnpm

run:
  - node --version
```

Pass the file to dXon:

```sh
dxon create myenv --template /path/to/myenv.yaml --trust
```

### Template field reference (quick)

| Field         | Type   | Notes                                                          |
| ------------- | ------ | -------------------------------------------------------------- |
| `schema`      | string | Must be `dxon/v1`                                              |
| `name`        | string | Short identifier, no spaces                                    |
| `description` | string | Optional one-line summary                                      |
| `base`        | string | Suggested distro: `arch` \| `debian` \| `alpine`               |
| `packages`    | map    | Per-distro raw package names, keys: `arch`, `debian`, `alpine` |
| `env`         | map    | Env vars set at container enter time                           |
| `options`     | list   | Interactive prompts (`id`, `prompt`, `choices`, `default`)     |
| `steps`       | list   | Ordered commands (`name`, `run`, `tools`, `distro`, `when`)    |
| `run`         | list   | Commands run after all steps                                   |

Logical `tools` names (resolved per distro automatically):
`git`, `curl`, `wget`, `make`, `cmake`, `ninja`, `gcc`, `clang`,
`python`, `pip`, `nodejs`, `npm`, `yarn`, `pnpm`, `bun`, `go`,
`rustup`, `cargo`, `docker`, `vim`, `neovim`, `tmux`, `zsh`, `fish`,
`htop`, `jq`, `unzip`.

---

## CI / automation recipes

### Build and test in a Debian container

```sh
#!/usr/bin/env sh
set -e

dxon create build-env --distro debian --template rust --trust
dxon enter build-env -- cargo test --release
dxon delete build-env --force
```

### Multi-step pipeline

```sh
#!/usr/bin/env sh
set -e

# provision
dxon create pipeline-env \
  --distro arch \
  --template nodejs \
  --repo https://github.com/user/app \
  --trust

# install deps
dxon enter pipeline-env -- bash -c "cd /workspace && npm ci"

# run tests
dxon enter pipeline-env -- bash -c "cd /workspace && npm test"

# build
dxon enter pipeline-env -- bash -c "cd /workspace && npm run build"

# cleanup
dxon delete pipeline-env --force
```

### Check if a container exists before acting

```sh
if dxon info myenv >/dev/null 2>&1; then
  echo "container exists"
else
  dxon create myenv --distro arch --template rust --trust
fi
```

### Use a custom registry

```sh
dxon config set registry_url https://your-host/registry.json
dxon template list
dxon create myenv --template mytemplate --trust
```

---

## Host dependencies

dXon needs these tools on the **host** machine:

| Tool             | Required for               | Install command                                                        |
| ---------------- | -------------------------- | ---------------------------------------------------------------------- |
| `systemd-nspawn` | Running any container      | `apt install systemd-container` / already in `systemd`                 |
| `pacstrap`       | Arch Linux containers      | `apt install arch-install-scripts` or `pacman -S arch-install-scripts` |
| `debootstrap`    | Debian / Ubuntu containers | `apt install debootstrap` / `pacman -S debootstrap`                    |
| `curl` or `wget` | Alpine containers          | `apt install curl` / `pacman -S curl`                                  |

dXon prints a clear diagnostic if a required tool is missing.

---

## Trust model for agents

Templates from the **official registry** are trusted automatically — no confirmation needed.

Templates from **local paths** or **third-party URLs** require confirmation unless `--trust` is passed.

```sh
# registry template — no --trust needed
dxon create env --distro arch --template rust

# local template — needs --trust to run non-interactively
dxon create env --distro arch --template ./custom.yaml --trust

# remote template — needs --trust
dxon create env --distro arch --template https://my.host/env.yaml --trust
```

---

## Useful references

| Resource                   | URL                                                            |
| -------------------------- | -------------------------------------------------------------- |
| Full documentation         | <https://dxon.p8labs.in>                                       |
| Machine-readable reference | <https://raw.githubusercontent.com/P8labs/dxon/master/llm.txt> |
| Source code                | <https://github.com/P8labs/dxon>                               |
| Template registry          | <https://github.com/P8labs/dxon-registry>                      |
| Issue tracker              | <https://github.com/P8labs/dxon/issues>                        |
