> [!NOTE]  
> Discontinued since I don't use Linux anymore.

# Waysnip

A simple screenshot selection tool for Wayland.

## Features

* It can do screenshot (shocking)

## Dependencies

* GTK4
* gtk4-layer-shell
* grim
* wl-clipboard

### Install (examples)

**Arch Linux**

```sh
pacman -S gtk4 gtk4-layer-shell grim wl-clipboard
```

**Fedora**

```sh
dnf install gtk4-devel gtk4-layer-shell-devel grim wl-clipboard
```

**Ubuntu / Debian**

```sh
apt install libgtk-4-dev libgtk4-layer-shell-dev grim wl-clipboard
```

## Build

```sh
cargo build --release
```

Binary location:

```
target/release/waysnip
```

## Usage

```sh
waysnip
```

### Keyboard Shortcuts

* `Ctrl+A` — Select entire screen
* `Ctrl+C` — Copy to clipboard
* `Ctrl+S` — Save to file
* `Esc` — Exit

## License

MIT
