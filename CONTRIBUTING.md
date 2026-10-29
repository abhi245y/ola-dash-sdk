# Contributing

Thanks for looking at this. The most useful contributions right now are themes, theme checks, documentation fixes, and careful reports of how the dash behaves on a real scooter.

Before you start, read [docs/safety.md](docs/safety.md). The safety rules are not negotiable and they shape what we accept.

## Things we will not accept

- Ola's vendor files: AIDL interfaces, decompiled code, firmware, keys, passcodes, account credentials, or anything taken from the scooter's update or service paths.
- Theme or plugin code that sends vehicle commands, or a way to hide the host safety overlay.
- Anything that makes the test channel active in a release build.
- Logs or screenshots that contain a passcode, account identifiers, or a VIN. Redact before attaching.

## Development setup

- **Themes and sim:** a recent stable Rust toolchain plus fontconfig development files (`fontconfig-devel` on Fedora, `libfontconfig1-dev` on Debian and Ubuntu).
- **Host:** needs the Android toolchain and the vendor classes. See [docs/building.md](docs/building.md). Contributors without the vendor classes can still work on the sim, themes, contract, docs and tools.

```sh
tools/check-themes.sh                                   # must pass before a PR
mkdir -p tmp
cargo run --release --manifest-path sim/Cargo.toml -- themes/minimal --screenshot tmp/minimal.png --at 20
```

Screenshots go under `tmp/`, which is git-ignored.

## Adding a theme

1. Copy `themes/minimal/` to `themes/<your-id>/`. The minimal theme is the smallest one that uses every contract field.
2. Edit `theme.json`. `id` must equal the folder name. Set `contract` to the lowest contract version your theme needs.
3. Keep the top 48 px of the window empty. The host draws its telltale bar there.
4. Run `tools/check-themes.sh`, then render screenshots with `sim` at a few times (see [docs/testing.md](docs/testing.md)).
5. Open a PR with the theme and two screenshots. Use the theme submission issue template if you only want to propose one.

Full rules: [docs/themes.md](docs/themes.md).

## Adding a plugin

The plugin API is a specification. The host implementation is in progress, so plugin work today means:

- Proposing a change to the API through an issue (plugin submission template), or
- Building a plugin against the AIDL in `sdk/plugin-api/aidl/` as an independent APK. Do not add it to this repository without discussion first.

See [docs/plugins.md](docs/plugins.md).

## Listing a related project

Know another open Ola scooter project? Add one row to
[docs/related-projects.md](docs/related-projects.md) in a pull request; the format is
described at the bottom of that page.

## Contract changes

The theme contract (`contract/vehicle.slint`) is the stable API that themes depend on. Changes must follow these rules:

1. **Additive only.** Never rename, remove or change the type of a field, enum value or callback. Append new enum values at the end of the enum.
2. **Bump `contract-version`** in `contract/vehicle.slint` by one.
3. **Add a line to the history comment** at the top of the file, for example `v6 <what was added>`.
4. **Update [docs/contract.md](docs/contract.md)** with every new field, enum value or callback, including units and the `-1` convention where it applies.
5. **Update the CHANGELOG** under "Unreleased".
6. Run `tools/check-themes.sh`. Both bundled themes must still pass.

Changes to the plugin AIDL follow the same additive rule and bump the API version documented in `docs/plugins.md`.

## Commit style

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
feat(sim): validate theme.json in --check
fix(host): keep overlay above theme after reattach
docs(contract): document Battery.health-pct
```

Common types: `feat`, `fix`, `docs`, `refactor`, `test`, `build`, `ci`, `chore`. Keep the subject under about 72 characters. Use the body to explain why when it is not obvious.

Sign-off (DCO) is optional. If you want it, add `git commit -s`, which appends `Signed-off-by:`.

## Pull request checklist

- [ ] `tools/check-themes.sh` passes (if you touched `themes/`, `contract/` or `sim/`).
- [ ] Screenshots attached for any visual change.
- [ ] Contract changes follow the rules above, including the version bump and history line.
- [ ] Docs updated for user-visible changes. CHANGELOG updated under "Unreleased".
- [ ] No vendor files, secrets, logs with identifiers, or binaries in the diff.
- [ ] Safety rules still hold: overlay not hideable, no vehicle commands from themes or plugins, test channel gated to debug builds.
- [ ] Commit messages follow Conventional Commits.

## Reviews and maintainers

See [.github/CODEOWNERS](.github/CODEOWNERS). Maintainers may ask for hardware test notes on anything that touches the host runtime, the bridge, or the unlock module.

## Code of conduct

Participation is covered by the [Code of Conduct](CODE_OF_CONDUCT.md). Security issues go through [SECURITY.md](SECURITY.md), not public issues.
