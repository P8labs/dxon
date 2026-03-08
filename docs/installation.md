# Installation

dXon runs on Linux only. It uses `systemd-nspawn` under the hood, which is not available on macOS or Windows.

## Install script (recommended)

The fastest way to install dXon is the one-liner install script. It detects your architecture, downloads the latest release binary from GitHub, and places it in `/usr/local/bin`:

```sh
curl -sSfL https://raw.githubusercontent.com/P8labs/dxon/master/install.sh | sh
```

Or with `wget`:

```sh
wget -qO- https://raw.githubusercontent.com/P8labs/dxon/master/install.sh | sh
```

To install a specific version:

```sh
DXON_VERSION=v0.2.0 curl -sSfL https://raw.githubusercontent.com/P8labs/dxon/master/install.sh | sh
```

To install to a custom directory (e.g. `~/.local/bin`):

```sh
DXON_INSTALL_DIR=~/.local/bin curl -sSfL https://raw.githubusercontent.com/P8labs/dxon/master/install.sh | sh
```

Verify the installation:

```sh
dxon --version
```

## Prebuilt binaries

Prebuilt binaries for Linux (x86_64 and aarch64) are available on the [GitHub releases page](https://github.com/P8labs/dxon/releases). Download the binary for your architecture, make it executable, and move it somewhere on your `PATH`:

```sh
# x86_64
curl -sSfL -o dxon https://github.com/P8labs/dxon/releases/latest/download/dxon-linux-x86_64

# aarch64 / ARM64
curl -sSfL -o dxon https://github.com/P8labs/dxon/releases/latest/download/dxon-linux-aarch64

chmod +x dxon
sudo mv dxon /usr/local/bin/
```

## Install from source

dXon is written in Rust. You'll need [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html) installed.

```sh
git clone https://github.com/P8labs/dxon.git
cd dxon
cargo build --release
sudo cp target/release/dxon /usr/local/bin/
```

## Distribution-specific setup

After installing the `dxon` binary, you need `systemd-nspawn` plus the bootstrap tools for whichever base distros you plan to use. Instructions are grouped by the distribution you are **running dXon on** (your host).

---

### Arch Linux (host)

```sh
# systemd-nspawn is part of systemd — already present on a standard Arch install
# Bootstrap tools for each container distro you want:
sudo pacman -S arch-install-scripts   # to create Arch containers
sudo pacman -S debootstrap            # to create Debian/Ubuntu containers
sudo pacman -S wget curl              # to create Alpine containers (rootfs download)
```

---

### Debian / Ubuntu (host)

Install `systemd-container` to get `systemd-nspawn`:

```sh
sudo apt update
sudo apt install systemd-container
```

Bootstrap tools:

```sh
sudo apt install debootstrap          # to create Debian/Ubuntu containers
sudo apt install wget curl            # to create Alpine containers
```

To create **Arch Linux containers** on a Debian/Ubuntu host, install `arch-install-scripts`:

```sh
sudo apt install arch-install-scripts
```

> **Note:** `arch-install-scripts` may not be available in older Debian/Ubuntu repositories. You can build it from source or use a Debian Sid/Ubuntu 22.04+ mirror.

---

### Fedora / RHEL / CentOS Stream (host)

```sh
sudo dnf install systemd-container    # provides systemd-nspawn
sudo dnf install debootstrap          # to create Debian/Ubuntu containers
sudo dnf install wget curl            # to create Alpine containers
```

For Arch Linux containers, `pacstrap` is not packaged in Fedora repos. Install it manually via the [arch-install-scripts AUR tarball](https://github.com/archlinux/arch-install-scripts) or use a containerised build.

---

### openSUSE Leap / Tumbleweed (host)

```sh
sudo zypper install systemd-container
sudo zypper install debootstrap
sudo zypper install wget curl
```

---

### Alpine Linux (host)

> **Warning:** `systemd-nspawn` is not available on Alpine Linux (musl libc, OpenRC). dXon cannot run on an Alpine host.

---

### NixOS (host)

Add `systemd` to your system packages (it is typically present). Bootstrap tools can be added to your configuration:

```nix
environment.systemPackages = with pkgs; [
  debootstrap
  arch-install-scripts
  wget
  curl
];
```

Or install them into a temporary shell:

```sh
nix-shell -p debootstrap arch-install-scripts wget curl
```

---

## Dependencies summary

| Tool | Required for | Installed by |
|---|---|---|
| `systemd-nspawn` | Running any container | `systemd-container` package |
| `pacstrap` | Creating Arch Linux containers | `arch-install-scripts` |
| `debootstrap` | Creating Debian / Ubuntu containers | `debootstrap` package |
| `wget` or `curl` | Creating Alpine containers | `wget` / `curl` packages |

dXon will print a clear error if a required tool is missing when you try to create a container.

## Updating

Use the upgrade script to update dXon to the latest release:

```sh
curl -sSfL https://raw.githubusercontent.com/P8labs/dxon/master/upgrade.sh | sh
```

Or with `wget`:

```sh
wget -qO- https://raw.githubusercontent.com/P8labs/dxon/master/upgrade.sh | sh
```

The script checks your currently installed version and only downloads a new binary if a newer release is available.

To update dXon installed from source, pull the latest changes and rebuild:

```sh
cd dxon
git pull
cargo build --release
sudo cp target/release/dxon /usr/local/bin/
```
