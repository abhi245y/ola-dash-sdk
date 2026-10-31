# Architecture

The dash is a Slint app running inside an Android app on the scooter's head unit. The Ola vehicle-parser service stays in charge of the vehicle. The host reads from it, draws the screen, and sends no vehicle command except the one gated trunk request (see [safety.md](safety.md#the-one-vehicle-request)).

## Target hardware (S1 Pro Gen 1, measured on one scooter)

Android 10 (API 29), Qualcomm SDM450 (msm8953 family, arm64), Adreno 506 GPU (GLES 3.2), 2.9 GB RAM, 1280x768 display at density 236 (the host forces a 1.0 scale factor so themes use panel pixels), multitouch. Wi-Fi with Wi-Fi Direct, Bluetooth and BLE, GPS, cellular modem. No Google services. Other generations are unknown.

## Components

```
Ola vehicle-parser service  (stock, Android service on the head unit)
        |  AIDL callbacks (IVehicleInfoCallbacks, IBatteryInfoCallbacks, ...)
        v
VehicleBridge.java          host/java/dev/olatool/dash/
        |  JNI: VehicleBridge.nativeSet(...)
        v
Rust host (liboladash.so)   host/src/lib.rs
   |- theme loader          loads themes/<id>/main.slint at runtime (slint-interpreter)
   |- contract globals      Vehicle, Display, Battery, Ride, Media, Navigation, Plugins, Actions
   |- host screens          host/ui/settings.slint, above the theme, below the safety overlay
   |- safety overlay        host/ui/overlay.slint, drawn above the theme
   '- plugin data           JSON snapshot from PluginHost (widgets, media, navigation)
        ^  JNI PluginHost.nativeSetPlugins(json)
PluginHost.java, StockMediaService.java: bind enabled plugins, one IDashHost each
        ^  AIDL IDashPlugin / IDashHost (plugin APKs, display-only)
```

| Component | Path | Role |
|---|---|---|
| Vehicle-parser service | stock, on the head unit | Source of vehicle state, reached over binder. |
| Vehicle interface | `sdk/vehicle-interface/` | Clean-room AIDL, enums and logging callback bases for that service, generated from `SPEC.md` (no Ola code). |
| `VehicleBridge` | `host/java/dev/olatool/dash/VehicleBridge.java` | Receives callbacks, forwards values to Rust, runs keep-alive, writes trace logs. |
| `TrunkGate` | `host/java/dev/olatool/dash/TrunkGate.java` | Pure rules for the trunk request. Unit-tested on a desktop JVM. |
| Rust host | `host/src/lib.rs` | Owns the Slint runtime, loads the active theme, pushes values into the contract globals. |
| Safety overlay | `host/ui/overlay.slint` | Telltales, alert cards, the TEST DATA banner. Painted after the theme, so no theme can cover it. |
| Unlock module | `host/java/dev/olatool/dash/Unlock*.java`, `ProfileService.java` | Passcode screen (covers the dash until PARK) and stock profile-service client. See [safety.md](safety.md). |
| Theme | `themes/<id>/` | Only Slint code. Reads the contract, calls the allowed callbacks. |
| Contract | `contract/vehicle.slint` | The only API a theme sees. See [contract.md](contract.md). |
| Sim | `sim/` | Desktop runner that feeds the same contract from a mock clock. |
| Plugin host | `host/java/dev/olatool/dash/plugin/PluginHost.java` | Binds owner-enabled plugin services, gives each its own `IDashHost`, pushes one JSON snapshot to Rust (coalesced there). |
| Built-in media | `host/java/dev/olatool/dash/plugin/StockMediaService.java` | Notification listener that reads the active media session and sends transport controls. |
| Example plugin | `plugins/example-sysinfo/` | Canonical plugin template: three widgets, with its own `build.sh`. |

## Data flow

1. The vehicle-parser service calls back into `VehicleBridge` on a binder thread.
2. `VehicleBridge` hands each value to Rust through JNI. Each callback is also written to the calls log.
3. The Rust host sets the `in` properties on the contract globals. Themes read them. Themes never write them.
4. Theme callbacks (`Media.play-pause`, `Actions.open-settings`, and the others the contract allows) come back to the host. The host decides what they do. A theme cannot reach any other function. `Actions.open-trunk` goes to `MainActivity.onOpenTrunk`, then `VehicleBridge.openTrunk`, where `TrunkGate` decides.

## Themes and fallback

Themes are loaded in three tiers. A failed compile or create logs its diagnostics under logcat tag `OlaDash` and falls to the next tier, so the app does not crash:

1. The active theme (name in `files/themes/active`, default `default`).
2. The bundled fallback, a copy of `themes/default`.
3. The bundled last-resort placeholder, `host/ui/last-resort/main.slint`.

A theme that compiles but crashes the host at start (panic, out of memory) is caught by the bad-theme guard: `MainActivity.onCreate` counts a strike for the active theme in `files/themes/strikes` (`<name> <n>`) before Slint starts, and clears it 10 s later, on pause or on a clean exit (`BadThemeGuard.cleanExit()`, which the theme switch must call before `exit(0)`). A theme with two strikes is replaced by `default` in `files/themes/active` and added to `files/themes/bad`, one name per line.

## Safety overlay

The overlay is compiled into the host and painted after the theme's `Dash` component. Themes can choose its placement, telltale order and style through optional `Dash` properties, but have no API to hide it, drop a telltale, or draw over it. Telltales and alerts are host-owned. Debug builds add a TEST DATA banner while injected values are active. The full model is in [safety.md](safety.md).

## Host screens

`Host.screen` in `host/src/theme_root.rs` accepts `dash`, `settings`, `switching` and `confirm-theme`. Host screens are drawn after the wrapped theme and before `HostOverlay`, so the strip, alerts and lock screen stay above them. Navigation stays in Slint: the hub and `Actions.open-settings()` / `open-themes()` request settings through the root gate, and Close returns to the dash. Leaving parked, an alert, locking or test mode resets the screen; the draw condition also checks the gate. The gate reads only host state, including `TrunkGate`'s parked result.

The rail lists Display, then About. Display has Night mode (Off / On) and Clock (24-hour / 12-hour), using the shared `Choice` control and checked `LockConfig` colours. About shows the host crate version, set once at startup in `HostState.version-text`, and the contract version. The shared clock and battery status line reads host copies and shows `--` for unknown values. `Host.settings-pane` stores the selected index (0 Display, 1 About), bound to `SettingsScreen.selected`. Only `SettingsScreen` has a rail `cursor`, initialized from `selected` when opened; `move(delta)` moves it, `select()` applies it, and `close()` returns to the dash. Apps is not built yet; its callback logs only.

## Theme switching

`Host.switch-theme(id)` (no caller yet; the Theme pane comes in E1.S4 PR2) runs only while `host-screens-allowed` and no switch or confirmation is pending. It shows the switching card (`host/ui/theme-switch.slint`) and calls Rust `apply-theme`. Rust refuses invalid IDs (loader rule), unknown, bad-listed, already-active and incompatible themes, and returns to Settings. After 50 ms, so the card is drawn, it rechecks the gate, writes `files/themes/previous` and `active` (`host/src/theme_switch.rs`, synced `.tmp` + rename), writes the `restart` marker with the theme name, clears the bad-theme strike through `MainActivity.onThemeExit()`, and exits. Android relaunches the home app; a development install that is not home must be reopened by hand.

On the next start Rust reads and deletes `restart` into `HostState.restart-name`. While it is set and the lock state is still unknown (auth phase 0, no connection error), the overlay draws the switching card in place of the lock screen, so the keypad does not flash. If `previous` exists, the Keep / Revert card counts down 10 s while host screens are allowed (it pauses otherwise). Keep deletes `previous`. Revert or timeout restores it to `active` and restarts the same way; a corrupt `previous` is dropped. If the new theme fails to compile, the card shows over the fallback and Keep keeps the broken selection; the next start falls back again.

`host/res/values/styles.xml` sets a `#F6F3EC` window background, the day switching colour, for the restart gap; at night the gap flashes light. `MainActivity` copies bundled assets only when `PackageInfo.lastUpdateTime` differs from `files/.assets-stamp` (written after a full copy). A fatal start deletes the stamp so the next start recopies.

## Settings persistence

Rust owns `files/settings.json` (`host/src/settings.rs`); Java never reads or writes it. `run()` reads it before `load_ui`. Missing files use defaults without creating a file. Invalid JSON or a non-object is renamed to `settings.json.bad`, logged once, and replaced in memory with defaults. Read or rename errors are logged and startup continues.

Current keys are `night` (`"off"` by default, or `"on"`) and `clock24` (boolean, default `true`). `night: "auto"` is accepted and preserved when reading, but acts as Off until E3.S1. Missing or mistyped values use defaults. Unknown keys, including nested future values, survive rewrites.

The root `Host.set-setting(key, value)` callback accepts `night=off|on` and `clock24=true|false`. One writer creates `settings.json.tmp` beside the file, writes the updated object, calls `sync_all`, then atomically renames it over `settings.json`. Failed writes log an error and keep the previous in-memory value. The settings borrow is released before applying UI changes. `Host.clock24` and `HostState.clock24` expose the clock preference; `Host.host-night` exposes `HostState.night`. Night changes set `Display.night` and `HostState.night` together, reusing the root's night-change hook to reapply checked overlay and lock defaults. Clock format changes immediately, using the last `CLOCK_HHMM` value. Only `HostState.clock-text` follows the preference; `Vehicle.clock-text` always stays 24-hour `HH:MM`, so themes are unaffected. The lock screen reads numeric `HostState.clock-hhmm` (-1 means unknown), shows `--:--` while unknown, and in 12-hour mode shows h:mm with a separate AM/PM label aligned to the clock baseline.

## Keep-alive

`VehicleBridge` runs a one-second keep-alive tick, the same cadence as the stock HMI's `WakeLockNotifier`. It logs a `keep-alive` event with its reason (`touch`, `media` or `none`) when the value or the reason changes, and a `keep-alive failed` event on error. Media counts when the built-in source is playing while the vehicle is in PARK or charging, as the stock HMI does (see [plugins.md](plugins.md)). The tick runs on its own handler. Unlock service calls run on a separate worker so they cannot block it.

## Logging

Session logs live in the app's external files directory:

```
/sdcard/Android/data/dev.olatool.dash/files/logs/
  calls-<yyyyMMdd-HHmmss>.log   every vehicle callback with its arguments, plus host events
  can-<yyyyMMdd-HHmmss>.log     raw vcan0 traffic from a read-only candump
```

The folder is capped at 1 GiB. The oldest session files are deleted first. Test-channel events are prefixed `TEST`. Pull the logs with `adb pull` (see [testing.md](testing.md)). Logs contain vehicle identifiers and state history, so do not publish them unredacted.

## Home-screen swap (development only)

During development the host can replace the stock home screen. `host/test/hmi-swap.sh` disables the stock `com.olaelectric.hmiclient` package and makes the host the home activity:

```sh
bash host/test/hmi-swap.sh dash     # host becomes home
bash host/test/hmi-swap.sh stock    # stock HMI restored
bash host/test/hmi-swap.sh status
```

It needs a root adb session. There is no end-user install path yet; see the [roadmap](roadmap.md).

## Plugins

Plugins are ordinary APKs that export a service with the action `dev.olatool.dash.plugin.PLUGIN`. The host binds only plugins the owner has enabled (`files/plugins/enabled`), and plugins can publish to named slots only. They receive user actions and cannot command the vehicle. The API is in [plugins.md](plugins.md).

How the host handles them:

- `PluginHost` (main looper) reads the enablement file, discovers the service, checks its declared slots against the owner's choice, and binds it. Each plugin gets its own `IDashHost` object, so calls are attributed to the plugin. Calls from any other uid are dropped.
- Widgets, media and navigation are kept per plugin. Every change builds one JSON snapshot, which `nativeSetPlugins` hands to Rust. Rust keeps only the latest snapshot and applies it on the UI thread with the other values.
- `Media.play-pause`, `next` and `previous` go to the current media source. The plugin that published media most recently wins; otherwise the built-in `StockMediaService` does. `Plugins.widget-clicked(id)` goes back to the plugin that owns the widget as `onAction("widget.<id>")`.
- A plugin that dies or is force-stopped loses its data and is rebound with backoff.
- The keep-alive also counts media playing while in PARK or charging, as the stock HMI does. Only the built-in source counts, never a plugin's `publishMedia`.

Limits and the enablement format are in [plugins.md](plugins.md#owner-enablement).

## Build-time pieces

- `host/build.sh` cross-compiles the Rust crate for `aarch64-linux-android`, compiles the Java sources to a dex, packages the APK with the contract, overlay, fallback theme and last-resort theme as assets, and signs it. See [building.md](building.md).
- `sim` links the same Slint version (1.18.1) against the same contract, so a theme that renders in `sim` uses the same API as on the device. Renderer differences are noted in [themes.md](themes.md).
