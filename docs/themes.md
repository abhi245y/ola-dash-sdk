# Themes

A theme is a folder `themes/<id>/` with a `main.slint` entry point and a `theme.json` manifest. The host loads it at runtime with `slint-interpreter`, so a theme needs no Rust code and no rebuild. `themes/default/` is the full example. `themes/minimal/` is the smallest theme that uses every contract field.

Read [contract.md](contract.md) first. Themes see only that API.

## Manifest: `theme.json`

```json
{
  "id": "minimal",
  "name": "Minimal",
  "version": "1.0.0",
  "author": "Ola Dash SDK contributors",
  "license": "GPL-3.0-or-later",
  "contract": 5,
  "entry": "main.slint",
  "description": "Plain text layout. Reference for the smallest working theme."
}
```

| Key | Type | Rule |
|---|---|---|
| `id` | string | Required. Only `a-z`, `0-9`, `-`. Must equal the folder name. |
| `name` | string | Required. Display name. |
| `version` | string | Required. Theme version, for example `1.0.0`. |
| `author` | string | Required. Name or handle. |
| `license` | string | Required. An SPDX identifier. Themes in this repository are `GPL-3.0-or-later`. |
| `contract` | integer | Required. The lowest contract version the theme needs. Must be at least 1 and no higher than the SDK's `contract-version`. |
| `entry` | string | Required. Path to the entry file, relative to the theme folder. Usually `main.slint`. |
| `description` | string | Required. One sentence. |

`tools/check-themes.sh` checks all of these. A theme that asks for a contract newer than the SDK ships fails the check.

## Entry file

```slint
import { Vehicle, Media, Actions, DriveMode } from "vehicle.slint";
export { Vehicle, Media, Actions, DriveMode }   // required

export component Dash inherits Window {
    width: 1280px;
    height: 768px;
    background: #05070b;
    // ...
}
```

- The root component is exported as `Dash`, sized 1280 x 768 logical px (landscape).
- **Re-export the globals (for `sim`).** `sim` loads the theme file as the root component and sets `Vehicle.*` and `Media.*` on that instance. slint-interpreter only exposes globals the main file exports, so `sim` needs the `export { ... }` line; without it the values never arrive. The host wraps the theme and re-exports the globals itself, so the export line is not what the device depends on. Keep it in every theme.
- Import the contract as `"vehicle.slint"`. The host and `sim` put `contract/` on the include path, so no copy is needed.

> **Open question:** the bundled themes use `export component Dash inherits Window`, and `sim` accepts that. `host/README.md` says the host generates its own root window and that `Dash` must **not** inherit `Window`. Only the bundled themes were tested in `sim`; a plain-element `Dash` was not tried. Until the host and sim agree, follow the bundled themes and expect the host path to need a change.

## Overlay placement and the reserved strip

The host draws the telltale strip over every theme. Its place depends on the theme's optional `overlay-placement` (contract v6, see [contract.md](contract.md#host-overlay-settings-v6)). The default is `top`.

| Placement | Reserved strip | Notes |
|---|---|---|
| `top` | top 48 px, full width | Default. The minimal theme uses it. |
| `bottom` | bottom 48 px, full width | |
| `left` | left 72 px, full height | The default theme uses it. |
| `right` | right 72 px, full height | |

- Keep the reserved strip as plain background. The host paints its own background there, so nothing drawn by the theme shows through. Still, do not place anything important under it: a theme that does is hard to read and will be partly hidden.
- Telltales and alert cards use the area outside the strip. Alert cards are centred there (or placed by `alert-position`).
- A theme that declares no placement gets `top`, as in contract 5.
- The theme's own layout is still yours. If a theme declares `bottom` or `right`, check that its controls stay clear of the strip.

## Telltale order and style

```slint
out property <OverlayPlacement> overlay-placement: OverlayPlacement.left;
out property <[Telltale]> telltale-order: [Telltale.left-indicator, Telltale.right-indicator, Telltale.ready];
out property <OverlayStyle> overlay-style: {
    background: #05070b,
    foreground: #f4f6fa,
    corner-radius: 16px,
    show-labels: true,
};
out property <AlertPosition> alert-position: AlertPosition.upper;
```

Import the types from `vehicle.slint`. The host checks the rest:

- **Order:** the list may be partial. Unknown entries and duplicates are dropped, and missing telltales are appended in host order.
- **Contrast:** if `background` and `foreground` are below 4.5:1 (WCAG 2.x), the host uses its own colours and logs why.
- **Semantic colours and icon sizes** are host-fixed. Labels take the theme's `foreground`.
- **Wrong types** make the theme fail to compile, so it falls back to the bundled default, as any compile error does.

Declare only what you need. A theme that omits all four properties works exactly as before.

## What a theme may draw

- Speed, battery, range, mode, clock, trip and odometer, the media card, and the two buttons. Touch targets at least 64 px.
- Telltales and safety alerts: **none.** The host draws them and can be placed and ordered by the theme (see above). The one exception is the turn arrows inside the main gauge, which are optional.
- High beam, headlight and locked state are drawn by the host overlay. The default theme leaves them to it.
- `DriveMode.unknown` is valid. Render it sensibly (the default theme uses a neutral grey accent).

## Callbacks allowed

`Media.play-pause()`, `Media.next()`, `Media.previous()`, `Actions.open-android-auto()`, `Actions.open-settings()`. Nothing else. Never add a callback that can change vehicle state.

## Fonts

Bundle fonts. Do not rely on the device's fonts:

```slint
import "fonts/Inter-Regular.ttf";
import "fonts/Inter-Bold.ttf";
```

Ship the font's license file next to it (see `themes/default/fonts/`). The default theme uses Inter under the SIL Open Font License 1.1.

## Size budget

- Theme folder under 2 MB in total. The default theme is about 1.2 MB, almost all of it font files.
- No bitmap over 200 KB. Prefer `Path`, gradients and `opacity`.
- No video or Lottie-style animation files.

Reviewers check the size budget by hand. `tools/check-themes.sh` does not enforce it yet.

## Packaging and installing

- A theme is a folder. Zip it or publish it as a folder; the format is the folder.
- On the device, a theme lives at `files/themes/<id>/` in the app's private directory. The active theme is named in `files/themes/active`. The host falls back to the bundled default if the active theme fails to compile.
- Themes are not in the APK. The bundled default and last-resort placeholder are.
- A theme store is on the [roadmap](roadmap.md). Until then, themes are installed by hand.

## Renderer limits

These were checked by rendering in the software renderer used by `sim`. The device uses a GL renderer that has not been tested here, so re-check on the device.

- `Path` coordinates are scaled to the path's bounding box unless `viewbox-x/-y/-width/-height` are set. Set them to the element size, as `themes/default/parts.slint` does.
- Path arcs (`A` commands) render correctly. Build them from a `commands` string bound to animated values.
- `transform-rotation` is ignored. Do not rely on rotated elements.
- `drop-shadow-*` is not drawn. Fake a glow with a radial gradient or a wider translucent stroke.
- Path stroke gradients come out as a flat colour (the first stop). Use solid strokes.
- Radial and conic gradients, `clip`, `border-radius`, `opacity` and `letter-spacing` work.
- `parent` cannot be used in the root element of a component. Use fixed sizes there.
- `ArcTo` uses `radius-x` and `radius-y`. Path coordinates are plain floats, not lengths.

## Motion

- `animate` on a property eases it whenever its value changes, including when the binding changes. Use it on readings (speed, battery) and on the mode accent.
- Do not animate a property built from another eased property. Each change restarts the animation and it stalls. The default theme's mode pills take the accent directly for this reason.
- `animation-tick()` in a binding (blink, breathing, charge sweep) redraws the screen every frame. Keep these bindings few.

## Previewing

- Live window with mock data (needs a display): `sim themes/default`, or `cargo run --release --manifest-path sim/Cargo.toml -- themes/default`.
- Headless PNG at a given sim time: `sim themes/default --screenshot out.png --at 75`. The frame is the same every run.
- `sim` loads the theme inside the host's `Host` root, so the overlay is drawn above it, as on the device. Mock telltales cycle (indicators, high beam, fault, a side-stand alert card at 100-110 s of each 120 s cycle).
- `--placement top|bottom|left|right` previews another placement. `--alert 1..6` holds one host alert on screen.
- `sim` looks for the contract at `<theme>/../../contract` by default. Pass `--contract <dir>` to override.
- `sim --check themes/<id>` validates the manifest and compiles the theme without a window. `tools/check-themes.sh` runs it for every theme.
- `slint-viewer -I contract themes/default/main.slint` gives a static preview, with globals at their defaults and no mock data. Not checked in this project.

`sim` is a Rust crate in `sim/`. Building it needs fontconfig's `.pc` file (`fontconfig-devel` on Fedora). See [building.md](building.md).
