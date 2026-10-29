## Summary

<!-- What changed and why. Link related issues. -->

## Type

- [ ] Theme
- [ ] Contract (version bump and history line included)
- [ ] Host (safety-relevant: bridge, overlay, unlock, logging)
- [ ] sim / tools
- [ ] Docs
- [ ] CI / build

## Checklist

- [ ] `tools/check-themes.sh` passes (if `themes/`, `contract/` or `sim/` changed)
- [ ] Screenshots attached for visual changes
- [ ] Contract changes are additive, bump `contract-version`, add a history line, and update `docs/contract.md`
- [ ] Docs updated; CHANGELOG entry under "Unreleased"
- [ ] No vendor files, passcodes, keys, account data, logs with identifiers, or binaries
- [ ] Safety rules hold: overlay not hideable, no vehicle commands from themes or plugins, test channel gated to debug builds
- [ ] Commits follow Conventional Commits

## Safety notes

<!-- Required for host, contract, or plugin changes. Say "none" if not applicable. -->

## Testing

<!-- Sim renders, check output, or hardware test notes. For hardware tests say whether the scooter was real or data was injected. -->
