# The study of plugins

A workspace of its own: `wwrt` (the runtime of the programs with plugins, a Rust dylib), `wview`
(the viewer) and its plugins `wv-text` and `wv-image`. What it is for, what it measured and the
plan of wview: [docs/plugins.md](../../docs/plugins.md).

```bat
cargo build --release --workspace
target\release\wview --plugins
target\release\wview --probe FILE
```

`.cargo/config.toml` links std dynamically for every crate here (`-C prefer-dynamic`); the release
profile has no LTO, which a dylib cannot take. The crates of W# come from `../../crates` by path,
and `Cargo.lock` started as a copy of the one of WinWeaver, so the versions are the same.
