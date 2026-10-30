# Roadmap

Nothing here is promised. Items are ordered roughly by how much they unblock.

## Next

- **Release flavor.** A build with `android:debuggable` off and the test channel compiled out, so "debug builds only" is true for the installed software.

## Later

- **Stock media and navigation plugins.** Reference plugins for the common media and turn-by-turn cases, so themes can be tested against real data.
- **Theme store index.** A static, signed index of community themes with the `theme.json` metadata, so the host can list and install them. No install from arbitrary URLs.
- **Settings UI.** Choose the theme, enable plugins per slot, and review the logs from the dash itself, while stopped.
- **CI release APKs.** CI already builds the debug APK; add a signed release build once the release flavor exists.

## Hardware and platform

- **Gen 2 support.** Only the Gen 1 S1 Pro is supported. Gen 2 would need its own vehicle interface and a new test pass.
- **GL renderer verification.** The sim uses the software renderer. The device uses a GL renderer. Renderer limits in [themes.md](themes.md) were checked in the software renderer only.

## Not planned

- Any vehicle command from a theme or plugin beyond the host-gated trunk request.
- Hiding the safety overlay or dropping a telltale (themes may move, reorder and restyle it).
- Unlock or ignition that bypasses the scooter's own passcode attempts, cooldowns or interlocks.
- Firmware update tooling.
