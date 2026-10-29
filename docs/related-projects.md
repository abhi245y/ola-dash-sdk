# Related projects

Other open projects around Ola Electric scooters. None of them, and none of the
projects listed here, is affiliated with or endorsed by Ola Electric.

Listing a project here is not an endorsement by Ola Dash SDK either: check each
project's license, safety notes and what it changes on your scooter before you use it.

| Project | What it is | License |
|---|---|---|
| [ola-s1-rea](https://github.com/itsmobsys/ola-s1-rea) | Reverse engineering of the Ola S1 head unit: root access over IPv6, ADB setup, CAN bus map, HMI internals, OTA and security findings, plus a companion Android app and scripts. | See repository |
| [Axiom](https://codeberg.org/pitzel/axiom) | Android app for CAN bus telemetry, fault decoding and system management over the scooter's hotspot network: reads CAN frames through the vehicle's gRPC service and can provision wireless ADB. | Apache-2.0 |

## Adding a project

Open a pull request that adds one row to the table above:

- **Project**: name linked to the repository (any public forge is fine).
- **What it is**: one or two plain sentences: what it does, which scooter model or
  component (app, BLE, head unit, cloud, firmware) it touches. No marketing copy.
- **License**: the SPDX identifier from the project's LICENSE file, or "See repository".

Keep rows in alphabetical order by name. Projects must be public and must not ship
code or keys copied from Ola Electric software, credentials, or tools aimed at other
people's vehicles. See [CONTRIBUTING.md](../CONTRIBUTING.md).
