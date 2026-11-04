# The SDK of wview

The SDK of a release of wview makes native plugins for that release. A native plugin draws in the
window of wview with its egui, and Rust has no stable ABI, so a plugin works with the release it was
built for alone: build it again for every release. `wview --plugins` lists the plugins and names
the reason of one that did not load.

## What it holds

The SDK is the tree the release itself was built in:

- `framework/ww-base`, `framework/ww`: W#, the framework of the window.
- `crates/wwrt`: the runtime of the plugins (`wwrt.dll`) and their interface: `wwrt::plugin`
  (`Plugin`, `View`, `Host`, `plugin!`), `wwrt::input` (the keys and the mouse of a view),
  `wwrt::blocks` (a file read in blocks), `wwrt::testing` (the tests of translations).
- `crates/wview`: the program.
- `plugins/wv-text`, `plugins/wv-image`: the plugins of the release, examples of real ones;
  `plugins/wv-hello`: the template of a plugin.
- `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `.cargo/config.toml`: the workspace, the
  versions of its crates, the compiler and the flags of the release.

rustc names the symbols of `wwrt.dll` by the versions, the features and the flags of its crates and
by their places in this tree. A plugin built in a copy of the tree, wherever it is, finds them in
the `wwrt.dll` of the release; change none of the files of the release, or the plugin does not load.

## A plugin of one's own

1. Install Rust with rustup (https://rustup.rs); the compiler `rust-toolchain.toml` names comes by
   itself at the first build. The toolchain msvc needs the Visual Studio Build Tools with the
   workload "Desktop development with C++". Unpack the SDK into a short path (`C:\wview-sdk`,
   say): the toolchain gnu writes files deep in `target`, and its `dlltool` stops at a path of
   more than 260 characters.
2. Copy `plugins/wv-hello` to `plugins/<name>` and give the package that name in its `Cargo.toml`.
3. Write the plugin: a `Plugin` (its formats, `open`, its translations and settings), a `View` (its
   `ui` with `wwrt::input::area` for the keys and the mouse, its tools and its status) and one
   `wwrt::plugin!(...)`.
4. In the root of the SDK: `cargo test -p <name>`, then `cargo build --release -p <name>`; the DLL
   is `target\release\<name>.dll` (a hyphen of the name becomes `_`).
5. Copy the DLL into the folder `plugins` next to `wview.exe` of the same release.

## The rules

- Every crate wview shares comes through wwrt: `wwrt::egui`, `wwrt::ww`, `wwrt::serde` (a derive
  names it: `#[serde(crate = "wwrt::serde")]`), `wwrt::serde_json`, `wwrt::windows`. A dependency
  of the plugin on one of them, or another feature of one, makes another build of it, and the
  plugin does not load. A crate wview does not hold is the plugin's own.
- `cargo update` only for the crates of the plugin (`cargo update -p <crate>`), never for the ones of
  wwrt.
- The texts of the code are English, through `tr` and `trf` of `wwrt::ww::i18n`; the host translates
  the name of the plugin, of its formats and the labels of its tools. `lang/<code>.json` gives the
  translations of a language, `Plugin::translations` names them, and the test of the template checks
  that every text has one.
- A view takes the keys it names while it has the focus, without Ctrl and Alt; Tab and Escape give
  them back to the window, so the shortcuts of the window stay the window's.
- Keep the feature `library = []` of the template: with it the plugin is a library without the
  exports of a DLL, for the tests of a program that take several plugins into one executable.
- A plugin runs with the rights of wview; a panic of it reaches the window.
