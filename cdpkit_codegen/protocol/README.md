# CDP protocol snapshot

The checked-in protocol inputs are an immutable snapshot of
[`ChromeDevTools/devtools-protocol`](https://github.com/ChromeDevTools/devtools-protocol)
at commit [`f8bae521a2574e3e414b4268d2b9be2b2a633ecf`](https://github.com/ChromeDevTools/devtools-protocol/commit/f8bae521a2574e3e414b4268d2b9be2b2a633ecf)
(`Roll protocol to r1578551`, 2026-02-03).

| File | SHA-256 |
|---|---|
| `browser_protocol.json` | `663e772ae339908f114ccdf294f0b9d5b9d3bf026c2796d967724b20f0d7ece1` |
| `js_protocol.json` | `5a54a335617a0ff088c22f8d7a39ee7616ebdba3eb982ebf4e6b1869239e60f5` |

Normal generation reads these files without network access. To refresh the
snapshot, update the immutable revision in `cdpkit_codegen/src/fetch.rs`, run
`cargo run -p cdpkit_codegen -- --update`, regenerate `protocol.rs`, and update
the revision and hashes in this file in the same commit.
