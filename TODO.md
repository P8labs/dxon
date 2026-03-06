# dxon TODO

## Core

- [x] Define CLI structure with clap (create, delete, list, info, enter)
- [x] Global `--dir` / `DXON_DIR` flag to override container storage directory
- [x] Container metadata (JSON) per container (distro, created_at, template, packages, repo, config)
- [x] Container storage (create dirs, save/load/list/remove meta)
- [x] Structured error types with actionable messages
- [x] Validate required system tools before operations (pacstrap, debootstrap, systemd-nspawn)
- [x] Check for root privileges before bootstrap/enter operations

## Container lifecycle

- [x] `dxon create` – interactive step-by-step mode when flags are omitted
- [x] `dxon create` – non-interactive mode when `--distro` and `--template` flags are provided
- [x] `dxon create --repo <url>` – clone a Git repository into `/workspace` inside the container
- [x] `dxon delete <name>` – remove a container with confirmation prompt
- [x] `dxon delete --force` – skip confirmation
- [x] `dxon list` – tabular list of all containers
- [x] `dxon info <name>` – detailed view of container metadata
- [x] `dxon enter <name>` – drop into an interactive shell with systemd-nspawn
- [x] `dxon enter <name> -- <cmd>` – run a specific command inside the container

## Bootstrap

- [x] Arch bootstrap via `pacstrap -c`
- [x] Debian bootstrap via `debootstrap stable`
- [x] Alpine bootstrap via curl + tar (mini-rootfs) + `apk update`
- [ ] Alpine: resolve correct architecture variant URL automatically
- [ ] Support Ubuntu as an alias for Debian bootstrap

## Template system

- [x] `.dx` template format (TOML)
- [x] `TEMPLATE_SPEC.md` documenting all template fields
- [x] Built-in `nodejs` template with pnpm / yarn / bun / npm prompt
- [x] Built-in `rust` template with rustup + clippy + rustfmt
- [x] Built-in `go` template
- [x] Built-in `cpp` template (gcc, g++, cmake, ninja, gdb, clang)
- [x] Built-in `python` template with pip / poetry / uv prompt
- [x] All built-in templates support optional Docker tooling install
- [x] Remote template fetching via URL (`--template https://...`)
- [x] Local file template (`--template ./my-stack.dx`)
- [ ] `dxon template list` – list all available built-in templates
- [ ] `dxon template show <name>` – print the TOML source of a built-in template

## Developer experience

- [x] Colored, structured terminal output (colored crate)
- [x] Interactive prompts via dialoguer
- [x] Human-readable error messages instead of raw system output
- [x] Clean separation of concerns across modules
- [ ] Progress spinner during long-running bootstrap operations
- [ ] `--quiet` flag to suppress non-essential output
- [ ] Shell completion generation (`dxon completions <bash|zsh|fish>`)

## Configuration

- [ ] `~/.dxon/config.toml` for persistent defaults (default distro, default template)
- [ ] `dxon config set <key> <value>` command

## Workspace integration

- [x] Clone Git repo into container at creation time
- [ ] Bind-mount a host directory into the container (`--bind <host:container>`)
- [ ] `dxon exec <name> <cmd>` – run a one-off command without entering interactively

## Distribution & packaging

- [ ] Release binary builds via GitHub Actions (x86_64, aarch64)
- [ ] AUR package (Arch)
- [ ] Install script (`curl | sh`)
