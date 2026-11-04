# Plugins of W#: the study and wview

The study of 04.11.2026 of plugins written in Rust for the programs on W#, and the plan of wview,
the viewer of files built on them. The prototype is `study/plugins`, a workspace of its own.

## 1. The question

A program loads the DLLs of a folder, written in Rust as the program is. A plugin should hold its
own code alone: std, egui, eframe and W# belong to DLLs every program of this kind shares, which is
the exception such a program makes to the rule of one executable (AGENTS.md, rule 4). The way should
stay as close to ordinary Rust as it can: traits, `egui::Ui`, the types of W#, no C layer in
between.

## 2. The ways Rust has

| Way | The plugin draws in the window of the host | Base code in each plugin | ABI | Verdict |
|---|---|---|---|---|
| A runtime as a Rust `dylib`, the plugins `cdylib` on it | yes: one egui, one `Ui`, the traits of W# | none | the compiler and the lock file of the build | chosen |
| `cdylib` with a C ABI | no: each plugin has its own egui, its own statics; a `Ui` of the host cannot cross | all of it, about 10 MB a plugin | stable | only for plugins of others that give data |
| `abi_stable`, `stabby` | no: only their stable types cross | all of it | stable, checked at the load | the same |
| WebAssembly (wasmtime, extism) | no: the plugin describes, the host draws | a runtime of 10 MB and more | stable, a sandbox | for plugins nobody vouches for |
| A process a plugin, IPC | no | a whole program | stable, isolated | for crashes and rights apart |
| Descriptions as data (JSON) | not code | none | the format of the file | inside a plugin: the binary formats of wv-binary (section 6.4) |

Rust has no stable ABI, so the first way needs one build of the host, the runtime and the plugins:
one compiler, one `Cargo.lock`, one profile. For a family of programs and plugins of one repository
that is no loss, and it is the only way where a plugin is its own code alone and draws with the egui
of the program.

## 3. What was built

`study/plugins`:

- `wwrt`, a Rust `dylib` (`wwrt.dll`): W# with egui, eframe, glow, wgpu, winit, serde, serde_json,
  the crate `windows` and libloading, all re-exported (`wwrt::ww`, `wwrt::egui`, `wwrt::serde`,
  `wwrt::windows`). `wwrt::plugin` is the interface: `Plugin` (its name, its `Format`s, `open`),
  `View` (`ui`, `tools` and `act`, `search`, `browses`, `status`), `Host` (the settings a view
  draws with), the macro `plugin!` and `Plugins::load_dir`, the loader. `wwrt::blocks::BlockFile`
  reads a file in blocks with a cache of a fixed size.
- `wview`, the program: the window of W#, a tab a file, the command bar of the view in front, the
  page of the plugins without a file; `--plugins` lists them, `--probe FILE` opens a file and draws
  three frames without a window.
- `wv-text`, Markdown pages and text files, two formats in one plugin, drawn by `ww::mdview` with
  the search of W#, Wrap lines and Width of pages.
- `wv-image`, images through the Windows Imaging Component: the formats of the decoders of this
  Windows, read at the load (on this PC 14 decoders: BMP, GIF, ICO, CUR, JPEG, PNG, TIFF, DNG,
  JPEG XR, DDS, HEIF and AVIF, WebP, camera RAW of 36 extensions, JPEG XL); Fit, 100%, Zoom in,
  Zoom out and Ctrl with the wheel; an image above 8192 pixels a side is scaled down by WIC as it is
  decoded. The host steps through the images of the folder: Previous and Next in the bar, the
  arrows, Page Up and Page Down, Home and End.

A whole plugin is its formats, its view and one line:

```rust
use wwrt::plugin::{Format, Host, Plugin, View};

struct TextPlugin;

impl Plugin for TextPlugin {
    fn name(&self) -> &'static str { "Text and Markdown" }
    fn formats(&self) -> &'static [Format] { &FORMATS }
    fn open(&self, path: &Path) -> Result<Box<dyn View>, String> { /* read, make the view */ }
}

wwrt::plugin!(TextPlugin);
```

`plugin!` exports two functions: `ww_plugin_abi`, the build of wwrt the plugin was made with, and
`ww_plugin`, which makes the plugin. The loader compares the first with its own build, then calls
the second and keeps the DLL for the whole run.

## 4. Measurements

On 04.11.2026 on the computer of the developer: Rust 1.99.1, the target
`x86_64-pc-windows-gnu`, half of an i5-1035G1 (`cargo50.cmd`), the release profile of the study (no
LTO, one codegen unit, stripped).

| File | Size | It needs |
|---|---|---|
| `wview.exe` | 477 KB | `wwrt.dll`, `std-*.dll` |
| `wwrt.dll` | 20.9 MB | `std-*.dll` |
| `std-f903e3d16df36da5.dll` | 5.3 MB | the toolchain gnu also `libgcc_s_seh-1.dll` (142 KB) and `libwinpthread-1.dll` |
| `plugins\wv_text.dll` | 27 KB | `wwrt.dll`, `std-*.dll` |
| `plugins\wv_image.dll` | 52 KB | `wwrt.dll`, `std-*.dll` |

For comparison the static programs of the repository with LTO: `wedit.exe` 12.9 MB,
`winweaver.exe` 21.6 MB. A plugin with its own copy of egui would weigh about what wedit does (an
estimate, not built).

| What | Time |
|---|---|
| The two plugins loaded (the folder read, the DLLs, the checks, the list of the decoders of WIC) | 0.9 ms |
| AGENTS.md (96 KB, 1007 lines) opened / three frames drawn | 12 ms / 78 ms |
| A text of 40 lines | 10 ms / 8 ms |
| A PNG of 640 x 480, a BMP of 320 x 240 | 13 ms / 2 ms |
| A PNG of 9000 x 2000, decoded at 8192 x 1820 | 144 ms / 46 ms |
| A whole release build of the study | 9 min 10 s |
| A whole debug build (the dependencies at opt-level 2) | 10 min 39 s |
| A plugin rebuilt after a change, debug (finding 11) | 50-55 s |
| wwrt, wview and both plugins after a change of wwrt, debug | 86 s |

The folder of the program ran with a PATH of `C:\Windows\System32` and `C:\Windows` alone. A DLL
that is no plugin (a copy of `std-*.dll` in `plugins`) and a plugin whose build differs (its text
of the build patched) were refused with their reasons, and Windows showed no dialog. The window
itself was not opened on the computer of the developer, which runs nothing visible: `--probe`
draws the same views with the same egui, without a screen.

## 5. What the study found

1. **One std for the family.** A `dylib` takes std into itself, and an executable that links it
   wants std of its own: rustc stops with `cannot satisfy dependencies so std only shows up once`.
   Every crate of the family is built with `-C prefer-dynamic` (`study/plugins/.cargo/config.toml`),
   std becomes `std-<hash>.dll` of the toolchain, and it ships next to the executable.
2. **The 65535 exports of a DLL.** A `dylib` exports every public symbol of the crates it holds,
   and the export table of a PE file numbers its entries in 16 bits. A debug `wwrt.dll` wanted
   128973 exports: `ld: error: export ordinal too large: 128973` (debug builds share the instances
   of generic functions between crates and export them). The release one has 17390, 15616 with std
   apart. The largest shares: `windows` 4671, `read_fonts` 2556, `egui` 688, `wgpu_core` 615,
   `alloc` 529, `naga` 433, `glow` 416, `wgpu` 373, `winit` 285. A debug build links when the
   dependencies and wwrt are optimized (`opt-level = 2` for `"*"` and `wwrt` in the profile `dev`,
   since a crate at opt-level 2 shares no instances): 17600 exports, `wwrt.dll` 40.5 MB,
   `wview.exe` 2.2 MB, the plugins 150 and 320 KB, and only the program and the plugins stay
   unoptimized. When the runtime grows past the limit, it splits into two DLLs (wgpu and naga
   apart, say) or takes fewer features of `windows`.
3. **No LTO across the runtime.** rustc runs LTO for executables, cdylibs and static libraries,
   never for a `dylib`, and a `dylib` keeps every public function, since it exports them all:
   `wwrt.dll` with W# is larger than wedit with W# and LTO. With the release profile of
   WinWeaver (`lto = "thin"`) cargo builds `wwrt.dll` without LTO but every plugin and wview stop:
   `cannot prefer dynamic linking when performing LTO`, `only 'staticlib', 'bin', and 'cdylib'
   outputs are supported with LTO`. The family has a release profile of its own.
4. **Every shared crate through the runtime.** A plugin with a dependency of its own on a crate
   wwrt holds (serde, egui, windows) would bring a second copy, which rustc refuses; it takes
   `wwrt::serde` and so on. A derive of serde names its crate: `#[serde(crate = "wwrt::serde")]`.
   A crate wwrt does not hold (a parser of a format) is the plugin's own.
5. **One build, checked.** `plugin::ABI` is the build of wwrt: its version, the compiler, the
   target, the profile and the FNV of the lock file (`wwrt/build.rs`). A plugin carries the one it
   was made with, and the loader, which runs inside `wwrt.dll`, compares it with its own. A plugin
   built against another `wwrt.dll` misses its imports and fails in `LoadLibrary`; the loader sets
   the error mode of its thread (`SetThreadErrorMode`), so Windows shows no dialog and the list of
   the plugins names the reason.
6. **A plugin is never unloaded.** Its code and the tables of its traits live as long as the views
   it made; `Plugins::load_dir` forgets the `Library`.
7. **COM belongs to the window.** winit takes the apartment of the main thread for drag and drop; a
   plugin that asks COM before the window (the list of the decoders) does it in a thread of its own.
8. **The toolchain gnu brings MinGW.** Its `dylib` and `std-*.dll` link the unwinder of MinGW
   dynamically: `libgcc_s_seh-1.dll` and `libwinpthread-1.dll` ship too. The toolchain msvc of CI
   needs none of them; a release of wview is built with it.
9. **A plugin has the rights of the program.** It is native code in the process, as trusted as the
   folder of the program; there is no sandbox. A later setting may list the SHA-256 of the plugins
   allowed.
10. **A panic of a plugin unwinds into the host**, since both have one std; the host can catch it
    around every call of a plugin and close the tab instead of the program.
11. **A link costs the import libraries of Windows.** A plugin changed rebuilds in about 50 s on
    this PC (measured in a debug build), and nearly none of it is the plugin: `link_binary` takes
    50.3 s, the linker 0.7 s, the codegen of the plugin 0.2 s (`-Z time-passes`). The crates in
    wwrt call 68 DLLs of Windows through `raw-dylib`, rustc writes an import library of each for
    every link that depends on them (`--print link-args` names them), and the toolchain gnu runs
    `dlltool` and `as` for each one. The toolchain msvc should write them through LLVM itself; CI
    measures what a link of a plugin costs there. The links of WinWeaver on this PC likely pay the
    same.

## 6. The design of wview

### 6.1 The interface

Today: `Plugin` (`name`, `formats`, `open`) and `View` (`ui`, `tools`, `act`, `search`, `browses`,
`status`). To come: the settings of a plugin (a JSON object a plugin in the settings of the host,
drawn by the plugin in a section of Settings: the cache of wv-binary, the width of its rows); a
format found by its first bytes besides its extension; a view that saves its own file (wwbin);
the translations of a plugin (its `lang/<code>.json`, read by the `i18n` of W#).

### 6.2 Images (wv-image)

Done in the study: WIC, the formats of this Windows, fit, zoom, the folder. To come: tiles for an
image above the size of a texture (a region decoded at 100% with `CopyPixels` of a rectangle), the
frames of a GIF, the orientation of EXIF, the keys of the zoom, the crate `image` (in wwrt already
for PNG) where WIC has no decoder.

### 6.3 Text and Markdown (wv-text)

Done in the study: both formats, the page and the text of W#, the search. To come: a file above
64 MB read by `BlockFile` with an index of its lines instead of whole.

### 6.4 Binary structures (wv-binary)

One plugin, many formats: the engine reads the descriptions of `formats/*.wwformat` next to it and
opens the extensions they name after a check of their first bytes. A description is data, never
code:

```json
{
  "format": "wwformat", "version": 1, "name": "Portable Executable",
  "extensions": ["exe", "dll", "sys", "ocx", "cpl", "efi", "mui"],
  "endian": "little",
  "magic": [{ "at": 0, "bytes": "4D5A" }],
  "root": "file",
  "spaces": {
    "rva": { "table": "sections", "virtual": "virtual_address", "size": "virtual_size",
             "file": "pointer_to_raw_data" }
  },
  "types": {
    "file": { "fields": [
      { "name": "dos_header", "type": "IMAGE_DOS_HEADER" },
      { "name": "nt_headers", "type": "IMAGE_NT_HEADERS", "at": "dos_header.e_lfanew" },
      { "name": "sections", "type": "IMAGE_SECTION_HEADER", "key": "name",
        "count": "nt_headers.file_header.number_of_sections",
        "at": "nt_headers.optional_header.@end" }
    ] },
    "IMAGE_FILE_HEADER": { "fields": [
      { "name": "machine", "type": "u16", "enum": "machine" },
      { "name": "number_of_sections", "type": "u16" },
      { "name": "characteristics", "type": "u16", "flags": "file_characteristics" }
    ] }
  },
  "enums": { "machine": { "0x14c": "I386", "0x8664": "AMD64", "0xaa64": "ARM64" } }
}
```

The language: integers of 1 to 8 bytes and both orders, floats, texts of a size or ending in a
zero (ASCII and UTF-16), arrays of a count or a size, `at` (an offset of the file) or `rva` (an
address of a space, mapped by its table), `when` and `switch` (PE32 or PE32+ by the magic of the
optional header), types that hold themselves (the tree of the resources), `enum` and `flags` for
the names of values, `key` for the items of an array, `raw` for data shown as bytes alone, and
small expressions over the fields (`+ - * / & | << >>`, comparisons, `@offset`, `@size`, `@end`).
PE comes in parts: the headers, the sections, the imports and the exports first; the resources,
the relocations, debug, TLS, load config, the certificates and the header of .NET after.

- **Never the whole file in memory.** A node of the tree is its type and its offset; its children
  are read when it opens, through `BlockFile`; the size of the cache and of a block are settings of
  the plugin.
- **The hexadecimal view.** Rows of the offset, the bytes (16 a row, the width a setting) and
  their ASCII; only the rows in view are read; the range of the node selected is marked, and a
  click on a byte selects the deepest node that holds it.
- **The text view.** The tree as a Markdown page drawn by W#: Wrap lines, the width of pages and
  the search as for any page; more views later.
- **`.wwbin`.** The tree of a file saved without its binary data: every field with its value and
  its names, every `raw` range with its offset, its size and its SHA-256, the file with its size and
  SHA-256, the description with its version.
- **`.wwbindiff`.** A file compared with a `.wwbin`: a tree of the nodes that are the same,
  changed, added or removed (the marks of the comparison of catalogs), and the comments and
  bookmarks of the nodes kept in the diff file itself, by the key of a node: the names of its
  path, an item of an array by its `key` (`sections[.text].characteristics`), so a new section does
  not move the others. A tab of its own, for the audit of the changes of a file.

### 6.5 The repository and the release

- **Rule 4 gets its exception**: a program with plugins is an executable, `wwrt.dll`, `std-*.dll`
  and the folder `plugins`; WinWeaver, wedit and wwlite stay one executable each.
- **A second workspace.** `-C prefer-dynamic` is a flag of a whole build and the release profile
  of WinWeaver has LTO, so the family lives in a workspace of its own next to the one of WinWeaver
  (the study is one), with the same versions of the shared crates (its `Cargo.lock` starts as a copy
  and a check of CI compares them). One workspace would need `--config` with the flag and a profile
  of its own for every build of the family, and `--exclude` of the family in every other, a trap for
  each new command.
- **CI and build-all.cmd**: a step that builds the family with msvc and packs
  `wview-<version>.zip` (wview.exe, wwrt.dll, std-*.dll, plugins, formats, lang, README).
- **Tests.** wwrt and every plugin have their unit tests; the tests of the window need
  `egui_kittest` inside wwrt (a feature for the tests), since a dev-dependency of its own on
  kittest would bring a second egui.

### 6.6 Phases

1. The study: the runtime, the loader with its check, two plugins, the measurements. Done.
2. wview as a program of the repository: its workspace, its archive in CI and build-all.cmd, the
   settings of the plugins, the translations, the guide, the tests of the window.
3. Images whole: tiles, the frames of a GIF, EXIF, the keys of the zoom.
4. wv-binary 1: the engine with its language, the headers, sections, imports and exports of PE, the
   hexadecimal and the text views.
5. wv-binary 2: the rest of PE, `.wwbin`.
6. `.wwbindiff`: the tree of the differences, the comments and the bookmarks.

## 7. Open questions

- The size of `std-*.dll` of msvc and of `wwrt.dll` built with msvc: CI measures them at the
  first build of phase 2.
- Plugins by other people would need the same compiler and lock file, or a second, stable
  interface for data alone (section 2).
- WinWeaver and wedit could move onto wwrt later; nothing in them needs it.

## 8. Building the study

On a computer of Windows with Rust, in `study/plugins`:

```bat
cargo build --release --workspace
```

`target\release` then holds `wview.exe`, `wwrt.dll`, `wv_text.dll` and `wv_image.dll`. A folder to
copy anywhere: `wview.exe` and `wwrt.dll`, `std-<hash>.dll` of the toolchain
(`rustc --print sysroot`, then `bin` for gnu or `lib\rustlib\<target>\lib`), with gnu also
`libgcc_s_seh-1.dll` and `libwinpthread-1.dll` of its `bin`, and the plugins in `plugins\`. Then
`wview --plugins` lists them, `wview --probe FILE` opens a file without a window and `wview FILE`
shows it.
