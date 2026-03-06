# Templates

Templates are YAML files that describe a complete development environment. They tell dXon which packages to install, which setup commands to run, and optionally which questions to ask you before creating the container.

## Template format

Templates use the `dxon/v1` schema and are standard YAML files with a `.yaml` or `.yml` extension.

A minimal template looks like this:

```yaml
schema: dxon/v1
name: myenv
description: My development environment

base: arch

packages:
  arch:
    - git
    - curl
    - nodejs
    - npm

steps:
  - name: Enable corepack
    run: corepack enable
```

## What a template contains

- **`base`** — the suggested base distro for the container (`arch`, `debian`, `alpine`)
- **`packages`** — distribution-specific packages installed before any steps run
- **`options`** — interactive prompts shown to the user before container creation
- **`steps`** — ordered setup commands run inside the container after packages are installed
- **`env`** — environment variables set inside the container
- **`run`** — commands run after all steps complete

## Using templates

### From the registry

Reference a template by name and dXon fetches it from the official registry:

```sh
dxon create mynode --template nodejs
dxon create myrust --template rust --distro arch
```

### From a local file

```sh
dxon create myenv --template ./myenv.yaml
```

### From a URL

```sh
dxon create myenv --template https://example.com/templates/myenv.yaml
```

Templates from sources other than the official registry will show a [trust warning](security.md) before proceeding.

## Official templates

The official registry includes templates for common environments:

| Name | Description |
|---|---|
| `nodejs` | Node.js with optional package manager (npm, pnpm, yarn, bun) |
| `python` | Python 3 with pip |
| `rust` | Rust via rustup with clippy and rustfmt |
| `go` | Go development environment |
| `cpp` | C/C++ with build tools and optional cmake/ninja |

Pull the full list with:

```sh
dxon template list
```

## Writing your own template

1. Create a file with a `.yaml` extension
2. Set `schema: dxon/v1` and give it a `name`
3. Define `packages`, `options`, and `steps` as needed
4. Test locally: `dxon create test --template ./mytemplate.yaml`

The [Template Specification](template-spec.md) documents every field in detail.
