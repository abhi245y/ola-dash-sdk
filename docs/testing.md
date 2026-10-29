# Testing

Three kinds of checks exist: static checks on themes, deterministic screenshots from the sim, and on-device checks with the test channel and session logs.

## 1. Theme checks

```sh
tools/check-themes.sh                    # every themes/*/ with a theme.json
tools/check-themes.sh themes/minimal     # one theme
SIM=/path/to/sim tools/check-themes.sh   # use an existing sim binary
```

For each theme, `sim --check` verifies:

- `theme.json` parses and has the required keys (see [themes.md](themes.md)).
- `id` uses only `a-z`, `0-9`, `-` and matches the folder name.
- `contract` is an integer between 1 and the `contract-version` in `contract/vehicle.slint`.
- `entry` exists.
- The entry file compiles against the contract and exports `Dash`.

The script exits non-zero if any theme fails. The check does not render anything, so it does not cover visual problems. Use screenshots for those. The check also does not enforce the theme size budget; reviewers check that by hand.

## 2. Sim screenshots

The sim renders a theme headlessly with a mock clock. The mock vehicle is a pure function of time, so a given `--at` produces the same frame every run.

With `SIM_PLUGINS=1` the sim also feeds mock plugin widgets and an active navigation card, so both can be checked without a plugin installed. The built-in media source has no mock; the sim's own media mock stands in for it.

```sh
cargo run --release --manifest-path sim/Cargo.toml -- themes/default --screenshot out.png --at 75
```

The sim loop is 120 s:

- 0 to 60 s: riding. Mode cycles eco, normal, sport, hyper every 15 s. Battery falls from 100 to 60.
- 60 to 120 s: stopped and charging back to 100.

Useful times are 5 s (start), 20 s (eco, indicators on), 50 s (hyper) and 75 s (charging). Include screenshots of the default theme and of your theme when you change the look.

The live window (no `--screenshot`) runs the same mock data in real time and needs a display.

## 3. Host checks

Unit tests for the unlock state machine run on a desktop JDK without a scooter:

```sh
mkdir -p host/build-tests
javac -d host/build-tests \
    host/java/dev/olatool/dash/UnlockSession.java \
    host/java/dev/olatool/dash/TestEventGate.java host/test/UnlockSessionTest.java
java -cp host/build-tests dev.olatool.dash.UnlockSessionTest
```

These run from a plain checkout. Check `host/README.md` if the paths move.

The overlay preview renders the real host overlay with mock state, without Android:

```sh
cargo run --release --manifest-path sim/Cargo.toml --example unlock-preview -- host/out/unlock
```

## 4. On-device test channel (debug builds)

The test channel injects fake vehicle callbacks. It exists only in builds where `FLAG_DEBUGGABLE` is set. The overlay shows a TEST DATA banner while injected values are active. Injected events are written to the calls log with a `TEST` prefix. The channel turns itself off after a timeout, and `TEST_OFF` turns it off at any time.

It is for shell-driven testing only: the broadcasts need the DUMP permission, which a root adb shell has.

```sh
host/test/inject.sh motor onSpeedChanged 25   # <target> <callback method> [args...]
host/test/inject.sh off                       # end test mode
ADB_SERIAL=<serial> host/test/inject.sh ...   # when more than one device is attached
```

Targets: `display`, `motor`, `modes`, `alerts`, `info`, `commands`, `battery`, `charger`, `handlebar`. Method names are the parser callback names (see `sdk/vehicle-interface/SPEC.md`).

**Screenshots:** `adb exec-out screencap -p` returns a plain white image while the head unit's display is asleep (`dumpsys power` shows `mWakefulness=Asleep`). Wake it first with `adb shell input keyevent KEYCODE_WAKEUP`; no vehicle command is involved.

While test mode is active, real keypad submission is disabled, and real vehicle state is replayed when test mode ends. Starting the scooter is always a manual, supervised test, never an automated one.

## 5. Session logs

The host writes two kinds of logs per session:

| File | Contents |
|---|---|
| `calls-<start>.log` | Every vehicle callback with its arguments, plus host events such as keep-alive changes |
| `can-<start>.log` | Raw `vcan0` traffic from a read-only candump |

Pull them without root:

```sh
adb pull /sdcard/Android/data/dev.olatool.dash/files/logs ./logs
```

Logs contain vehicle identifiers and state history. Keep them out of git and redact before sharing. The `tmp/` folder in the repository is git-ignored for this reason. `docs/ride-log-2026-10-30.md` is an example write-up made from such logs.

## 6. Reporting a test

For a hardware test, note: the APK's git revision, the theme and its version, the steps, what the calls log shows around each event, and any visual problem with a screenshot. Mark whether the test was run on a real scooter or with injected data.
