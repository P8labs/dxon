# dXon

> [!NOTE]  
> Use base distro same as host currently only that is supported. I am working to fix the combinations. You guys can try and raise Issue if it is not working.
> Thanks

small tool to spin up dev containers quickly. nothing fancy. just simple environments so your host system doesn't get polluted.

the idea is pretty straightforward: instead of installing a bunch of language stuff directly on your machine, you create a container for it and work inside that. keeps things clean and reproducible.

dXon uses system tools under the hood like `systemd-nspawn`, `pacstrap`, `debootstrap`, etc. so it's not trying to reinvent containers or anything. just making them easier to use for dev workflows.

still pretty early stage. expect things to break sometimes.

**[full documentation → https://dxon.p8labs.in](https://dxon.p8labs.in)**

## what it does

right now dXon lets you:

- create development containers
- delete containers
- list and inspect containers
- create containers from templates
- bootstrap containers with arch, debian or alpine
- clone a repo into the container on creation

containers are usually stored in:

```
~/.dxon/containers
```

config lives here:

```
~/.config/dxon/config.toml
```

you can change those if you want.

## quick example

create a node container:

```
dxon create node-dev --template node
```

enter it:

```
dxon enter node-dev
```

list containers:

```
dxon list
```

remove one:

```
dxon delete node-dev
```

nothing too magical.

---

## templates

dXon environments are driven by templates. templates are just yaml files that describe:

- base distro
- packages to install
- setup steps
- optional prompts

example (super simplified):

```yaml
schema: dxon/v1

name: node

base: arch

packages:
  arch:
    - nodejs
    - npm

steps:
  - run: corepack enable
```

templates can also ask you stuff during creation (like which package manager to use).

## template registry

we keep a public registry of templates here:

[https://github.com/P8labs/dxon-registry](https://github.com/P8labs/dxon-registry)

that's where common environments live:

- node
- rust
- python
- go
- c/c++

more will be added later probably.

dXon can pull templates directly from there so you don't have to copy files around.

you can also write your own templates if you want.

## why this exists

sometimes docker is too heavy for quick dev environments. sometimes installing stuff directly on your system becomes messy.

dXon is trying to be the middle ground:

- lightweight containers
- quick environment setup
- no system pollution
- simple templates

that's pretty much it.

## future stuff (maybe)

things we might add later:

- template registry improvements
- ai workflows / automation
- MCP integration
- project-based environments
- better container lifecycle commands

but yeah, one step at a time.

## project

dXon is built and maintained by **P8labs Team**.

it's an experiment right now but we'll see where it goes.

## license

MIT probably. will sort that out soon.
