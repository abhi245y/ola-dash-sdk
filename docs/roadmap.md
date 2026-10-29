# Roadmap

Nothing here is promised. Items are ordered roughly by how much they unblock.

## Next

- **Clean-room AIDL for the vehicle interface.** Replace the vendor interface classes with our own AIDL, written from observed behaviour rather than copied, so the public build works from a clean checkout. This is the main blocker for outside contributors.
- **Release flavor.** A build with `android:debuggable` off and the test channel compiled out, so "debug builds only" is true for the installed software.
- **Plugin host implementation.** Bind enabled plugins, implement `IDashHost`, route `onAction`, and feed `Plugins.widgets`, `Media` and `Navigation` from plugins. Spec: [plugins.md](plugins.md).

## Later

- **Stock media and navigation plugins.** Reference plugins for the common media and turn-by-turn cases, so themes can be tested against real data.
- **Theme store index.** A static, signed index of community themes with the `theme.json` metadata, so the host can list and install them. No install from arbitrary URLs.
- **Settings UI.** Choose the theme, enable plugins per slot, and review the logs from the dash itself, while stopped.
- **CI APK builds.** Build and sign the release APK in CI once the vendor classes are replaced.

## Hardware and platform

- **Gen 2 support.** Only the Gen 1 S1 Pro is supported. Gen 2 would need its own vehicle interface and a new test pass.
- **GL renderer verification.** The sim uses the software renderer. The device uses a GL renderer. Renderer limits in [themes.md](themes.md) were checked in the software renderer only.

## Not planned

- Any vehicle command from a theme or plugin.
- Hiding or reordering the safety overlay.
- Unlock or ignition that bypasses the scooter's own passcode attempts, cooldowns or interlocks.
- Firmware update tooling.
