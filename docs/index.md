# dXon

dXon is a lightweight development container manager. instead of installing language toolchains and project dependencies directly on your host system, you create isolated containers for them. keeps things clean and reproducible.

it wraps system tools like `systemd-nspawn`, `pacstrap`, `debootstrap`, and Alpine's rootfs setup — no reinventing the wheel here, just making containers easier to use for everyday dev workflows.

## quick example

create a Node.js container from a template:

```sh
dxon create node-dev --template nodejs
```

enter it:

```sh
dxon enter node-dev
```

list all containers:

```sh
dxon list
```

delete one when you're done:

```sh
dxon delete node-dev
```

that's the core workflow. the [Usage](usage.md) page covers all commands in detail.

## how it works

when you create a container, dXon:

1. bootstraps a base Linux environment (Arch, Debian, or Alpine)
2. installs packages defined by the template
3. runs setup steps inside the container
4. stores everything under `~/.dxon/containers/`

from there you can enter the container any time with `dxon enter`.

## next steps

- [Installation](installation.md) — get dXon on your system
- [Usage](usage.md) — full CLI reference
- [Templates](templates.md) — understand and write templates
- [Registry](registry.md) — community template registry
- [Configuration](configuration.md) — configure dXon to your liking
- [Security](security.md) — how template trust works
