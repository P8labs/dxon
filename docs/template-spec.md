# Template Specification

This page documents the complete `dxon/v1` template format. Every field a template can contain is described here.

## File format

Templates are YAML files (`.yaml` or `.yml`) and must begin with the schema identifier:

```yaml
schema: dxon/v1
name: myenv
```

Every field is optional except `schema` and `name`.

## Top-level fields

| Field | Type | Required | Description |
|---|---|---|---|
| `schema` | `string` | **yes** | Must be `dxon/v1` |
| `name` | `string` | **yes** | Short identifier shown in the registry, no spaces |
| `description` | `string` | no | One-line description of the environment |
| `base` | `string` | no | Pinned base distribution for this template: `arch`, `debian`, or `alpine` |
| `packages` | `map<distro, list<string>>` | no | Per-distro package lists installed before steps run |
| `env` | `map<string, string>` | no | Environment variables set inside the container |
| `run` | `list<string>` | no | Commands run after all steps complete |
| `options` | `list<Option>` | no | Interactive prompts shown before container creation |
| `steps` | `list<Step>` | no | Ordered setup steps executed inside the container |

---

## `packages` — per-distro package lists

Maps distribution names to raw package lists. Lets a single template work across Arch, Debian, and Alpine without relying on logical name translation.

```yaml
packages:
  arch:   [curl, git, nodejs, npm]
  debian: [curl, git, ca-certificates, nodejs, npm]
  alpine: [curl, git, ca-certificates, nodejs, npm]
```

dXon selects the list matching the chosen distribution and installs those packages before running any steps. If no entry exists for the chosen distribution, the field is silently skipped.

Supported keys: `arch`, `debian`, `alpine`.

If your template is intentionally distro-specific, set `base` and only define that distro key in `packages`.

```yaml
base: debian
packages:
  debian: [curl, git, ca-certificates]
```

When `base` is set, `dxon create --template ...` automatically uses that distro.

---

## `env` — environment variables

Variables set here apply during step execution and are preserved in the container metadata. They are applied every time you run `dxon enter`.

```yaml
env:
  NODE_ENV: development
  GOPATH: /root/go
  CARGO_HOME: /root/.cargo
```

---

## `run` — post-step commands

Commands in `run` execute inside the container after all steps have completed. Useful for final configuration that depends on everything installed by steps.

```yaml
run:
  - . /root/.cargo/env && rustup component add clippy rustfmt
```

---

## `options` — interactive prompts

Options present the user with a choice before the container is created. The selected values can then gate individual steps via step `when` conditions.

### Option fields

| Field | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | **yes** | Identifier referenced by step `when` conditions |
| `prompt` | `string` | **yes** | Question displayed to the user |
| `choices` | `list<string>` | **yes** | Valid answers; must be non-empty |
| `default` | `string` | no | Pre-selected answer; must appear in `choices` |

```yaml
options:
  - id: pkg_manager
    prompt: "Which package manager would you like to use?"
    choices: [npm, pnpm, yarn, bun]
    default: npm

  - id: docker
    prompt: "Install Docker CLI tooling inside the container?"
    choices: ["no", "yes"]
    default: "no"
```

---

## `steps` — setup sequence

Steps execute inside the container in order. Each step can install packages using logical tool names, run shell commands, or both.

### Step fields

| Field | Type | Required | Description |
|---|---|---|---|
| `name` | `string` | **yes** | Display label shown in CLI output |
| `distro` | `string` | no | Restrict step to a single distro: `arch`, `debian`, or `alpine` |
| `tools` | `list<string>` | no | Logical tool names resolved to distro-specific packages |
| `run` | `string` or `list<string>` | no | Shell commands executed inside the container |
| `when` | `map<option_id, value>` | no | Run only when all listed option conditions match |

### `tools` — logical tool names

Logical names are translated to the correct distro package automatically. Using them means your template works across all supported distributions without per-distro overrides.

| Logical name | arch | debian | alpine |
|---|---|---|---|
| `build-tools` | `base-devel` | `build-essential` | `build-base` |
| `cpp-compiler` | `gcc` | `g++` | `g++` |
| `c-compiler` | `gcc` | `gcc` | `gcc` |
| `ninja` | `ninja` | `ninja-build` | `ninja` |
| `go` | `go` | `golang-go` | `go` |
| `python3` | `python` | `python3` | `python3` |
| `pip` | `python-pip` | `python3-pip` | `py3-pip` |
| `docker` | `docker` | `docker.io` | `docker` |

### `run` — inline commands

`run` accepts a single string or a list of strings:

```yaml
steps:
  - name: Install pnpm
    run: npm install -g pnpm

  - name: Configure Rust
    run:
      - . /root/.cargo/env && rustup default stable
      - rustup component add clippy rustfmt
```

### `when` — conditional execution

A step runs only when every entry in `when` matches the user's answer for that option. Steps without a `when` map always run.

```yaml
steps:
  - name: Install pnpm
    run: npm install -g pnpm
    when:
      pkg_manager: pnpm
```

Multiple conditions must all match:

```yaml
steps:
  - name: Setup with Docker
    run: echo "both matched"
    when:
      pkg_manager: pnpm
      docker: "yes"
```

### `distro` guard

Restricts a step to a single distribution. Useful when the setup procedure differs between distros:

```yaml
steps:
  - name: Install Rust toolchain
    distro: arch
    run:
      - pacman -Sy --noconfirm rustup
      - rustup default stable

  - name: Install Rust toolchain
    distro: debian
    tools: [build-tools]
    run:
      - curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --no-modify-path
      - . /root/.cargo/env && rustup default stable
```

---

## Validation

dXon validates every template before container creation begins. Errors print clearly:

```
error: invalid template: options[0] 'pkg_manager': default 'bun' is not in choices ["npm", "pnpm", "yarn"]
```

Validation rules:

- `schema` must equal `dxon/v1`
- `name` must be non-empty
- Each option must have a non-empty `id` and at least one choice
- If `default` is set, it must appear in `choices`
- Each step must have a non-empty `name`

---

## Complete example

```yaml
schema: dxon/v1
name: nodejs
description: Node.js development environment

packages:
  arch:   [curl, git, nodejs, npm]
  debian: [curl, git, ca-certificates, nodejs, npm]
  alpine: [curl, git, ca-certificates, nodejs, npm]

env:
  NODE_ENV: development

options:
  - id: pkg_manager
    prompt: "Which package manager would you like to use?"
    choices: [npm, pnpm, yarn, bun]
    default: npm

  - id: docker
    prompt: "Install Docker CLI tooling inside the container?"
    choices: ["no", "yes"]
    default: "no"

steps:
  - name: Install pnpm
    run: npm install -g pnpm
    when:
      pkg_manager: pnpm

  - name: Install yarn
    run: npm install -g yarn
    when:
      pkg_manager: yarn

  - name: Install bun
    run: npm install -g bun
    when:
      pkg_manager: bun

  - name: Install Docker
    tools: [docker]
    when:
      docker: "yes"
```
