# wview

A viewer of files on W#, every format a native plugin: a direction of its own beside WinWeaver and
a workspace of its own (docs/plugins.md, the study, the design and the phases).

| Crate | What it is |
|---|---|
| `crates/wwrt` | the runtime, a Rust dylib (`wwrt.dll`): W#, egui and the crates around them re-exported; `plugin` (the interface, version 2: `Plugin`, `View`, `Host`, `plugin!`, the formats with their MIME types, the items of the menu, bytes in memory, the loader that checks the build and takes a plugin of the stable interface too), `inner` (a view inside a view, by the plugin of the type of some bytes), `mime` (the type of some bytes by their first ones), `stable` (the stable interface of `wview.h` as a `Plugin`), `input` (the keys and the mouse of a view), `blocks` (a file read in blocks), `testing` (the tests of the translations) |
| `crates/wview` | the program: a tab a file, the page of the plugins, Settings, Plugins, the guide; `--plugins`, `--probe FILE` |
| `plugins/wv-text` | Markdown pages, text files and XML files in one plugin, of files and of bytes in memory |
| `plugins/wv-image` | images through the Windows Imaging Component: decoded in a thread, kept with their levels and drawn in tiles of the level the zoom needs, the frames of a GIF played, the pages of a TIFF or an ICO, the orientation of EXIF; fit, zoom, the mouse, the keys and the steps through the folder; `sample`, the BMP and the GIF of the tests |
| `plugins/wv-binary` | binary files read by a description of their format (`formats/pe.wwformat` built in, more in the folder `formats` next to the DLL): `expr` and `format` (the language), `engine` (the tree, the bytes left in the file, the fields read when they open), `hex`, `content` and `view` (the tree with the bytes, the tree as a page, the content of a node shown by the plugin of its type), `wwbin` (the structure saved without the bytes, and read back), `diff` and `compareview` (a file compared with its structure, the notes of a `.wwbindiff`), `sample` (the files of the tests of wview) |
| `plugins/wv-hello` | the template of a plugin, the one the SDK carries |
| `stable/` | the stable interface for plugins in other languages: `wview.h` (version 1), its README, its example in C, `wv-mandelbrot` (built by CI with the compiler of Visual Studio), and `rust/`, the crate `wview-stable` for a plugin of it in Rust of any toolchain; the host side is `wwrt::stable` |
| `sdk/` | `README.md` of the SDK and `pack.py`, which makes the tree of a release: W# in `framework/`, wwrt and wview in `crates/`, the plugins, the lock file, the flags and the compiler; the release is built in it, and it is the SDK |
| `package/README.md` | the README of the archive of wview, for its user |

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

`target\release` then holds `wview.exe`, `wwrt.dll` and the plugins `wv_text.dll`,
`wv_image.dll`, `wv_binary.dll` and `wv_hello.dll` (the template; the archive leaves it out). A folder to copy anywhere: `wview.exe`, `wwrt.dll`, `std-<hash>.dll` of the
toolchain (`rustc --print sysroot`, then `bin`; with the toolchain gnu also `libgcc_s_seh-1.dll`
and `libwinpthread-1.dll` of it), and the plugins in `plugins\`.

A plugin depends on `wwrt` alone and takes every shared crate through it (`wwrt::egui`,
`wwrt::ww`, `wwrt::serde` with `#[serde(crate = "wwrt::serde")]`); its texts are English with
`lang/<code>.json` beside them, given by `Plugin::translations`, and its test checks them with
`wwrt::testing`. Each declares the feature `library = []`, with which `wwrt::plugin!` exports
nothing: the tests of wview take the plugins as libraries into one executable. The tests of the
window draw it with `wwrt::egui_kittest` (the feature `kittest` of wwrt), and with
`WINWEAVER_SCREENSHOTS` they save the screenshots `v1-` to `v13-` there. CI and `build-all.cmd`
pack `wview-<version>` and `wview-sdk-<version>`; CI builds the template of the SDK in another
folder and loads it into wview (`docs/spec/05-details.md`, section 8).
