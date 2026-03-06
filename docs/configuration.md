# Configuration

dxon stores its configuration file at:

```
$HOME/.config/dxon/config.toml
```

The file is created with defaults the first time dxon runs. View current settings with:

```sh
dxon config show
```

Change a value directly from the CLI:

```sh
dxon config set containers_dir /data/dxon/containers
```

Or edit the file directly with any text editor.

## Configuration options

### `containers_dir`

Path where container filesystems are stored.

```toml
containers_dir = "/home/user/.dxon/containers"
```

Default: `~/.dxon/containers`

### `registry_url`

URL of the template registry index. Change this to use a custom or self-hosted registry.

```toml
registry_url = "https://raw.githubusercontent.com/P8labs/dxon-registry/main/registry.json"
```

Default: the official dxon registry index.

## Full default configuration

```toml
containers_dir = "~/.dxon/containers"
registry_url = "https://raw.githubusercontent.com/P8labs/dxon-registry/main/registry.json"
```

## Example: move container storage

If you want to store containers on a different drive or partition:

```toml
containers_dir = "/data/dxon/containers"
```

Make sure the directory exists and is writable before creating any containers.

## Example: use a custom registry

To use a private or self-hosted template registry:

```toml
registry_url = "https://registy.example.com/dxon/registry.json"
```

The registry endpoint must return a JSON index in the same format as the official registry. See [Registry](registry.md) for details on the registry format.
