# Theme contract reference

The contract is `contract/vehicle.slint`, currently **`contract-version: 6`**. It is the only API a theme may use. Themes read `in` properties and invoke the callbacks listed here. The host writes the `in` properties; themes must never assign to them.

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
| 6 | Optional `Dash` properties for the host overlay: `overlay-placement`, `telltale-order`, `overlay-style`, `alert-position`; enums `OverlayPlacement`, `Telltale`, `AlertPosition`; struct `OverlayStyle`; `Vehicle.trunk-open`; `Actions.open-trunk()` (gated, see below) |

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
| `contract-version` | `out int` | 6 | Read-only. The contract version the theme was compiled against. |
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
| `trunk-open` | `in bool` | false | v6. True while the seat/trunk latch is open. It closes when the seat is pressed down. Themes may ignore it. |
| `clock-text` | `in string` | "" | Preformatted clock, for example `10:24` |

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

The host implementation of `Plugins` is in progress. Until it lands, `widgets` stays empty.

## `Actions` global

Harmless actions, plus one gated vehicle request. Other vehicle commands (lock, lights, modes) are never exposed to themes. Those live in host-owned screens only.

| Member | Type | Notes |
|---|---|---|
| `open-android-auto()` | callback | Launches Android Auto |
| `open-settings()` | callback | Opens system settings |
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

## Re-exporting globals

A theme's entry file must `export { Vehicle, Media, Actions, DriveMode }` and `export component Dash`. `sim` loads the theme as the root component, so it needs these exports. The host documentation also expects `Dash` to be a plain element; see the open question in [themes.md](themes.md).

Themes import the contract as:

```slint
import { Vehicle, Media, Actions, DriveMode } from "vehicle.slint";
```

The `Battery`, `Ride`, `Navigation` and `Plugins` globals are optional for a theme. Export them from the entry file when the theme uses them.
