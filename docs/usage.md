# Usage

## Creating a container

The main command is `dxon create`. At minimum you provide a name for the container:

```sh
dxon create myenv
```

This starts an interactive prompt where you choose the base distro, shell, and optionally a template.

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
| `--shell <name>` | Shell to install: `bash`, `zsh`, or `fish` |
| `--shell-config <mode>` | Share host shell config: `copy` or `bind` |
| `--trust`, `-y` | Skip confirmation prompts (including trust warnings for untrusted templates) |

### Example: clone a repo into a container

```sh
dxon create myproject --distro arch --template rust --repo https://github.com/you/myproject
```

The repository is cloned into `/workspace` inside the container after setup completes.

### Shell selection

During `dxon create` you can choose which shell to install in the container: `bash`, `zsh`, or `fish`. The chosen shell becomes the default for `dxon enter`.

```sh
dxon create myenv --distro arch --shell zsh
```

### Sharing host shell config

You can bring your host shell configuration into the container with `--shell-config`:

| Mode | Behaviour |
|---|---|
| `copy` | Shell config files are copied into the container once at creation time. Paths referencing your home directory are rewritten to `/root`. |
| `bind` | Shell config files are bind-mounted from the host every time you enter. Changes on the host are immediately reflected inside the container. |

```sh
dxon create myenv --distro arch --shell zsh --shell-config copy
dxon create myenv --distro arch --shell zsh --shell-config bind
```

**Files copied/bound per shell:**

- **bash** — `.profile`, `.bash_profile`, `.bash_login`, `.bashrc`, `.bash_aliases`, `.bash_logout`, `.inputrc`
- **zsh** — `.zshenv`, `.zshrc`, `.zprofile`, `.zlogin`, `.zlogout`, `.inputrc` (respects `$ZDOTDIR`)
- **fish** — `~/.config/fish/` directory (respects `$XDG_CONFIG_HOME`)

## Entering a container

```sh
dxon enter myenv
```

This drops you into an interactive shell inside the container using the shell that was chosen at creation time. If no shell was configured, `bash` is used.

## Opening a container in an editor

```sh
dxon open myenv
```

Opens the container's `/workspace` directory (or root filesystem if no workspace exists) in a supported code editor. Editors are detected automatically in this order: VS Code (`code`), Cursor (`cursor`), Zed (`zed`).

To use a specific editor:

```sh
dxon open myenv --editor cursor
dxon open myenv --editor zed
```

### VS Code / Cursor terminal integration

When opening with VS Code or Cursor, dXon automatically writes a `.vscode/settings.json` inside the workspace that configures a **dXon** terminal profile. This profile runs `dxon enter <name>` so that every integrated terminal you open is already inside the container.

The profile is set as the default terminal, so pressing `` Ctrl+` `` drops you directly into the container shell.

### Zed

Zed does not yet support automatic terminal profile injection. Use `dxon enter <name>` from a host terminal alongside Zed.

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

To run a single command inside a container without entering an interactive shell:

```sh
dxon enter myenv -- cargo build --release
```

## Template commands

Inspect the template registry:

```sh
dxon template list
dxon template search nodejs
dxon template refresh
```

See [Registry](registry.md) for more.

## Configuration commands

```sh
dxon config show
dxon config set containers_dir /data/dxon/containers
```

## Global flags

| Flag | Description |
|---|---|
| `--help` | Show help for any command |
| `--version` | Print the dXon version |
