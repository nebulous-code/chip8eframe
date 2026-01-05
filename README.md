# Chip-8 eframe

Desktop GUI for the Chip-8 emulator built with egui/eframe.

## Run

- `cargo run`
- `cargo run --release`

## Notes

- The ROM path is resolved relative to the crate directory.
- The window icon is loaded from `assets/icon-256.png`.

## Linux dependencies

On Ubuntu/Debian:

`sudo apt-get install libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libxkbcommon-dev libssl-dev`

On Fedora Rawhide:

`dnf install clang clang-devel clang-tools-extra libxkbcommon-devel pkg-config openssl-devel libxcb-devel gtk3-devel atk fontconfig-devel`
