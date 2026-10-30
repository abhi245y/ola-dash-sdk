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

## Safety overlay

The overlay is compiled into the host and painted after the theme's `Dash` component. Themes can choose its placement, telltale order and style through optional `Dash` properties, but have no API to hide it, drop a telltale, or draw over it. Telltales and alerts are host-owned. Debug builds add a TEST DATA banner while injected values are active. The full model is in [safety.md](safety.md).

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
