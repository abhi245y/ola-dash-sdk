# Changelog

All notable changes to this project are documented here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and the project uses [Semantic Versioning](https://semver.org/) for the host app. The theme contract has its own integer version (`contract-version` in `contract/vehicle.slint`).

## [Unreleased]

### Changed

- Host design language "Ride Buddy, day and night" (E1.S2; tokens in `ola-dash/docs/design/host-ui-tokens.md`). `HostState.night` (default day; `sim --night` sets it with `Display.night`, Settings sets both on the device; E3.S1 adds auto) switches the host defaults for the strip and the lock screen between day and night. The strip is always dark (`#0F2A2A` day, `#071414` night), 48 px top or bottom and 72 px left or right, and shows solid glyphs: lit in the status colour, unlit as 40 % outlines (the old lamp windows are gone). NO DATA and TEST DATA stay in its end region. Alert cards are white on a 60 % scrim with a red or amber frame, a 32 px title and a one-line hint, the same in both modes; while the lock screen is up, an active warning shows its title on the same card in the left column, so it never hides behind the keypad. Alert hints now carry only an action or extra information (the title is not repeated). A theme's `overlay-style` is also refused when a lit telltale colour (green, blue, grey, amber or red) has less than 3:1 on its `background`, because the glyphs no longer sit in a dark lamp window; the host strip colours are used and the reason is logged. `OverlayStyle.corner-radius` stays in the contract but has no visible effect.
- Lock screen: clock (120 px), date and battery (`82%`, or `--` when unknown) on the left; caption "Enter your scooter passcode", six passcode cells and the keypad (keys 188 x 96 px, 16 px gaps) on the right. Delete is the accent-filled strong key and Clear is quiet. The padlock, "LOCKED" and a battery glyph are gone from the body (the strip shows lock and charging). Under the caption only the attempts left or the error shows, and during the cooldown the caption is the countdown ("Try again in 272 s"). Charging progress while locked is not shown: the contract has no ETA data. `lock-style` themes still work; `accent` now fills the Delete key, so its rule is "text on it reaches 4.5:1" (was 3:1 on the background), and the host picks that text colour. The date comes from `HostState.date-text`: `MainActivity`'s minute clock pushes `yyyymmdd` to host slot 23 (`VehicleBridge.DATE_YMD`, not replayed) and Rust formats "Fri 31 Oct" (`theme_root::date_text`); the sim sets it too.
- Host font: Figtree (SIL OFL 1.1) Medium, Bold and ExtraBold, subset and renamed `OlaDash Host`, replaces the Inter subset. Digits are tabular. `tools/subset-host-font.py` now takes the directory with the Figtree files.

### Added

- Plugins pane (E1.S5): installed plugins expose declared slots and immediate Off / On choices behind the existing host-screen gate; atomic enablement writes preserve comments and slot restrictions. Sim adds `--pane plugins --plugins-file FILE` with in-memory toggles, a seeded fixture, unit tests and an On-pill pixel probe.
- Theme pane and first-run setup (E1.S4 PR2): compatible installed themes can be selected and applied through the existing restart flow. Setup offers theme and immediate Night mode choices, persists Done, and waits for the host-screen gate beneath alerts and the lock screen. Sim adds `--pane theme` and `--first-run`, with discovery/setup unit tests and pixel probes.
- Theme switching (E0.S6 T1–T3): host-only `switch-theme(id)` behind the host-screen gate shows a switching card, saves `themes/previous` and `themes/active`, clears the bad-theme strike and exits for the home app to relaunch. The next start keeps the switching card over the lock screen until lock state is known. After relaunch a Keep / Revert card counts down 10 s (paused while host screens are not allowed); Revert or timeout restores the old theme. A solid day-colour window background covers the restart gap (night may flash light). Assets are copied only after an APK update (`files/.assets-stamp`). Sim adds `--switch ID`, `--confirm-theme` and `--state restart`.
- Settings core (E1.S4 PR1): Display precedes About, with Night mode Off / On and 24-hour / 12-hour clock choices using checked lock colours. Rust owns `files/settings.json`, preserves unknown keys, syncs a sibling `.tmp` before atomic rename, and quarantines corrupt files as `.bad`. Night updates the host and theme together; host clock formatting changes immediately. `Vehicle.clock-text` stays 24-hour `HH:MM`, so themes are unaffected by the clock setting. The lock screen reads numeric `HostState.clock-hhmm` and shows h:mm with a separate baseline-aligned AM/PM label in 12-hour mode (`--:--` when unknown). The root stores the selected pane index; only `SettingsScreen` has a rail cursor, initialized from that selection. `night: "auto"` reads as Off until E3.S1. The sim adds `--settings FILE` and `--pane display|about`; persistence and clock-format unit tests and pixel probes cover the new behavior.
- Host shell (E1.S3): a host-owned hub in the strip opens Settings from any theme while parked, unlocked, with no alert and outside test mode. Settings has an About rail entry with host and contract versions, a Close button, and a clock/battery status line, using the host font and day/night tokens. Screens close on leaving parked, an alert, locking or test mode. `Actions.open-settings()` and `Actions.open-themes()` open this host frame; Android settings is no longer launched. Apps still logs "not built yet". NO DATA wins over the hub while stale. `sim --screen settings --state park` previews the frame; theme checks probe parked and riding screen captures.
- Release build: `host/build.sh --release` signs with the key named by `OLADASH_KEYSTORE`, `OLADASH_KEY_ALIAS`, `OLADASH_KS_PASS` and `OLADASH_KEY_PASS`, takes the version from the latest `v*` git tag, fails if the APK is debuggable, and prints the signer certificate SHA-256. Output `host/out/ola-dash-release.apk`. The manifest no longer sets `android:debuggable`; the default build passes `aapt2 link --debug-mode` and stays debuggable.
- Bad-theme guard (`BadThemeGuard.java`): the host counts a start strike for the active theme in `files/themes/strikes` before Slint starts and clears it after 10 s. A theme that crashed two starts in a row is replaced by `default` and listed in `files/themes/bad`. `BadThemeGuard.cleanExit()` clears the strike; it runs from the 10 s timer, from `onPause`, and through JNI before theme-switch/revert `exit(0)`.
- Contract v7 (additive, v6 themes load unchanged): `Display.night`, `Vehicle.data-stale`, `Actions.open-apps()` and `Actions.open-themes()`. `Actions.open-themes()` now opens host settings while parked; `Actions.open-apps()` still only logs "not built yet". `sim --night` sets `Display.night`, `sim --stale` sets `Vehicle.data-stale`. Bundled themes stay on contract 6.
- Stale data: the host sets `Vehicle.data-stale` and its own `HostState.data-stale` (field 38) when the vehicle parser disconnects or dies, or when no real vehicle data callback arrived for 5 s while in drive (checked by the existing 1 s keep-alive tick; injected test events do not count). The overlay shows a red NO DATA badge in the TEST DATA slot (hidden while TEST DATA shows). It clears when callbacks resume or the parser resubscribes.
- The host keeps the screen on (`FLAG_KEEP_SCREEN_ON`) and logs `timing probe|root|create|first-frame <N> ms` under tag `OlaDash` (first frame counts from `android_main` entry).
- Park gate: `TrunkGate.parked` (the trunk request's park rule, also true while unlocked and charging, never in test mode) is pushed to `HostState.parked`, and `parked -> false while paused` is logged when the host is in the background. The host root uses it to gate interactive screens. `sim` derives it from the scenario.
- `host/build.sh`: optional `EXTRA_THEMES=<dir>` bundles each subfolder with a `theme.json` into the APK as `themes/<id>/`.
- Lock screen style (contract v6, within v6): optional `Dash` property `lock-style` with `LockStyle`, `LockKeyShape` and `LockDigits`. A theme styles the passcode keypad (colours, key shape, digit drawing) and the host keeps the keypad, the PIN and all copy. The host checks contrast and falls back to host colours. Segment, Data Fields and Default declare a style. Minimal shows the host default. The status line (clock, battery, LOCKED) uses the same tokens. Rules are in [docs/contract.md](docs/contract.md#lock-screen-style-v6) and [docs/safety.md](docs/safety.md#the-lock-screen-is-styled-never-handled).
- `sim --state locked`, `--state locked-error` and `--state cooldown` preview the lock screen.
- Lock-screen captures for the four reference themes in `docs/img/themes/`.

### Fixed

- Theme sandbox, part 2: the overlay and the lock screen read `HostState` copies of the telltale values, clock and battery that only the host sets (`apply()` and the sim write both), so a theme writing `Vehicle.left-indicator = false` (from a `Timer`, `changed` or a `<=>` binding) can no longer blank a lit telltale. Host text now uses its own font family, `OlaDash Host` (Inter Regular, SemiBold and Bold, subset, 130 KB in `host/ui/fonts/`, shipped to `files/host/fonts/`), instead of inheriting the theme's `default-font-family`; the lock screen's `digits: sans` follows. The import guard refuses a theme whose `.ttf`/`.otf`/`.ttc` bytes contain that name (fontique merges same-name faces), a font file over 16 MiB, or a font import that leaves the theme folder. New fixtures `tools/fixtures/loads/evil1` (a theme that writes `Vehicle.*` to blank telltales) and `tools/fixtures/refused/evil4`; `tools/check-themes.sh` checks that a lit left indicator shows its colour at ride t=13 for every `loads/*` fixture. Host strings in a theme with a different font (Segment) now render in Inter.
- Theme import guard: the probe compile refuses a theme that imports `host/ui/*.slint` or any file outside its folder and `contract/`, follows a symlink, or uses `PopupWindow`, `ContextMenuArea`, `MenuBar`, `Tooltip`, `DragArea` or `std-widgets.slint`. The root compile is guarded too, and the host wraps `Dash` so a theme's `z` cannot lift it above the overlay. Before, a theme could write `HostState` and `OverlayConfig` to hide telltales, alerts and the keypad, or cover them with a full-screen popup. `tools/check-themes.sh` now requires `sim --check` to refuse the hostile themes in `tools/fixtures/refused/` and to load `tools/fixtures/loads/` with the overlay intact (new `sim --pixel X,Y`). CI now runs the sim unit tests.
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

- Reference themes declare their overlay settings: Segment and Default put the telltale strip on top, Data Fields on the left rail, each with its own telltale order and style. All four bundled themes (including minimal, which keeps the host defaults) declare contract 6.

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

- `Vehicle.trunk-open` is derived as `!isTrunkLock`; the polarity is inferred from ride logs and not yet verified on a scooter. The Open Trunk button and its gate have not been exercised on hardware.

- The host build is debuggable, so the test channel is present in the only APK produced so far. A release flavor is needed.

[0.1.0]: https://github.com/abhi245y/ola-dash-sdk/releases/tag/v0.1.0
