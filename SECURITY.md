# Security policy

## Reporting a vulnerability

Please do not open a public issue for security problems.

- Preferred: GitHub private vulnerability reporting on this repository (Security tab, "Report a vulnerability").
- Otherwise: email `<maintainer email>`.

Include what you found, the commit or release it affects, steps to reproduce, and the impact as you understand it. Do not include a real passcode, account credentials, a VIN, or unredacted logs. Describe the data instead.

We aim to acknowledge reports within a week and to agree on a fix and disclosure date with you. This is a volunteer project, so these are targets, not guarantees.

## Priority: vehicle safety

Anything that could let a theme, plugin, or the dash itself change vehicle state, hide a safety alert, unlock or start the scooter without the owner, or misreport speed, battery or drive state to the rider is treated as the highest priority. Report these even when you are unsure they are exploitable.

## Scope

In scope:

- The host app (`host/`): the Java bridge, the Rust runtime, the safety overlay, the unlock module.
- The theme loader and the theme contract (`contract/`).
- The plugin API definition (`sdk/plugin-api/`) once the host implements it.
- `sim/` and `tools/` only where they could affect a built host or a shipped theme.

Out of scope:

- Ola Electric's own services, firmware, apps and servers. Report those to Ola Electric through its own channels.
- Vulnerabilities that need physical access to an unlocked scooter that the owner has not configured this software on.
- Denial of service against a local development tool.

## Supported versions

Only the latest 0.x release receives fixes. The project is pre-1.0 and experimental.
