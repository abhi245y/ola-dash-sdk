# host/ - Android host app for the Ola dash

Rust + Slint (runtime-loaded themes) with a small Java bridge that binds the
stock vehicle parser for telemetry and keep-alive, and the stock profile service
for authenticated passcode unlock. Package `dev.olatool.dash`,
minSdk 29, arm64-v8a only.

## Build (home PC only)

Build on a Linux machine with the Android SDK, never on a laptop you use for other things. Sync this repo, then build:

```sh
rsync -a --delete --exclude .git --exclude tmp --exclude host/out --exclude host/build \
    --exclude host/target --exclude sim/target --exclude sim/out \
    ./ build-host:Documents/ola-dash-host/
ssh build-host 'cd ~/Documents/ola-dash-host/host && ./build.sh'
scp build-host:Documents/ola-dash-host/host/out/ola-dash-debug.apk host/out/
```

The sync must include `private/` (see below). `build.sh` runs:
`cargo build --release --target aarch64-linux-android`, `aidl --lang=java` (plugin
API, see below), `javac` (release 8), `d8`, `aapt2 link`, a small python step that
adds `classes.dex` and the `.so`, `zipalign`, `apksigner` (debug key at
`~/Android/debug.keystore`). Output: `host/out/ola-dash-debug.apk`. `host/out/`,
`host/build/` and the repo's `private/` are gitignored.

### Vendored vehicle interface (`private/`, not in git)

The AIDL stubs and enums of the stock vehicle service (`com.olaelectric.vehicleinterface`)
are decompiled from the stock app and cannot be redistributed, so they live in
`private/ola-interface/java/com/olaelectric/vehicleinterface/`. `build.sh` stops with
a message if that folder is missing. Restore it from your own decompile of the stock
`HVDP` tree with these rules:

- Keep package names, descriptors and transaction codes unchanged.
- Callback interfaces use `default` no-op methods that call
  `dev.olatool.dash.Trace.call(...)` (so every callback lands in the ride log). Only
  the methods the bridge uses are overridden.
- `ICommanderService` is included for keep-alive; the `ProximityParserKt` constant (201)
  is inlined.

The vendored sources compile in the same `javac` run as `host/java`, because the
defaults reference `Trace`.

### Plugin AIDL

`sdk/plugin-api/aidl/` is compiled with build-tools `aidl --lang=java` into
`build/gen/` and shipped in the dex. `build.sh` compiles a copy of the files: aidl
without `--lang=java` rewrites its inputs in place. `PluginHost` (package
`dev.olatool.dash.plugin`) implements `IDashHost` and binds plugins through the
generated `IDashPlugin`. Enablement, limits and the media listener grant are in
[docs/plugins.md](../docs/plugins.md).

### Toolchain (versions used)

| Piece | Version | Where |
|---|---|---|
| JDK | Temurin 17.0.21 | `~/Android/jdk17` (a system Java 25 runtime without `javac` is not enough) |
| Android cmdline-tools | 13114758 (sdkmanager 19.0) | `~/Android/Sdk` |
| platform | android-34 | `~/Android/Sdk/platforms` |
| build-tools | 34.0.0 | `~/Android/Sdk/build-tools` |
| NDK | r27c, 27.2.12479018 | `~/Android/Sdk/ndk` (clang linker, no cargo-ndk) |
| Rust | 1.99.1, target aarch64-linux-android | preinstalled |
| slint, slint-interpreter | =1.18.1 (`compat-1-18`) | crates |
| android-activity | 0.6.1 | crate |
| jni | 0.22.4 | crate |
| slint-compiler | 1.18.1 (`--no-default-features`) | `~/.cargo/bin`, syntax-checks `.slint` |

No Gradle, Kotlin, or cargo-ndk. Slint's Android backend always links Skia
(`i-slint-renderer-skia` is not optional), so the `.so` is about 23 MB.

## Install / uninstall (commands only, not run)

```sh
adb install -r host/out/ola-dash-debug.apk     # needs user approval
adb shell am start -n dev.olatool.dash/.MainActivity
adb uninstall dev.olatool.dash                  # or: adb shell pm uninstall dev.olatool.dash
```

## Passcode unlock and physical ignition

The host owns the passcode screen, above every theme. A vehicle passcode-window
event or locked state opens the keypad. It accepts the existing six-digit scooter
passcode, including leading zeroes. Digits stay in Java: Rust/Slint receives only
the count. The app does not persist or log the PIN.

Unlock uses `com.olaelectric.profilemanagerservice/.services.ProfileManagerService`,
calling the stock `challengeUnlockByPasscode(pin, false)` transaction. That service
checks the PIN, enforces its attempt limit and requests unlock. The app does not
send a separate direct-unlock command. Remaining attempts and the stock five-minute
cooldown are read from the service; reconnects and app restarts do not reset them.

PIN acceptance shows **Unlocking**. Only a real vehicle `PARK` event shows
**PARKED**. Press the handlebar power button to enter DRIVE using the scooter's
existing controls and interlocks; a real `DRIVE` event reveals the dashboard and
READY indicator. This test build adds no touchscreen ignition command or mode
selector. It displays the reported mode; use the scooter's default Normal mode
for initial testing. A service error, missing state, or timeout never fabricates
a successful unlock or DRIVE state. An unresolved in-flight verification is not
automatically retried.

The debug injection channel cannot authenticate or change this unlock state.
While debug test mode is active, real keypad submission is disabled. Real lock
and state values are replayed when debug mode ends. During unlock and PARK, alerts
remain visible in a footer so a side-stand alert cannot cover the keypad.

Validation without a scooter, on the build machine:

```sh
mkdir -p host/build-tests
~/Android/jdk17/bin/javac -d host/build-tests \
    host/java/dev/olatool/dash/UnlockSession.java \
    host/java/dev/olatool/dash/TestEventGate.java host/test/UnlockSessionTest.java
~/Android/jdk17/bin/java -cp host/build-tests dev.olatool.dash.UnlockSessionTest
cargo run --release --manifest-path sim/Cargo.toml --example unlock-preview -- host/out/unlock
```

The preview uses the shipped theme and overlay with mock state, without Android,
Binder, or a vehicle connection. Desktop Slint requires the host's normal desktop
font/window libraries; the Android APK build does not depend on them.

For the first manual test, keep the laptop connected. Exit test mode, wake with
the handlebar button, enter the correct PIN, wait for PARKED, then use the physical
power button and check DRIVE/Normal feedback. Starting the scooter is a manual
test, not part of the automated checks. Special side-stand override prompts are
not ported in this build. Restore stock HMI from the repository root if needed:

```sh
bash host/test/hmi-swap.sh stock
```

## Themes

- Theme folder: `files/themes/<name>/` in the app's private files dir
  (`/data/data/dev.olatool.dash/files/themes/<name>/`).
- Active theme: the name in `files/themes/active` (default `default`).
- `theme.json` manifest (required): `id`, `name`, `version`, `author`, `license`,
  `contract` (integer, the lowest contract version the theme needs) and `entry`
  (the Slint file to load, inside the theme folder). The host rejects a theme with
  `contract` above its own (`HOST_CONTRACT` in `src/lib.rs`, currently 5), a missing
  `contract` or `entry`, or an `entry` that is absolute, contains `..`, or does not
  exist. A rejected theme logs the reason under `OlaDash` and the fallback tier loads.
- The entry file must export `export component Dash inherits Window`. The host
  embeds it in its own root window and binds `default-font-family` from it, so keep
  the `Window` base. The host re-exports `Vehicle`, `Battery`, `Ride`, `Media`,
  `Navigation`, `Plugins`, `Actions` and `HostState` from its root. It imports the contract as
  `import { Vehicle, Battery, Ride, Media, Actions, DriveMode } from "vehicle.slint";`.
  The host supplies the include path, so no copy of the contract is needed.
- Unknown values: `-1` means "not reported yet" for `Vehicle.battery-pct`,
  `Vehicle.range-km` and the `Battery` and `Ride` temperatures and health. Themes show
  `--` for those.
- The root window is generated at runtime (`files/host/root.slint`): `Dash`
  fills the window, then the host overlay (`ui/overlay.slint`) is drawn on top.
  Leave the top 48 px empty; the host telltale bar owns it.
- Tiers: active theme -> bundled `fallback` (copy of `themes/default`) ->
  bundled `last-resort` placeholder (`ui/last-resort/main.slint`). Each failed
  compile or create logs its diagnostics under logcat tag `OlaDash` and moves
  to the next tier. The app does not crash.

Bundled assets (`contract/vehicle.slint`, `ui/overlay.slint`, `themes/default` as fallback
with its `theme.json`, `ui/last-resort/main.slint`) are copied from the APK to `files/`
on every start.

### Test channel (debug build)

`test/inject.sh <cb> <method> [args]` runs one parser callback as if the vehicle had
sent it. `cb`: `display|motor|modes|alerts|info|commands|battery|charger|handlebar`.
`inject.sh off` ends test mode and replays the real parser state. Test mode hides
the passcode screen; it never sends commands or changes unlock state.

## Source layout

- `src/lib.rs`: Slint runtime, theme loader, JNI entry `VehicleBridge.nativeSet`,
  Open Headunit / Settings launch via JNI.
- `ui/overlay.slint`: host-owned telltale bar and alert cards.
- `ui/last-resort/main.slint`: placeholder used only when the default theme fails.
- `java/dev/olatool/dash/`: `MainActivity` (NativeActivity, asset copy, clock),
  `VehicleBridge` (telemetry, keep-alive, callbacks, alert logic), `ProfileService`
  (small stock PMS Binder client), `UnlockController` (service lifecycle, worker
  calls, state feedback) and `UnlockSession` (offline-testable unlock state).
- `java/dev/olatool/dash/plugin/`: `PluginHost` (plugin binding, `IDashHost`, JSON
  snapshot to Rust) and `StockMediaService` (built-in media session source).
- `private/ola-interface/java/com/olaelectric/vehicleinterface/`: vendored stock
  AIDL stubs and enums (not in git, see above).
- `sdk/plugin-api/aidl/`: plugin API, compiled by `build.sh` (see above).
