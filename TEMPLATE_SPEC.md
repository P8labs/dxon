# dxon Template Specification

dxon templates are YAML files that describe a complete development environment.
They define which packages to install, which questions to ask the user, and which
setup commands to run inside the container. Templates can be stored locally,
hosted at a URL, or published to the dxon registry.

## File format

Templates use `.yaml` or `.yml` file extensions and must conform to the
`dxon/v1` schema. Every field is optional except `schema` and `name`.

```yaml
schema: dxon/v1   # required — identifies the template format version
name: myenv       # required — short identifier (no spaces)
description: My development environment
```

## Top-level fields

| Field | Type | Required | Description |
|---|---|---|---|
| `schema` | `string` | **yes** | Must be `dxon/v1` |
| `name` | `string` | **yes** | Short identifier, shown in `dxon registry list` |
| `description` | `string` | no | One-line description of the environment |
| `base` | `string` | no | Suggested base distribution (`arch`, `debian`, `alpine`) |
| `packages` | `map<distro, list<string>>` | no | Per-distro raw package names installed before steps |
| `env` | `map<string, string>` | no | Environment variables set inside the container |
| `run` | `list<string>` | no | Commands run after all steps complete |
| `options` | `list<Option>` | no | Interactive prompts shown before container creation |
| `steps` | `list<Step>` | no | Ordered setup steps executed inside the container |


## `packages` — per-distro package lists

The `packages` field maps distribution names to raw package lists. This allows
a single template to work across Arch, Debian, and Alpine without relying on
logical name translation.

```yaml
packages:
  arch:   [curl, git, nodejs, npm]
  debian: [curl, git, ca-certificates, nodejs, npm]
  alpine: [curl, git, ca-certificates, nodejs, npm]
```

When a container is created, dxon selects the list for the chosen distribution
and installs those packages before running any steps. If no entry exists for the
chosen distribution the field is skipped silently.

Supported distribution keys: `arch`, `debian`, `alpine`.


## `env` — environment variables

Environment variables defined here are set during step execution and preserved
in the container metadata. They are applied when you later run `dxon enter`.

```yaml
env:
  NODE_ENV: development
  GOPATH: /root/go
  CARGO_HOME: /root/.cargo
```


## `run` — post-step commands

Commands in `run` execute inside the container after all steps have completed.
Use this for final configuration that depends on the installed toolchain.

```yaml
run:
  - . /root/.cargo/env && rustup component add clippy rustfmt
```


## `options` — interactive prompts

Options present the user with a choice before the container is created. The
selected values control conditional step execution via the `when` map in each
step.

### Option fields

| Field | Type | Required | Description |
|---|---|---|---|
| `id` | `string` | **yes** | Identifier used in step `when` conditions |
| `prompt` | `string` | **yes** | Question displayed to the user |
| `choices` | `list<string>` | **yes** | Valid answers; must be non-empty |
| `default` | `string` | no | Pre-selected answer (must be in `choices`) |

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


## `steps` — setup sequence

Steps are executed inside the container in the order they are defined.
Each step can install packages using logical tool names, run shell commands,
or both.

### Step fields

| Field | Type | Required | Description |
|---|---|---|---|
| `name` | `string` | **yes** | Display label shown in CLI output |
| `distro` | `string` | no | Restrict step to one distro (`arch`, `debian`, `alpine`) |
| `tools` | `list<string>` | no | Logical tool names resolved to distro-specific packages |
| `run` | `string` or `list<string>` | no | Shell commands executed inside the container |
| `when` | `map<option_id, value>` | no | Run only when all option conditions match |

### `tools` — logical tool names

Tool names are translated to the correct package name for the chosen
distribution automatically. Using logical names means your template works
across all distros without manual per-distro overrides.

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

The `run` field accepts either a single command string or a list of strings.

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

A step executes only when every entry in `when` matches the user's answer for
that option. Steps without a `when` map always run.

```yaml
options:
  - id: pkg_manager
    prompt: "Package manager?"
    choices: [npm, pnpm, yarn]
    default: npm

steps:
  - name: Install pnpm
    run: npm install -g pnpm
    when:
      pkg_manager: pnpm   # only runs when the user chose pnpm
```

Multiple conditions must all match:

```yaml
steps:
  - name: Conditional step
    run: echo "both matched"
    when:
      pkg_manager: pnpm
      docker: "yes"
```

### `distro` guard

A `distro` guard restricts a step to a single distribution. This is useful when
the setup procedure differs between distributions.

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


## Validation

dxon validates every template before container creation begins. If a required
field is missing or a value is invalid the CLI prints a clear error:

```
error: invalid template: options[0] 'pkg_manager': default 'bun' is not in choices ["npm", "pnpm", "yarn"]
```

Validation checks:

- `schema` must equal `dxon/v1`
- `name` must be non-empty
- Each option must have a non-empty `id` and at least one choice
- If `default` is set it must appear in `choices`
- Each step must have a non-empty `name`


## Compatibility with `.dx` templates

dxon still loads legacy `.dx` (TOML) templates for backwards compatibility. When
a `.dx` file is detected the CLI prints a deprecation warning:

```
warning: template 'myenv' uses the deprecated .dx (TOML) format.
  Migrate to YAML (dxon/v1) — see TEMPLATE_SPEC.md for details.
```

Support for `.dx` templates will be removed in a future release. To migrate,
convert the TOML fields to the equivalent YAML fields described in this document.

### `.dx` to YAML field mapping

| `.dx` TOML | YAML (dxon/v1) |
|---|---|
| `[meta] name` | `name` |
| `[meta] description` | `description` |
| `[base] packages` | `packages` (per-distro raw names preferred) |
| `[runtime] env` | `env` |
| `[runtime] commands` | `run` |
| `[[prompts]]` id/question/options/default | `options` id/prompt/choices/default |
| `[[steps]]` name/distro/tools/commands/when | `steps` name/distro/tools/run/when |


## Example templates

### Node.js development environment

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
    run:
      - npm install -g pnpm
    when:
      pkg_manager: pnpm

  - name: Install yarn
    run:
      - npm install -g yarn
    when:
      pkg_manager: yarn

  - name: Install bun
    run:
      - npm install -g bun
    when:
      pkg_manager: bun

  - name: Install Docker
    tools: [docker, docker-compose]
    when:
      docker: "yes"
```


## Using templates

### From the registry

```sh
dxon create mynode --template nodejs
dxon create myrust --template nodejs --distro arch
```

### From a local file

```sh
dxon create myenv --template ./myenv.yaml
```

### From a URL

```sh
dxon create myenv --template https://example.com/templates/myenv.yaml
```

The CLI downloads and validates the template before creating the container.


## Creating your own template

1. Create a file with a `.yaml` or `.yml` extension.
2. Set `schema: dxon/v1` and give it a `name`.
3. Define `packages`, `options`, and `steps` as needed.
4. Test it locally: `dxon create test --template ./mytemplate.yaml`

Templates are plain YAML and work with any editor that has YAML support.
