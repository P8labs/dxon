# Installation

## Prebuilt binaries

Prebuilt binaries for Linux (x86_64) are available on the [GitHub releases page](https://github.com/P8labs/dxon/releases). Download the binary for your architecture, make it executable, and move it somewhere on your `PATH`:

```sh
chmod +x dxon
sudo mv dxon /usr/local/bin/
```

Verify the installation:

```sh
dxon --version
```

## Install from source

dXon is written in Rust. You'll need [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) installed.

```sh
git clone https://github.com/P8labs/dxon.git
cd dxon
cargo build --release
sudo cp target/release/dxon /usr/local/bin/
```

## Install script

An install script is in the works. Once available it will let you install dXon with a single command:

```sh
curl -sSfL https://raw.githubusercontent.com/P8labs/dxon/master/install.sh | sh
```

_This script is not yet published._

## Package managers

Distribution packages are planned but not yet available. The following may be supported in the future:

- **AUR** (Arch Linux) — `dxon` or `dxon-bin`
- Other distro packages as interest grows

## Updating

To update dXon installed from source, pull the latest changes and rebuild:

```sh
cd dxon
git pull
cargo build --release
sudo cp target/release/dxon /usr/local/bin/
```

For prebuilt binaries, download the latest release and replace the existing binary.

## Dependencies

dXon relies on system tools to do the actual container work. Depending on which base distributions you want to use, you may need:

| Distribution | Required tool |
|---|---|
| Arch Linux containers | `pacstrap` (from `arch-install-scripts`) |
| Debian/Ubuntu containers | `debootstrap` |
| Alpine containers | `wget` or `curl` to fetch the Alpine rootfs |

dXon will print a clear error if a required tool is missing when you try to create a container.
