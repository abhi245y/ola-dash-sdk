# Theme contract reference

The contract is `contract/vehicle.slint`, currently **`contract-version: 7`**. It is the only API a theme may use. Themes read `in` properties and invoke the callbacks listed here. The host writes the `in` properties; themes must never assign to them.

Units: distances in km, speed in km/h, temperatures in °C, current in A, voltage in V. `-1` means "not reported yet" wherever a default of `-1` is listed. Themes should show `--`.

## Versioning rules

> **Pre-1.0:** this is an early build. The maintainer may still make breaking changes to the contract when the design needs it; each one bumps `contract-version` and is listed in [CHANGELOG.md](../CHANGELOG.md). The rules below are what contributors follow, and what the project commits to from 1.0.

- **Additive only.** Never rename, remove or change the type of a field, enum value or callback.
- **New enum values go at the end** of the enum.
- **Bump `contract-version`** and add a line to the history comment at the top of the file.
- A theme declares the lowest contract it needs in `theme.json` (`contract`). `tools/check-themes.sh` rejects a theme that asks for a newer contract than the SDK ships.

History (from the file's comment):

| Version | Added |
|---|---|
| 1 | Original fields (`Vehicle`, `Media`, `Actions`) |
| 2 | `DriveMode.park`, `DriveMode.reverse` |
| 3 | `DriveMode.custom`, `Vehicle.hyper-available` |
| 4 | `DriveMode.easy-park` |
| 5 | `VehicleState`, `Battery`, `Ride`, `Navigation`, `Plugins`; `Vehicle.state` |
| 6 | Optional `Dash` properties for the host overlay: `overlay-placement`, `telltale-order`, `overlay-style`, `alert-position`; enums `OverlayPlacement`, `Telltale`, `AlertPosition`; struct `OverlayStyle`; `Vehicle.trunk-open`; `Actions.open-trunk()` (gated, see below). v6 also: optional `Dash` property `lock-style` for the lock screen (added within v6, see below) |
| 7 | `Display.night`; `Vehicle.data-stale`; `Actions.open-apps()`, `Actions.open-themes()`. Additive only: v6 themes load unchanged. |

## Enums

### `DriveMode`

`eco`, `normal`, `sport`, `hyper`, `unknown`, `park`, `reverse`, `custom`, `easy-park`

`unknown` is the default and is valid: render it with a neutral accent.

### `VehicleState` (v5)

| Value | Meaning |
|---|---|
| `unknown` | Not reported yet (default) |
| `sleep` | Locked, ECUs off |
| `idle` | Woken while locked |
| `park` | Unlocked, not ready |
| `drive` | Ready to ride |
| `charging` | Charger session active |

### `Regen` (v5)

`none`, `coast`, `brake`

## `Vehicle` global

| Property | Type | Default | Notes |
|---|---|---|---|
| `contract-version` | `out int` | 7 | Read-only. The contract version the theme was compiled against. |
| `speed-kmh` | `in float` | 0 | km/h |
| `battery-pct` | `in int` | -1 | Rounded state of charge, %. Use `Battery.soc-pct` for the fine value. |
| `range-km` | `in int` | -1 | Estimated range, km |
| `mode` | `in DriveMode` | `unknown` | Current drive mode |
| `hyper-available` | `in bool` | true | False when the owner's app revoked Hyper. The mode button then cycles to Custom. |
| `charging` | `in bool` | false | True during a charger session |
| `odometer-km` | `in float` | 0 | Total distance, km |
| `trip-km` | `in float` | 0 | Distance since the last unlock, km |
| `state` | `in VehicleState` | `unknown` | v5. Same as `ready-to-ride` when `drive`. |
| `left-indicator` | `in bool` | false | True while the stalk is on. The theme does the blinking. |
| `right-indicator` | `in bool` | false | As above |
| `high-beam` | `in bool` | false | Host overlay also shows this telltale; themes may ignore it |
| `headlight` | `in bool` | false | As above |
| `locked` | `in bool` | false | Vehicle lock state. Themes may ignore it; the host shows it. |
| `ready-to-ride` | `in bool` | false | Equals `state == VehicleState.drive`. Kept for v1 themes. |
| `trunk-open` | `in bool` | false | v6. True while the seat/trunk latch is open. It closes when the seat is pressed down. Themes may ignore it. Derived from `isTrunkLock` as `!locked`; that polarity is inferred from logs and not yet verified on a scooter. |
| `clock-text` | `in string` | "" | Preformatted clock, for example `10:24` |
| `data-stale` | `in bool` | false | v7. True when the vehicle parser is gone, or no vehicle callback arrived for 5 s while `state` is `drive`; the other values then show the last known state. Clears when data resumes. The host draws its own NO DATA badge; themes may add a hint or ignore it. |

## `Display` global (v7)

| Field | Type | Default | Notes |
|---|---|---|---|
| `night` | `in bool` | false | True when it is dark. Not set by the host yet (E3.S1). Themes may switch to a night palette or ignore it. |

## `Battery` global (v5)

All fields are `in`.

| Property | Type | Default | Notes |
|---|---|---|---|
| `soc-pct` | float | -1 | Fine-grained state of charge, % |
| `current-a` | float | 0 | Pack current in A. Negative while discharging. |
| `voltage-v` | float | -1 | Pack voltage, V |
| `temp-min-c` | float | -1 | Coldest cell, °C |
| `temp-max-c` | float | -1 | Hottest cell, °C |
| `health-pct` | int | -1 | State of health, % |

## `Ride` global (v5)

All fields are `in`.

| Property | Type | Default | Notes |
|---|---|---|---|
| `front-brake` | bool | false | Front brake lever engaged |
| `rear-brake` | bool | false | Rear brake lever engaged |
| `regen` | `Regen` | `none` | Active regenerative mode |
| `motor-temp-c` | float | -1 | Motor temperature, °C |
| `mcu-temp-c` | float | -1 | Controller temperature, °C |

## `Media` global

| Property | Type | Notes |
|---|---|---|
| `title` | `in string` | Track title |
| `artist` | `in string` | Artist |
| `playing` | `in bool` | Playback state |
| `play-pause()` | callback | Toggle playback |
| `next()` | callback | Next track |
| `previous()` | callback | Previous track |

## `Navigation` global (v5)

Turn-by-turn from a navigation plugin. Empty strings when inactive.

| Property | Type | Notes |
|---|---|---|
| `active` | `in bool` | True while guidance runs |
| `instruction` | `in string` | For example `Turn left onto Main St` |
| `distance` | `in string` | Preformatted, for example `400 m` |
| `eta` | `in string` | Preformatted, for example `10:42` |

## `Plugins` global (v5)

| Member | Type | Notes |
|---|---|---|
| `widgets` | `in [Widget]` | Value cards published by plugins |
| `widget-clicked(string)` | callback | Tells the owning plugin its widget was tapped. The argument is the widget `id`. |

### `struct Widget` (v5)

| Field | Type | Notes |
|---|---|---|
| `id` | string | Unique within its plugin |
| `title` | string | Label |
| `value` | string | Preformatted value |
| `unit` | string | Unit text, may be empty |

The host fills `widgets` from enabled plugins (see [plugins.md](plugins.md)); it is empty when no plugin publishes.

## `Actions` global

Harmless actions, plus one gated vehicle request. Other vehicle commands (lock, lights, modes) are never exposed to themes. Those live in host-owned screens only.

| Member | Type | Notes |
|---|---|---|
| `open-android-auto()` | callback | Launches Android Auto |
| `open-settings()` | callback | Opens system settings. The host may ignore it unless the scooter is parked. |
| `open-apps()` | callback | v7. Asks the host to open its apps screen. The host ignores it unless the scooter is parked; it only logs the call yet. |
| `open-themes()` | callback | v7. Asks the host to open its themes screen. The host ignores it unless the scooter is parked; it only logs the call yet. |
| `open-trunk()` | callback | v6. Asks the host to open the seat/trunk, like the stock trunk button. See below. |

### `open-trunk()` (v6)

The host sends the request only when all of these hold, using real vehicle values, never test data:

- The BCM reports the scooter unlocked.
- `state` is `park`.
- Speed is 0 km/h. An unknown speed is refused.
- The trunk latch is known to be closed (`trunk-open` is false). An unknown latch is refused.
- The host is not in test mode, and the vehicle parser is connected.
- The previous request was sent more than 3 s ago.

Otherwise nothing happens. The theme gets no result, so show nothing the rider could mistake for "trunk opening". Each call is logged in the calls log as `trunk request: sent` or `trunk request: refused <reason>`. The host sends one command per request and does not retry or wait for an acknowledgement. If the command is lost, nothing happens. See [safety.md](safety.md#the-one-vehicle-request).

## Host overlay settings (v6)

A theme may declare these four properties on its `Dash` component. They are **optional**: leave one out and the host uses its default. They are `out` properties; the host reads them once the theme loads and re-reads them when they change.

| Property | Type | Default | Notes |
|---|---|---|---|
| `overlay-placement` | `out OverlayPlacement` | `top` | Where the telltale strip sits. See [themes.md](themes.md#overlay-placement-and-the-reserved-strip). |
| `telltale-order` | `out [Telltale]` | host order | Order of the telltales in the strip. |
| `overlay-style` | `out OverlayStyle` | host dark style, labels off | Strip background, label colour, corner radius, labels on or off. |
| `alert-position` | `out AlertPosition` | `center` | Vertical position of alert cards. |

The host never lets these hide, shrink or cover a warning. Rules that apply regardless of theme are in [safety.md](safety.md#host-owned-safety-overlay).

### `OverlayPlacement` (v6)

`top`, `bottom`, `left`, `right`. Reserved strip: top and bottom 48 px tall, full width; left and right 72 px wide, full height.

### `Telltale` (v6)

`left-indicator`, `right-indicator`, `high-beam`, `headlight`, `ready`, `locked`, `side-stand`, `fault`, `charging`. The host default order is the order listed here. New values are appended.

The host normalises the theme's list: unknown and duplicate entries are ignored, and any telltale the theme leaves out is appended in default order. A theme can reorder telltales but cannot drop them.

### `AlertPosition` (v6)

`center`, `upper`, `lower`.

### `OverlayStyle` (v6)

| Field | Type | Notes |
|---|---|---|
| `background` | color | Strip background. Must contrast with `foreground` at 4.5:1 or more. |
| `foreground` | color | Label colour |
| `corner-radius` | length | Telltale disc corner radius |
| `show-labels` | bool | Labels under or beside each telltale |

If `background` and `foreground` fail the 4.5:1 contrast rule (WCAG 2.x), the host uses its own colours and logs the reason under `OlaDash` (`sim` prints it to stderr). Telltale icon colours are host-fixed and do not come from the theme.

## Lock screen style (v6)

A theme may declare one more optional `out` property on `Dash`: `lock-style`. It styles the passcode screen (the screen the host shows while the scooter is locked or unlocking). It is **style data only**. The host draws the keypad, handles every key press, and owns the layout, key order, labels, copy and phase logic. A theme never gets a callback, text or a layout from the lock screen. Absent means the host default style. This was added within v6, so `contract-version` stays 6. See [themes.md](themes.md#lock-screen) and [safety.md](safety.md#the-lock-screen-is-styled-never-handled).

| Property | Type | Default | Notes |
|---|---|---|---|
| `lock-style` | `out LockStyle` | host default look | Colours, key shape and digit drawing. All fields are required when the struct is written (use `#00000000` only where the table below allows it). |

### `LockStyle` (v6)

| Field | Type | Rule |
|---|---|---|
| `background` | color | Opaque. Foreground must reach 4.5:1 on it. |
| `foreground` | color | Keypad digits, title, status. 4.5:1 on background. |
| `muted` | color | Secondary copy, including attempt and cooldown lines. 4.5:1 on background. |
| `accent` | color | Filled passcode cells, battery fill, LOCKED. 3:1 on background. |
| `key-background` | color | Opaque, 4.5:1 with `foreground`. `#00000000` means the screen background. |
| `key-border` | color | Hairlines and key outlines. Decorative, not checked. |
| `key-shape` | `LockKeyShape` | `rounded`, `square` or `grid`. |
| `digits` | `LockDigits` | `sans` (font) or `segment` (the host's seven-segment drawing). |

### `LockKeyShape` (v6)

`rounded` (12 px corners, 1 px outline), `square` (4 px corners, 1 px outline), `grid` (no corners, no outline, 1 px hairlines between keys).

### `LockDigits` (v6)

`sans` draws digits with the host font (`OlaDash Host`), not the theme's. `segment` draws them with the host's seven-segment shapes in the foreground colour.

### Host checks

When a theme's `lock-style` is read, the host checks it and logs each rejection under `OlaDash` (`sim` prints it to stderr). A rejected value falls back to a host colour that passes the same rule, or to the foreground:

- Background and foreground fail together. If the pair is below 4.5:1, or either is transparent, both become host colours.
- Muted and accent each fall back on their own rule.
- A transparent `key-background` uses the screen background.
- An unknown `key-shape` becomes `rounded`. An unknown `digits` value becomes `sans`.
- The error copy uses amber when it reads on the background, otherwise the foreground.

The keypad's size and position are fixed by the host (about 155 x 97 px per key, above the 96 x 80 px minimum), so a theme cannot make keys smaller. There are no per-key colours, labels or callbacks.

## Re-exporting globals

A theme's entry file must `export component Dash inherits Window`. The host and `sim` wrap it in the same generated root, which imports the contract and re-exports its globals, so a theme does not need to export them (re-exporting them is harmless).

Themes import the contract as:

```slint
import { Vehicle, Media, Actions, DriveMode } from "vehicle.slint";
```

The `Battery`, `Ride`, `Navigation` and `Plugins` globals are optional for a theme. Export them from the entry file when the theme uses them.
