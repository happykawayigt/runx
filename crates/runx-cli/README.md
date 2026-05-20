# runx-cli

`runx-cli` is the Cargo package for the native `runx` command.

```bash
cargo install runx-cli
runx --help
```

The `runx` crate name on crates.io is already owned by an unrelated package, so
the published Cargo package is `runx-cli` while the installed binary remains
`runx`.

## Runtime Requirements

- Rust/Cargo for installation from crates.io.
- No Node.js runtime is required for the native CLI.
