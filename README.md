# Korsa

Korsa is a native Assetto Corsa companion for Linux.

The current milestone provides a native, Kvantum-painted KDE dashboard with KWin blur and contrast, metadata-rich car and track catalogs, visual car skin/livery and session-mode selection, AI grids and race details, persisted favorites, reusable session and controller presets, live quick and advanced track-condition editing with time shortcuts, searchable Assetto Corsa and CSP settings, a dedicated wheel/gamepad/keyboard assignment workflow, a visual in-game app layout editor, validated car/track/CSP ZIP installation with backups and rollback, official Kunos lobby browsing and joining, native showroom launch, local car setup management, safe asynchronous Steam discovery, and direct launch through Steam Linux Runtime and Proton.

Applying a session preset updates only its supported keys in `race.ini` using an atomic replacement. Standard Assetto Corsa settings are discovered from the user `cfg/*.ini` files and merged with shipped defaults. CSP changes are written as sparse user overrides under Assetto Corsa's Documents directory. Launching by itself does not modify race, assists, or CSP configuration.

## Requirements

- Rust 1.85 or newer
- Qt 6 with Core, Gui, Widgets, and Network
- KWindowSystem and an optional Qt widget style such as Kvantum
- A C++ compiler
- A MinGW-w64 x86_64 C compiler (`x86_64-w64-mingw32-gcc`, or set `KORSA_MINGW_CC`)
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

## Controller identity checks

Before launching a wheel-controlled session, Korsa enumerates attached DirectInput
controllers inside the configured Proton prefix and Steam runtime. It matches
hardware product GUIDs (falling back to names only for old presets without product
GUIDs), then refreshes the saved Wine instance IDs. It does **not** change axis,
button, force-feedback, or calibration bindings. The previous configuration is
saved as `cfg/controls.ini.before-korsa-controller-refresh` whenever a change is
needed. Keyboard and gamepad sessions keep their existing launch path.

The bundled Windows helper keeps the Wine session alive throughout validation
and starts AC itself after approval, so there is no Wine restart between
enumeration and launch. Missing required controllers, ambiguous matches, helper
errors/timeouts, and device changes during validation block launch with an error.
Unused/unbound controllers do not block launch. The same check covers the GUI,
dashboard, CLI, online races, and showroom launch paths.

Korsa remembers each matched HID path as `__KORSA_PATH<n>` in `[CONTROLLERS]`.
For identical devices, connect only the intended device for the first launch;
subsequent launches use that path to disambiguate them. To deliberately replace
hardware or accept a changed driver path, close AC and remove the corresponding
`__KORSA_PATH<n>` entry from `cfg/controls.ini`, then launch with only the intended
device connected. Do not copy instance IDs between machines or hardcode them:
Wine can recycle them for different devices. Live settings input uses Linux
joysticks; the pre-launch check independently validates the identities AC uses.

## Race dashboard synchronization

The dashboard supports six pinned cars and six pinned track layouts in 3×2 grids. Dashboard
clicks save the selected car, track, or time immediately, without starting AC;
the main window follows the saved session within its next refresh (about one
second). Main-window edits and presets also notify the dashboard. Selections
outside the pinned choices remain active without falsely highlighting a different
choice. Selection requests are serialized so rapid clicks cannot apply out of
order. START launches the saved shared session instead of reapplying a cached
client selection. D-Bus protocol version 2 adds `SetDashboardSelection`,
`LaunchSelectedSession`, `SessionChanged`, and
the active `selection` in `GetDashboardOptions`; update Korsa and the dashboard
client together.

Korsa remembers the last livery used for each car in
`cfg/korsa-liveries.json` inside that installation's Assetto Corsa Documents
directory. Switching cars (including variants and dashboard selections) restores
that car's livery; an explicit livery choice or session preset takes precedence.
The outgoing car's existing livery is remembered on the first switch too. If a
remembered skin was removed, Korsa uses an installed fallback instead. Dashboard
thumbnails show each car's current/remembered livery, with its name in the tooltip.

## Project Direction

1. Add skin selection and structured car specifications.
2. Add standard single-player modes, assists, results, and setups
3. Add typed CSP color, file, and enum editors alongside the bounded range editor.
4. Add safe, reversible content installation.

## Licensing

Original code in this repository is available under the GNU General Public License version 3. It does not contain Assetto Corsa or Content Manager source code or assets.

Assetto Corsa is proprietary software owned by Kunos Simulazioni. This project is not affiliated with or endorsed by Kunos Simulazioni, Valve, or the Content Manager developers.
