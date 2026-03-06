# Usage

## Creating a container

The main command is `dxon create`. At minimum you provide a name for the container:

```sh
dxon create myenv
```

This starts an interactive prompt where you choose the base distro and optionally a template.

### Non-interactive creation

Use flags to skip the prompts:

```sh
dxon create myenv --distro arch
dxon create myenv --distro debian --template python
dxon create myenv --distro alpine --template nodejs
```

**Flags:**

| Flag | Description |
|---|---|
| `--distro <name>` | Base distribution: `arch`, `debian`, or `alpine` |
| `--template <name>` | Template name from the registry, a local file path, or a URL |
| `--repo <url>` | Clone a git repository into the container after setup |
| `--trust`, `-y` | Skip confirmation prompts (including trust warnings for untrusted templates) |

### Example: clone a repo into a container

```sh
dxon create myproject --distro arch --template rust --repo https://github.com/you/myproject
```

The repository is cloned into `/root/` inside the container after setup completes.

## Entering a container

```sh
dxon enter myenv
```

This drops you into an interactive shell inside the container. The container environment matches what the template set up, including any environment variables defined in the template.

## Listing containers

```sh
dxon list
```

Prints all containers with their name, base distro, and creation date.

## Inspecting a container

```sh
dxon info myenv
```

Shows details about a container: base distro, template used, creation time, any environment variables, and the storage path.

## Deleting a container

```sh
dxon delete myenv
```

Permanently removes the container and all its files. This cannot be undone.

## Running commands

To run a single command inside a container without entering an interactive shell, use `dxon enter` with a `--` separator:

```sh
dxon enter myenv -- cargo build --release
```

## Template commands

Inspect the template registry:

```sh
# List available templates
dxon template list

# Search for a template by keyword
dxon template search nodejs

# Refresh the local registry cache
dxon template refresh
```

See [Registry](registry.md) for more.

## Configuration commands

```sh
# Show current configuration
dxon config show

# Set a configuration value
dxon config set containers_dir /data/dxon/containers
```

## Global flags

| Flag | Description |
|---|---|
| `--help` | Show help for any command |
| `--version` | Print the dXon version |
