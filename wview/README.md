# wview

A viewer of files on W#, every format a native plugin: a direction of its own beside WinWeaver and
a workspace of its own (docs/plugins.md, the study, the design and the phases).

| Crate | What it is |
|---|---|
| `crates/wwrt` | the runtime, a Rust dylib (`wwrt.dll`): W#, egui and the crates around them re-exported; `plugin` (the interface, version 1: `Plugin`, `View`, `Host`, `plugin!`, the loader that checks the build), `input` (the keys and the mouse of a view), `blocks` (a file read in blocks), `testing` (the tests of the translations) |
| `crates/wview` | the program: a tab a file, the page of the plugins, Settings, Plugins, the guide; `--plugins`, `--probe FILE` |
| `plugins/wv-text` | Markdown pages and text files, two formats in one plugin |
| `plugins/wv-image` | images through the Windows Imaging Component, with fit, zoom, the mouse, the keys and the steps through the folder |

```bat
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release --workspace
```

`.cargo/config.toml` links std dynamically for every crate here (`-C prefer-dynamic`); the release
profile has no LTO, which a dylib cannot take, and a debug build keeps the dependencies at
opt-level 2, else `wwrt.dll` holds more than the 65535 exports of a DLL. The crates of W# come from
`../crates` by path, so `Cargo.lock` here keeps their versions with the ones of WinWeaver.

`target\release` then holds `wview.exe`, `wwrt.dll` and the plugins `wv_text.dll` and
`wv_image.dll`. A folder to copy anywhere: `wview.exe`, `wwrt.dll`, `std-<hash>.dll` of the
toolchain (`rustc --print sysroot`, then `bin`; with the toolchain gnu also `libgcc_s_seh-1.dll`
and `libwinpthread-1.dll` of it), and the plugins in `plugins\`.

A plugin depends on `wwrt` alone and takes every shared crate through it (`wwrt::egui`,
`wwrt::ww`, `wwrt::serde` with `#[serde(crate = "wwrt::serde")]`); its texts are English with
`lang/<code>.json` beside them, given by `Plugin::translations`, and its test checks them with
`wwrt::testing`.
