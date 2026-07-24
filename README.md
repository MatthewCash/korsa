# AC Linux Manager

AC Linux Manager is a native Linux launcher and content manager for Assetto Corsa.

The current milestone provides a native, Kvantum-painted KDE dashboard with KWin blur and contrast, metadata-rich car and track catalogs, visual car skin/livery and session-mode selection, AI grids and race details, persisted favorites, reusable session and controller presets, live quick and advanced track-condition editing with time shortcuts, searchable Assetto Corsa and CSP settings, a dedicated wheel/gamepad/keyboard assignment workflow, a visual in-game app layout editor, validated car/track/CSP ZIP installation with backups and rollback, official Kunos lobby browsing and joining, native showroom launch, local car setup management, safe asynchronous Steam discovery, and direct launch through Steam Linux Runtime and Proton.

Applying a session preset updates only its supported keys in `race.ini` using an atomic replacement. Standard Assetto Corsa settings are discovered from the user `cfg/*.ini` files and merged with shipped defaults. CSP changes are written as sparse user overrides under Assetto Corsa's Documents directory. Launching by itself does not modify race, assists, or CSP configuration.

## Requirements

- Rust 1.85 or newer
- Qt 6 with Core, Gui, Widgets, and Network
- KWindowSystem and an optional Qt widget style such as Kvantum
- A C++ compiler
- `qmake6` in `PATH`, or `QMAKE` set explicitly
- Steam Linux Runtime 3.0 (sniper), installed automatically by Steam for Assetto Corsa
- Read access to matching `/dev/input/js*` devices for live wheel, pedal, shifter, and button assignment

## Build

On NixOS, enter the pinned development environment first:

```sh
nix develop path:.
```

```sh
cargo build
cargo run
```

Launch the existing `race.ini` session without opening the dashboard:

```sh
cargo run -- --launch-last
```

Set `RUST_LOG=debug` for detailed diagnostics. Set `STEAM_DIR` to override Steam discovery.

## Project Direction

1. Add skin selection and structured car specifications.
2. Add standard single-player modes, assists, results, and setups
3. Add typed CSP color, file, and enum editors alongside the bounded range editor.
4. Add safe, reversible content installation.

## Licensing

Original code in this repository is available under the GNU General Public License version 3. It does not contain Assetto Corsa or Content Manager source code or assets.

Assetto Corsa is proprietary software owned by Kunos Simulazioni. This project is not affiliated with or endorsed by Kunos Simulazioni, Valve, or the Content Manager developers.
