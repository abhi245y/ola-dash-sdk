# Building

This page describes what the build needs. It does not assume a particular machine. Paths below use the defaults in `host/build.sh`, and you can override them with environment variables.

## Who can build what

| Work | Needs |
|---|---|
| Themes, `sim`, `tools/check-themes.sh`, docs | Rust (stable) and fontconfig development files. Nothing Android. |
| Host APK | Everything below (Rust, Android SDK/NDK, JDK 17) |

The vehicle-service interface comes from `sdk/vehicle-interface/`: AIDL files and enum classes written from an interoperability spec (`SPEC.md`), not copied from Ola's software. `build.sh` compiles them with the SDK's `aidl` tool. No vendor files are needed.

## Desktop tools (sim and themes)

- Rust, stable toolchain. The sim pins `slint` and `slint-interpreter` to `=1.18.1`.
- Fontconfig development files: `fontconfig-devel` on Fedora, `libfontconfig1-dev` on Debian and Ubuntu. Winit links even for the headless path.
- `libxkbcommon-dev` on Debian and Ubuntu for the live window.

```sh
cargo build --release --manifest-path sim/Cargo.toml
tools/check-themes.sh
```

## Host APK toolchain

Versions used by `host/build.sh`:

| Piece | Version | Default location (override with env) |
|---|---|---|
| JDK | 17 (Temurin used) | `$JDK_HOME`, default `~/Android/jdk17` |
| Android SDK | cmdline-tools, platform `android-34`, build-tools `34.0.0` | `$ANDROID_HOME`, default `~/Android/Sdk` |
| Android NDK | r27c, `27.2.12479018` | `$ANDROID_HOME/ndk/27.2.12479018` |
| Rust | stable, target `aarch64-linux-android` | rustup |
| Slint | `=1.18.1` (`compat-1-18`) | crate |
| Debug keystore | created on first build | `~/Android/debug.keystore` |

The JDK must include `javac`. A Java runtime alone is not enough. The NDK provides the clang linker, so `cargo-ndk` is not used. Gradle and Kotlin are not used.

## Build the APK

```sh
host/build.sh
```

The script:

1. Runs `cargo build --release --target aarch64-linux-android` and copies `liboladash.so`.
2. Copies the contract, the overlay, the default theme (as the fallback tier) and the last-resort theme into the APK assets.
3. Compiles the Java sources with `javac --release 8` and converts them with `d8`.
4. Links the manifest with `aapt2`, then adds `classes.dex` and the native library with a short Python step.
5. Aligns with `zipalign` and signs with `apksigner` using the debug key.

Output: `host/out/ola-dash-debug.apk`. The APK is debug-signed and `android:debuggable="true"`. That enables the test channel. See [safety.md](safety.md).

## Install and uninstall

These commands change the scooter. Run them only with the owner's approval and with a connected, authorized adb session.

```sh
adb install -r host/out/ola-dash-debug.apk
adb shell am start -n dev.olatool.dash/.MainActivity
adb uninstall dev.olatool.dash        # or: adb shell pm uninstall dev.olatool.dash
```

To make the host the home screen, use the swap script, which needs a root adb session. Restore the stock HMI before uninstalling:

```sh
bash host/test/hmi-swap.sh dash
bash host/test/hmi-swap.sh stock
```

See [architecture.md](architecture.md#home-screen-swap-development-only).

## Themes on the device

Themes are not built into the APK. The host copies its bundled assets on each start. A user theme lives at `files/themes/<id>/` in the app's private files directory. Copy a theme there and set `files/themes/active`. Themes need no rebuild.

## Continuous integration

`.github/workflows/ci.yml` checks the themes and sim on every push. Its `host` job runs `cargo check` for the Android target. It then builds the debug APK and uploads it as an artifact.

## Keeping build output out of git

`host/.gitignore` covers `out/`, `build/` and `target/`. `sim/.gitignore` covers `target/` and `out/`. The root `.gitignore` covers `tmp/`. Keep large artifacts out of `/tmp` on machines where `/tmp` is RAM-backed.
