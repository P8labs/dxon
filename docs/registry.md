# Registry

The dXon template registry is a public collection of community-maintained environment templates. It lives at:

**[https://github.com/P8labs/dxon-registry](https://github.com/P8labs/dxon-registry)**

## Available templates

The registry currently includes:

| Name | Description |
|---|---|
| `nodejs` | Node.js with choice of npm, pnpm, yarn, or bun |
| `python` | Python 3 with pip |
| `rust` | Rust via rustup with clippy and rustfmt |
| `go` | Go development environment |
| `cpp` | C/C++ with build tools, optional cmake/ninja |

List available templates directly from the CLI:

```sh
dxon template list
```

Search for a specific template:

```sh
dxon template search rust
```

## How registry resolution works

The registry uses an index file (`registry.json`) that maps template names to their locations. When you run:

```sh
dxon create myenv --template rust
```

dXon:

1. Fetches the registry index from the configured registry URL
2. Looks up the `rust` entry in the index
3. Downloads the template YAML file
4. Validates the template against the `dxon/v1` schema
5. Proceeds with container creation

Templates from the official registry are considered **trusted** and do not produce a security warning.

## Custom registry URL

You can point dXon at a different registry by changing the registry URL in your [configuration](configuration.md). The registry must expose a `registry.json` index with the same structure as the official one.

## Loading templates from a URL

You can load any template directly from a URL without publishing it to the registry:

```sh
dxon create myenv --template https://example.com/templates/myenv.yaml
```

Templates loaded this way are **not** considered trusted and will trigger a [trust warning](security.md) before the container is created.

## Loading templates from a local file

```sh
dxon create myenv --template ./myenv.yaml
```

Local file templates also trigger the trust warning. Use `--trust` (or `-y`) to skip it:

```sh
dxon create myenv --template ./myenv.yaml --trust
```

## Contributing templates

To add a template to the official registry, open a pull request on [P8labs/dxon-registry](https://github.com/P8labs/dxon-registry). Templates should:

- Follow the `dxon/v1` schema
- Work on at least one supported distribution
- Include a brief `description` field
