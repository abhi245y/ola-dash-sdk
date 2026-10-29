# Product

<!-- impeccable:product-schema 1 -->

## Platform

android

Embedded vehicle display: the Ola S1 Pro Gen 1 head unit (Android 10, SDM450 / Adreno 506, 1280x768 landscape, ~236 dpi, touchscreen, outdoor sunlight). The UI is rendered by Slint 1.18.1 inside a NativeActivity, not Android Views or Material components. It is the device's home screen.

## Users

- **Riders** (primary moment): glance at the dash while riding a scooter, often in direct sunlight, with gloves and vibration. The job is to read speed, warnings, indicators and battery in well under a second and look back at the road.
- **Theme designers** (primary audience of the SDK's reference screens): community members who copy and remix the bundled default theme and the safety overlay to make their own dash. They work on a PC with the simulator, without a scooter.
- Owners also use the dash parked: unlocking, charging, media, plugins, launching Android Auto.

## Product Purpose

Ola Dash SDK is an unofficial, open platform that replaces the stock 640 MB dash with a ~70 MB Slint host. Themes (runtime-loaded `.slint` folders with `theme.json`) define the look; plugins (separate APKs) feed widgets, media and navigation; the host owns vehicle data and the safety overlay. Success for the reference screens: riders can read them instantly, and designers see what is possible and copy patterns confidently.

## Positioning

The only dash for this scooter where the look is fully replaceable at runtime by the community while safety warnings stay host-guaranteed above any theme.

## Operating Context

- Riding: speed, drive mode (eco, normal, sport, hyper or custom, reverse, easy park), turn indicators, beams, ready state, battery/range, brakes and regen.
- Parked/ready: media card, plugin widgets, Android Auto launch, settings, trip since unlock.
- Charging: charge state (charging-specific fields arrive in a later contract version).
- Locked/asleep: owner's unlock flow (separate module) covers the dash.
- Alerts: host overlay cards for fall/accident, tow/theft, side stand, steering lock (not dismissible), vehicle fault and low battery (tap-dismissible). Telltale strip placement is theme-chosen: top, bottom, left or right.
- Designers preview with `sim` (headless screenshots, mock ride/charge loop) and validate with `tools/check-themes.sh`.

## Capabilities and Constraints

- Theme contract v6 (`contract/vehicle.slint`): read-only Vehicle, Battery, Ride, Media, Navigation, Plugins globals; `-1` means not reported (show "--"); optional overlay placement, telltale order, overlay style, alert position.
- Top/bottom reserved strip 48 px, left/right 72 px, owned by the host overlay.
- Renderer limits: Slint software renderer in sim ignores rotation and drop shadows; path stroke gradients render flat; keep assets small (theme budget ~2 MB, no video). Few `animation-tick()` bindings (each redraws every frame on a low-end GPU).
- Fonts must be open-licensed and bundled per theme.
- No Ola Electric logos, artwork, fonts or colors (trademark; project is unaffiliated).
- Reference states to cover: every state the SDK exposes (riding, parked/ready, charging, alerts across placements), not every combination.

## Brand Commitments

- Name: Ola Dash SDK. Unofficial; not affiliated with Ola Electric.
- The reference look should feel neutral and theme-able: personality comes from craft (type, rhythm, motion, data presentation), not from branding, so community themes can stand out against it.

## Evidence on Hand

- Real measurements and logs: ride-log analysis in `docs/ride-log-2026-10-30.md` (modes, states, speeds, ranges).
- Existing screenshots in `docs/img/`.
- No testimonials, user counts or benchmarks beyond the measured RAM figures; do not invent any.

## Product Principles

1. Glance first: anything a rider needs while moving is readable in under a second in sunlight.
2. Safety is host-owned: themes arrange and style warnings but can never hide or weaken them.
3. Reference, not ornament: every pattern in the bundled screens should be copyable and explainable to a theme author.
4. Honest data: unknown values show as unknown, never as a fake zero.
5. Light on the hardware: designs respect the low-end GPU and RAM budget.

## Accessibility & Inclusion

- Sunlight legibility and glove use: large type, high contrast (overlay contrast ≥ 4.5:1 enforced by the host), large touch targets.
- Meaning never relies on color alone for safety signals (icon + color, and text on alert cards).
