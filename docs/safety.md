# Safety

This project changes what a rider sees on a vehicle. The rules below apply to the code, to themes and plugins, and to how the software is used. They are commitments, not suggestions.

## Use

- **Riding with experimental software is at your own risk.** The dash is in field testing. It can show wrong values, freeze, or disappear. Keep the stock dash working for anything that matters, and know how to restore it (see [building.md](building.md#install-and-uninstall)).
- **Glanceable information only while riding.** Do not read long text, navigate through a menu, pair devices, change settings or watch media while moving. Set these up while stopped.
- **Do not rely on the dash for warnings.** The host shows telltales and alerts, but the scooter's own indicators and the stock dash remain the reference. If the dash and the scooter disagree, trust the scooter and stop.

## Host-owned safety overlay

- The overlay (telltales, alert cards, the unlock screen while locked) is drawn by the host after the theme, above every theme.
- No theme API can hide, move, resize, or draw over it.
- Plugins cannot cover it either.
- Themes can choose where the telltale strip sits (`overlay-placement`), in which order the telltales appear, the strip's colours and labels, and where alert cards sit. See [contract.md](contract.md#host-overlay-settings-v6).

### What a theme can change (v6)

- Strip placement: top, bottom, left or right.
- Telltale order. Every telltale stays on the strip.
- Strip background, label colour and corner radius, and label visibility.
- Alert card vertical position.

### What a theme cannot change

- Whether the overlay is shown. It is always drawn, above the theme.
- Telltale and alert colours. Semantic colours (green indicators and ready, blue high beam, amber side stand, red fault and alert frames) are fixed by the host.
- Minimum sizes: telltale icons at least 28 px, alert text at least 32 px.
- Contrast. A background and foreground pair below 4.5:1 is replaced by the host default.
- Icons. Themes cannot supply their own telltale icons.
- Dismissal. Fall or accident, tow or theft, side stand and steering lock cannot be dismissed. Fault and low battery can be tapped away, as before.
- The TEST DATA badge, which is shown in every placement in debug builds.
- The alert scrim dims the whole screen, but the telltale strip is drawn above it, so telltales stay at full brightness during an alert.
- Placement is clamped: no overlay element goes off screen.
- The overlay shows the TEST DATA banner in debug builds while injected values are active. A release build never shows injected data.

If a theme or plugin ever hides the overlay, that is a critical bug. Report it under [SECURITY.md](../SECURITY.md).

## No vehicle commands from themes or plugins

- The contract exposes read-only vehicle values, harmless callbacks (media control, Android Auto, settings) and one gated vehicle request, `Actions.open-trunk()`. See [contract.md](contract.md).
- Other vehicle commands such as lock, lights and modes are never part of the contract. They live in host-owned screens only.
- Plugins are display-only. The plugin API has no method that commands the vehicle. See [plugins.md](plugins.md).

### The one vehicle request

`Actions.open-trunk()` is the only vehicle command a theme can reach. It sends the stock trunk command (`Seat_Unlock_Cmd`) once, and only when every guard passes:

| Guard | Source |
|---|---|
| Scooter unlocked (BCM) | Real BCM lock callback only |
| Vehicle state `park` | Real vehicle-state callback only |
| Speed 0 km/h | Real speed; unknown is refused |
| Trunk latch known closed | Real `isTrunkLock` callback; unknown is refused |
| Not in test mode | Test values never reach the gate |
| Commander connected | Parser binder present |
| No repeat within 3 s | Time since the last request that was sent |

Every request is logged as `trunk request: sent` or `trunk request: refused <reason>`. The host sends the command once. It does not retry and does not wait for an acknowledgement, so a lost command means the trunk does not open and the rider uses the stock control. Rules are in `host/java/dev/olatool/dash/TrunkGate.java`, tested by `host/test/TrunkGateTest.java`.

## Plugins require the owner

- A plugin is bound only after the owner enables it.
- Enablement is per slot. Installing an APK grants nothing.

## Test channel

- The test channel injects fake vehicle state and is compiled in only for debug builds (`FLAG_DEBUGGABLE`).
- It shows the TEST DATA banner while active and turns off after a timeout.
- It cannot submit an unlock PIN, and real vehicle state is replayed when it ends.
- **The current APK is debug-signed and sets `android:debuggable="true"` in `host/AndroidManifest.xml`.** That means the test channel is present in the only APK that exists. A release build with `debuggable` off is required before "debug builds only" is true for the installed software. Until then, treat any installed build as a test build.

## Unlock module (optional)

- Covers the dash only while locked or authenticating (connecting, locked, checking the passcode, waiting for unlock). From PARK on, the screen is hidden and the theme shows the dash. The host shows no PARK or side-stand text of its own; the host's side-stand telltale and alert card still apply.
- Off unless the owner uses it.
- Requires the owner's existing six-digit passcode.
- The PIN is passed to Ola's own profile service (`challengeUnlockByPasscode`), which checks it, enforces its attempt limit, and requests the unlock.
- The module does not send a direct unlock command, does not bypass the service's attempt limit or its five-minute cooldown, and does not store or log the PIN.
- Ignition is still a manual action on the scooter's own controls and interlocks. The dash only shows the state it reports.
- This project does not sign in to any account, create accounts, or change credentials. Do not add code that does.

## Firmware and updates

- The project does not install, trigger, or stage vehicle firmware updates, and does not touch the vehicle's update or over-the-air paths.
- Contributions that do must be rejected.

## For contributors

- Do not add Ola's proprietary files, passcodes, keys, or account data.
- Do not weaken the overlay, the test-channel gate, or the owner-enablement rule without a maintainer review and a hardware test note.
- Treat any change to the bridge, the overlay, or the unlock module as safety-relevant and say so in the PR.
