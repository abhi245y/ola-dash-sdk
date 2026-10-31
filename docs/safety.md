# Safety

This project changes what a rider sees on a vehicle. The rules below apply to the code, to themes and plugins, and to how the software is used. They are commitments, not suggestions.

## Use

- **Riding with experimental software is at your own risk.** The dash is in field testing. It can show wrong values, freeze, or disappear. Keep the stock dash working for anything that matters, and know how to restore it (see [building.md](building.md#install-and-uninstall)).
- **Glanceable information only while riding.** Do not read long text, navigate through a menu, pair devices, change settings or watch media while moving. Set these up while stopped.
- **Do not rely on the dash for warnings.** The host shows telltales and alerts, but the scooter's own indicators and the stock dash remain the reference. If the dash and the scooter disagree, trust the scooter and stop.

## Host-owned safety overlay

- The overlay (telltales, alert cards, the unlock screen while locked) is drawn by the host after the theme, above every theme.
- The hub button is host-owned: a fixed 64 px end cell along the strip (64 x 48 px horizontal, 72 x 64 px vertical), with no telltale moved or shrunk. It opens host settings only while parked, unlocked, with no alert and outside test mode. When data is stale, NO DATA wins and the hub hides; TEST DATA also hides the hub, so badges never overlap it.
- Interactive host screens sit above the theme and below the overlay. Their navigation gate lives in the host root, using `HostState.parked` from `TrunkGate`; leaving parked, an alert, locking or test mode closes them before the next frame. They use host state and `HostText`, like the overlay.
- No theme API can hide it, shrink it below the host minimums, drop a telltale, or draw over it. Themes may choose its placement, order and style (next point); the host still draws it.
- Plugins cannot cover it either.
- Themes can choose where the telltale strip sits (`overlay-placement`), in which order the telltales appear, the strip's colours and labels, and where alert cards sit. See [contract.md](contract.md#host-overlay-settings-v6).

### What a theme can change (v6)

- Strip placement: top, bottom, left or right.
- Telltale order. Every telltale stays on the strip.
- Strip background, label colour and corner radius, and label visibility.
- Alert card vertical position.

### What a theme cannot change

- Whether the overlay is shown. It is always drawn, above the theme.
- Telltale and alert colours. Semantic colours (green indicators and ready, blue high beam, amber side stand, red fault and alert frames) are fixed by the host. Lit telltales are solid glyphs in those colours; unlit ones are 40 % outlines of the strip foreground, so every telltale is always visible. The strip is dark in day and night (48 px top or bottom, 72 px left or right) and carries warning lights only: the clock and battery show on the lock screen, not in the strip.
- Minimum sizes: telltale icons at least 28 px, alert text at least 32 px.
- Contrast. A strip background and foreground pair below 4.5:1, or a background on which a lit telltale colour is below 3:1, is replaced by the host default.
- Icons. Themes cannot supply their own telltale icons.
- What the overlay shows. It reads host-owned copies (`HostState.left-indicator`, `locked`, `clock-text`, `battery-pct` and the rest of the telltale values), not `Vehicle.*`. A theme can write `Vehicle.*`, and that changes only the theme's own view.
- The font of host text. The overlay, alert cards, lock screen and host screens use the host's font family `OlaDash Host`. `sim --check` refuses a theme that ships a font file carrying that name, so a theme cannot swap in blank or look-alike glyphs.
- Dismissal. Fall or accident, tow or theft, side stand and steering lock cannot be dismissed. Fault and low battery can be tapped away, as before.
- The TEST DATA badge, which is shown in every placement in debug builds, and the NO DATA badge (red, same slot, hidden while TEST DATA shows) shown when vehicle data is stale: the parser is gone, or no vehicle data callback arrived for 5 s while in drive.
- Alert cards are the same in day and night: a white card on a 60 % scrim, a red frame (fall, tow, steering lock, fault) or an amber frame (side stand, low battery), an icon, a 32 px title and a one-line hint.
- The alert scrim dims the whole screen, but the telltale strip is drawn above it, so telltales stay at full brightness during an alert.
- Placement is clamped: no overlay element goes off screen.
- The overlay shows the TEST DATA banner in debug builds while injected values are active. A release build never shows injected data.

If a theme or plugin ever hides the overlay, that is a critical bug. Report it under [SECURITY.md](../SECURITY.md).

## The lock screen is styled, never handled

- The passcode keypad and the PIN are host-owned. The host draws every key, the passcode cells, the attempt and cooldown copy, and the keypad layout. Its key presses go to the host's unlock controller, not to the theme.
- The lock screen shows the clock, date and battery on the left and the caption, six passcode cells and the keypad on the right; it has no lock icon or "Locked" text, because the strip shows lock and charging. While a warning is active, its card stays in the left column, so it never hides behind the keypad.
- A theme supplies only `lock-style` (see [contract.md](contract.md#lock-screen-style-v6)): colours, key shape and digit drawing. It has no callback, text or layout for the keypad, and it cannot read the digits entered. The contract exposes only the count of digits, and only to the host.
- The host checks the style's contrast before use. A theme cannot make the attempt count, cooldown or error copy unreadable (these are in the `muted` and error colours, which are checked) or make the keys too small to press.
- `host/test/unlock-preview.rs` renders the host overlay with the default theme and checks that a tap on a key dispatches the key through the host overlay, with the expected digit, Delete and Clear codes.

## One gated vehicle request, nothing else

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

Every request is logged as `trunk request: sent` or `trunk request: refused <reason>`. The host sends the command once. It does not retry and does not wait for an acknowledgement, so a lost command means the trunk does not open and the rider uses the stock control. Rules are in `host/java/dev/olatool/dash/TrunkGate.java`, tested by `host/test/TrunkGateTest.java`. The park rule is also exposed as `HostState.parked` (unlocked and parked and stopped, or unlocked and charging, never in test mode), and host screens use the same gate.

## Plugins require the owner

- A plugin is bound only after the owner enables it.
- Enablement is per slot. Installing an APK grants nothing.

## Test channel

- The test channel injects fake vehicle state and is registered only in debug builds: the code is in every APK, but `MainActivity` checks `FLAG_DEBUGGABLE` at runtime.
- It shows the TEST DATA banner while active and turns off after a timeout.
- It cannot submit an unlock PIN, and real vehicle state is replayed when it ends.
- **The default `host/build.sh` APK is debug-signed and debuggable (`aapt2 link --debug-mode`), so it has the test channel. `host/build.sh --release` builds are not debuggable and never register it; the build fails if the APK is debuggable.** Treat any build you did not make with `--release` as a test build.

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
