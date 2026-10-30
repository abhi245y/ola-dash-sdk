# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses [Semantic Versioning](https://semver.org/) for the host app. The theme contract has its own integer version (`contract-version` in `contract/vehicle.slint`).

## [Unreleased]

### Added

- Lock screen style (contract v6, within v6): optional `Dash` property `lock-style` with `LockStyle`, `LockKeyShape` and `LockDigits`. A theme styles the passcode keypad (colours, key shape, digit drawing) and the host keeps the keypad, the PIN and all copy. The host checks contrast and falls back to host colours. Segment, Data Fields and Default declare a style. Minimal shows the host default. The status line (clock, battery, LOCKED) uses the same tokens. Rules are in [docs/contract.md](docs/contract.md#lock-screen-style-v6) and [docs/safety.md](docs/safety.md#the-lock-screen-is-styled-never-handled).
- `sim --state locked`, `--state locked-error` and `--state cooldown` preview the lock screen.
- Lock-screen captures for the four reference themes in `docs/img/themes/`.

### Fixed

- Segment theme: the odometer (six digits) lost its last digit and showed a stray zero. Multi-digit segment numbers now use six places, with ghost leading cells.

- Clean-room vehicle interface (`sdk/vehicle-interface/`): an interoperability spec (`SPEC.md`) and AIDL, enum and callback-base files generated from it, written without Ola's code. The host builds from a clean checkout; CI builds the APK. Verified on a Gen 1 scooter against the previous classes (same callbacks and values).

- Contract v6: `Vehicle.trunk-open` (latch state from `isTrunkLock`) and `Actions.open-trunk()`. The host sends the stock trunk command only when the scooter is unlocked, in Park, stopped, latch closed, not in test mode and not within 3 s of the last request. Each request is logged. Guards are in `safety.md`; the gate is `host/java/dev/olatool/dash/TrunkGate.java` with a JVM test.
- Overlay icons redrawn as solid ISO-style glyphs (turn arrows, high and low beam, READY power symbol, padlock, side stand, fault, charging). Each sits in a lamp window with a hairline rim.
- Alert card hint "Tap to dismiss" for fault and low battery.

- Contract v6: optional `Dash` properties for the host safety overlay. `overlay-placement` (top, bottom, left, right; default top), `telltale-order`, `overlay-style` (background, foreground, corner radius, labels) and `alert-position` (center, upper, lower). New enums `OverlayPlacement`, `Telltale`, `AlertPosition` and struct `OverlayStyle`, exported from `contract/vehicle.slint`. Host enforces: minimum telltale icon 28 px and alert text 32 px, 4.5:1 contrast with host fallback, host-fixed semantic colours, non-dismissible safety alerts, and placement inside the screen.
- Overlay redesign: telltale strip on any edge with the reserved strip (48 px top or bottom, 72 px left or right), telltale glyphs, alert cards with a one-line hint, and a pulsing frame for fall and tow alerts. Overlay tokens at the top of `host/ui/overlay.slint`.
- `sim` renders the host overlay through the same root as the device. Mock telltales and a side-stand alert phase; `--placement` and `--alert` flags.

- Plugin host (API v1). Owner-enabled plugin APKs are bound through `files/plugins/enabled` and publish widgets, media and navigation. Each plugin gets its own `IDashHost`. Limits: 4 widgets per plugin, string lengths cut. A plugin that dies is rebound with backoff.
- Plugin data reaches the UI as one JSON snapshot per change, coalesced on the UI thread.
- Built-in media source (`StockMediaService`): the active media session feeds the Media card, and its play, pause, next and previous controls. Needs the notification listener grant, see [docs/plugins.md](docs/plugins.md).
- Example plugin `plugins/example-sysinfo`: free RAM, CPU temperature and uptime widgets, with its own build script and a README on the plugin anatomy.
- `sim` with `SIM_PLUGINS=1` shows mock plugin widgets and a navigation card, for theme work without a plugin.
- Default theme: plugin widget cards above the gauge and a navigation card in place of the media card while navigation is active. The minimal theme shows the first widget.

### Changed

- The unlock screen covers the dash only in the locked and authentication phases. From PARK on it is hidden, and the theme shows the dash. The PARKED text, the power-button hint and the side-stand hint were removed from the host screen.
- Overlay motion: telltale fade is 180 ms with a 6 px slide; the blink period stays 800 ms.
- Safety alert frames (codes 1-4) pulse; fault and low battery stay still. Tow hint reads "Tow / theft alert".

- Default theme declares a left-rail overlay with its own telltale order and style, and moves the plugin widget cards clear of the rail. Its contract is now 6. The minimal theme is unchanged (contract 5, top placement).

- Keep-alive also counts media playing in PARK or charging, as the stock HMI does. Only the built-in media source counts, never a plugin. Each reason change is logged (`keep-alive <bool> reason=<touch|media|none>`).
- `docs/plugins.md` is no longer a specification only: the host implements it.

## [0.1.0] - unreleased

First public snapshot. Experimental, in field testing on one rooted scooter.

### Added

- Android host app with a Rust and Slint runtime that loads themes at runtime, with a fallback chain (active theme, bundled default, last-resort placeholder).
- Theme contract, versions 1 to 5:
  - v2: park and reverse drive modes.
  - v3: custom drive mode and `hyper-available`.
  - v4: easy-park drive mode.
  - v5: `VehicleState`, `Battery`, `Ride` (brakes, regen, motor and MCU temperature), `Navigation`, `Plugins`. `-1` means "not reported yet".
- Bundled themes: `default` (arc gauge, battery ring, mode pills, media card, Android Auto and Settings buttons) and `minimal` (smallest theme using every field).
- `sim`: desktop mock-data runner with a live window and deterministic headless PNG output, used to design and screenshot themes.
- `sim --check` and `tools/check-themes.sh`: validate `theme.json` and compile every theme.
- Host-owned safety overlay for telltales and alerts, drawn above every theme. Debug builds show a TEST DATA banner when injected data is active.
- Keep-alive for the vehicle-parser service, matching the stock HMI's one-second cadence.
- Session logging: every vehicle callback with its arguments (`calls-*.log`) and raw CAN traffic (`can-*.log`), capped at 1 GiB.
- Debug-only test channel (`dev.olatool.dash.TEST`, `TEST_OFF`) with automatic timeout.
- Optional unlock module: passcode entry passed to the stock profile service. It does not bypass the service's attempt limit or cooldown.
- Development helpers for the home-screen swap (`host/test/hmi-swap.sh`) and test injection (`host/test/inject.sh`).
- Plugin API v1 specification (AIDL under `sdk/plugin-api/`). The host implementation is listed under Unreleased.
- Project documentation, Contributor Covenant 2.1 code of conduct, security policy, issue and PR templates, and CI.

### Known issues

- The host build is debuggable, so the test channel is present in the only APK produced so far. A release flavor is needed.

[0.1.0]: https://github.com/abhi245y/ola-dash-sdk/releases/tag/v0.1.0
