# Plugins of W#: the study and wview

The study of 04.11.2026 of plugins written in Rust for the programs on W#, and the plan of wview,
the viewer of files built on them. The prototype of the study grew into `wview/`, the workspace
of wview (phase 2).

## 1. The question

A program loads the DLLs of a folder, written in Rust as the program is. A plugin should hold its
own code alone: std, egui, eframe and W# belong to DLLs every program of this kind shares, which is
the exception such a program makes to the rule of one executable (AGENTS.md, rule 4). The way should
stay as close to ordinary Rust as it can: traits, `egui::Ui`, the types of W#, no C layer in
between.

wview is a direction of its own, apart from WinWeaver, and its plugins come in two kinds: native
ones with the SDK of a release, the main way of every example, and a stable interface of C for
other languages (the decisions of the customer of 04.11.2026, sections 6.5 and 6.7).

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

The prototype of the study (now `wview/`):

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
   Every crate of the family is built with `-C prefer-dynamic` (`wview/.cargo/config.toml`),
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
5. **One build, checked.** `plugin::ABI` is the build of wwrt: its version, the version of the
   interface, the compiler, the target and the profile (`wwrt/build.rs`). The study had the FNV of
   the lock file in it too, which phase 2 left out: rustc names every symbol by a hash of its
   crate, so other versions of the shared crates fail at the load anyway, and a plugin of the SDK
   has a lock file of its own. A plugin carries the build it was made with, and the loader, which
   runs inside `wwrt.dll`, compares it with its own. A plugin built against another `wwrt.dll`
   misses its imports and fails in `LoadLibrary`; the loader sets the error mode of its thread
   (`SetThreadErrorMode`), so Windows shows no dialog and the list of the plugins names the reason.
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
12. **A plugin of the SDK loads when the release and the SDK are one tree.** Phase 2 packed the
    crates of the repository into an SDK at another place and built the template there: the
    release refused it with the error 127 of a missing import, though egui had the same hash in
    both builds. Cargo hashes a path dependency relative to the root of the workspace only when it
    is inside it, and W# is outside `wview/`. Built in the tree of `wview/sdk/pack.py`, where every
    crate is inside the root, the release (8 min on this PC) loaded the template built in a copy of
    the tree in another folder, and refused the plugin of the first SDK with its reason.

## 6. The design of wview

### 6.1 The interface

Version 1 (`wwrt::plugin::API`, phase 2):

- `Plugin`: `name`, `formats`, `open` of a path with the settings of the plugin, `translations`
  (the code and a file of each language, added to the window at the load by `ww::i18n::add`),
  `has_settings` and `settings_ui` (its card in Settings, Plugins; the host keeps the JSON object
  of each plugin in its settings by the name of the DLL; `plugin::setting` and `set_setting` read
  and write a field).
- `View`: `ui` with a `Host` (Wrap lines, the width of pages, the settings of the plugin, and the
  requests of the view, `Request::Browse` of a step through the folder), `tools` and `act`,
  `search`, `browses` (Previous and Next in the bar), `status`.
- `wwrt::input`: `area` takes the rectangle of a view for its keys and its mouse by the rules of
  section 6.8 and gives an `Input`; `cursor` sets the cursor.
- `wwrt::testing`: the texts of the sources of a plugin and the ones its translations lack.

To come: a format found by its first bytes besides its extension; a view that saves its own file
(wwbin).

### 6.2 Images (wv-image)

Done in the study: WIC, the formats of this Windows, fit, zoom, the folder. Done in phase 3:

- **The decoding** runs in a thread of its own (`view::start`). The window waits 250 ms for it, so
  a small image opens with its tab and one that does not decode stays closed with the reason; a
  larger one opens with Decoding... and shows when it is there. An image is decoded whole up to
  the budget of the settings (64, 256 or 1024 million pixels), a larger one smaller by one factor
  (`picture::within`), and the status bar says so.
- **The levels and the tiles.** The plugin keeps the image with its levels, each half of the one
  before (the mean of two by two, weighted by alpha), down to one of 2048 pixels a side, the
  smallest texture every renderer takes. The view draws the tiles of 1024 pixels of the smallest
  level with a pixel for every pixel of the screen (`view::level`), each with a pixel of its
  neighbours so that a smooth zoom shows no seam, and under a tile still to come the last level,
  whose tiles are made first; at most 6 new tiles a frame, 64 kept, the ones drawn least lately
  go first. A region decoded with `CopyPixels` of a rectangle, the plan of the study, would decode
  a JPEG from its start for every tile; the image is decoded once instead.
- **The frames of a GIF.** WIC gives the frames as the file holds them; the plugin draws each on
  the canvas of the file (`compose`: its place, its disposal of 0 to 3 and its delay from the
  metadata of WIC, a delay of 0 or 1 hundredth as 100 ms as browsers show it, the transparent
  index from the palette of WIC), at most 512 MB of frames. The animation plays when it opens (a
  setting), Space plays and pauses it, the comma and the full stop step through its frames. A GIF
  above 2048 pixels a side shows its frames as pages.
- **The pages** of a TIFF or an ICO of several turn with the same keys and the buttons of the bar,
  each page decoded when it is asked for; an icon opens at its largest frame.
- **The orientation of EXIF** (`System.Photo.Orientation` of the metadata of WIC, a setting) turns
  the pixels in memory (`Picture::oriented`, the eight values).
- **The keys of the zoom**: + and - by the steps of browsers (`view::LEVELS`), 0 fits, 1 to 4 show
  100% to 400%, the arrows move an image larger than the view (else Left and Right step through
  the folder, Page Up and Page Down always do).
- **The tests** make their files: `sample` writes a BMP and a GIF (LZW of nine bits a code), the
  tests of WIC a TIFF of two pages with an orientation through the encoder of WIC, and the tests
  of wview take `sample` with the feature `library`.

To come: the crate `image` (in wwrt already for PNG) where WIC has no decoder.

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

Done in phase 4 (`wview/plugins/wv-binary`):

- **The language** of the description above, checked whole before a file is read by it (strict
  JSON, an unknown field or type, a wrong expression, an enum or flags of no names, raw bytes
  without a size are messages), with: integers of 1 to 8 bytes signed or not in either order
  (`endian` of the description or of a field), `f32` and `f64`, `text` (one byte a character,
  ASCII) and `utf16`, of a `size` or ending in a zero, `raw`, structures, arrays of a `count` or
  `terminated` by an item of zeros, `at` and `rva`, `when`, `switch` with its `cases` (the
  optional header of PE32 or PE32+ by `u16(@here)`), `value` (a number computed of the fields
  before it, the ordinal of an import), `key` (one path or several, the first an item has:
  `["function.name", "ordinal"]`), `enum`, `flags` (the names of the bits set, the rest in
  hexadecimal) and `show` (`hex`, `time` for seconds since 1970). Expressions (`expr.rs`) have
  the operators of Rust in its precedence, `@here`, `@offset`, `@size`, `@end`, `@count` and the
  reads `u8()` to `u64()`; a path names a field read so far in the structure or the nearest one
  above that has it.
- **PE**: the DOS header with its stub, the NT headers, the optional header of PE32 and PE32+ with
  its sixteen data directories, the sections with their bytes, the exports (the name of the DLL,
  the names, their ordinals, the functions) and the imports (each DLL with its functions by name
  or ordinal). `kernel32.dll` of Windows 11 reads into 15917 nodes without an error.
- **The tree is read at the open**, within its limits (500000 nodes, 100000 items an array, 32
  structures deep, 4096 bytes a text), and the bytes stay in the file, read through `BlockFile`
  (the block and the cache are settings); a node that stops at a limit or past the end of the
  file is kept with the reason. A `lazy` field (phase 5) is read when its node opens.
- **The views**: the tree on the left (`ww::tree`, its keys) and the bytes on the right (`hex.rs`:
  8, 16 or 32 a row, the rows in view alone read, the bytes of the node selected marked, a click on
  a byte selects the smallest node that holds it), or **Text**, the tree as a Markdown page of
  `ww::mdview` with its search (at most 20000 nodes). The root is the file: its name, its format
  and all its bytes.
- **Descriptions of others**: the `.wwformat` files of the folder `formats` next to the DLL of the
  plugin (found by `GetModuleHandleExW` of an address of the plugin) add formats, at most 100 of
  1 MB each; one of the name of the built-in one replaces it. Settings, Plugins lists the formats
  read and the files that did not read with the reason. A file opens by the extension and the
  `magic` of a description; a file of the extension without its magic stays closed with the
  reason.

Done in phase 5:

- **The language** got `lazy` (a structure at an offset or an rva read when its node opens:
  `engine::expand`, and `expand_all` before a save), `align` (a field or every item of an array
  ends at a multiple of so many bytes of the file: the names of the streams of .NET, the
  certificates) and `length` (the items of an array until they fill so many bytes: the
  relocations, the certificates); a value with names (`enum`) names an item as its key (`ICON`).
- **The rest of PE**: the resources in their three levels (types, names, languages: two levels
  `lazy`, a name of UTF-16 or an id, the data entry with its bytes), the exceptions of x64
  (`RUNTIME_FUNCTION`), the certificates (a file offset, items aligned to 8), the relocations (the
  blocks of the directory with their entries), the debug directory (the CodeView `RSDS` with the
  path of the PDB, the bytes of every entry), TLS of PE32 and PE32+ with its callbacks (a virtual
  address less the image base), the load configuration of both (a field there when the size of
  the structure reaches its end), the delay imports (their DLL and functions) and the header of
  .NET (`IMAGE_COR20_HEADER`, the root of its metadata `BSJB` with its version and the headers of
  its streams with their bytes). `kernel32.dll` of Windows 11 reads into 23090 nodes at the open,
  and whole without an error; `System.dll` of .NET Framework shows its streams.
- **`.wwbin`** (`wwbin.rs`, the tool Save the structure, next to the file by default through the
  dialog of rfd): every node read first, then a document of JSON: `format` `wwbin`, `version` 1,
  `by` (the plugin and its version), `file` (its name, size and SHA-256), `description` (its name,
  version and the SHA-256 of its text) and `tree` (every node: `name`, `key`, `offset`, `size`,
  and `type`, `items`, `value` (a number of JSON, 64 bits whole), `text` or `sha256` of raw bytes,
  `names` of an enum or flags, `error` with the reason in the language of the window, and
  `children`). SHA-256 is the crate `sha2`, the plugin's own: wwrt holds none.

Done in phase 6:

- **A `.wwbin` opens** alone as its tree or its page (`wwbin::read`, at most 512 MB and two
  million nodes, back into an `engine::Tree`: a raw range keeps its SHA-256 as its note).
- **Compare with a structure** (`diff.rs`, `compareview.rs`): the file read whole, the SHA-256 of
  its raw ranges, and the two trees matched among siblings by the key of a node, else its name
  (`[3]` of an item of no key), the second of one name `name#2`; a line a node of either tree with
  its path (`sections[.text].characteristics`) and its mark: the same, changed (a value, a text,
  the SHA-256 or the size of raw bytes, the count of an array; a structure with a change under it),
  added in the file, removed from it; an offset alone is no change, and a number of the structure
  shows the way the file shows it. The view opens at the changes (every structure with one open,
  the first selected), with Changed only (or All nodes), the values before and after, the offsets
  and the sizes of both, a comment and a bookmark of the line, and Show in the tree and bytes. The
  status counts the values changed, added and removed.
- **`.wwbindiff`** (Save the comparison; opening one reads its file and its structure and shows the
  comparison in its tab): `format` `wwbindiff`, `version` 1, `file` and `structure` (relative to
  its folder inside it, else absolute) and `notes`, each a `key` (the path of a line), a `comment`
  and a `bookmark`; strict JSON of at most 16 MB.
- `wv_binary::sample` (the feature `library`) saves a structure and a comparison for the tests of
  wview without their dialogs.

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
- **A second workspace** (the decision of the customer of 04.11.2026). `-C prefer-dynamic` is a
  flag of a whole build and the release profile of WinWeaver has LTO, so the family lives in a
  workspace of its own next to the one of WinWeaver (the study is one), with the same versions of
  the shared crates (its `Cargo.lock` starts as a copy and a check of CI compares them). One
  workspace would need `--config` with the flag and a profile of its own for every build of the
  family, and `--exclude` of the family in every other, a trap for each new command.
- **CI and build-all.cmd** (phase 2): the Linux job checks the format of `wview` and its clippy for
  Windows; a Windows job of its own, beside the one of WinWeaver, runs its clippy and tests (the
  window drawn into the screenshots `v1-` to `v12-`), builds it in release in the tree of its release
  (section 6.7), packs `wview-<version>` (wview.exe, wwrt.dll, the `std-<hash>.dll` of the
  toolchain, the plugins, README) and `wview-sdk-<version>`, and builds the template of the SDK from
  a copy in another folder, which wview must load. That build takes the target folder of the tree:
  a crate of the registry has the same hash wherever its user is, so only the crates of the SDK,
  cleaned first, are built from the copy: 16 s instead of 7 minutes. With the two jobs side by
  side a run took 42 minutes instead of 70 to 83 (04.11.2026: the job of WinWeaver 41, the one of
  wview 26 without a cache of its own). `build-all.cmd` makes the same two archives (`nowview`
  leaves them out).
- **Tests.** wwrt and every plugin have their unit tests; the tests of the window need
  `egui_kittest` inside wwrt (a feature for the tests), since a dev-dependency of its own on
  kittest would bring a second egui.

### 6.6 Phases

1. The study: the runtime, the loader with its check, two plugins, the measurements. Done.
2. wview as a program of the repository and the native SDK: its workspace, its archive in CI and
   build-all.cmd, the version of the native interface, the template workspace of a plugin, the
   settings of the plugins, the keys and the mouse of a view (section 6.8), the translations, the
   guide, the tests of the window. Done: the workspace `wview/`, the interface 1, the settings,
   the translations in the eight languages of WinWeaver, the keys and the mouse of the images, the
   guide, the page of the plugins as a tab, the SDK with its template (finding 12), wview and the
   SDK in CI and build-all.cmd, the tests of the window with their screenshots.
3. Images whole: tiles, the frames of a GIF, EXIF, the keys of the zoom. Done: the decoding in
   a thread, the levels and their tiles, the frames of a GIF and the pages of a TIFF or an ICO,
   the orientation of EXIF, the steps of the zoom and the arrows that move an image, the tests of
   WIC on files the tests make, the screenshots `v6-` to `v8-` (section 6.2).
4. wv-binary 1: the engine with its language, the headers, sections, imports and exports of PE, the
   hexadecimal and the text views. Done (section 6.4): the language, PE in its first parts, the
   tree, the bytes and the text, the descriptions of the folder `formats`, the settings, the
   eight languages and the guide, the screenshots `v9-` and `v10-`.
5. wv-binary 2: the rest of PE, `.wwbin`. Done (section 6.4): `lazy`, `align` and `length`, the
   resources, the exceptions, the certificates, the relocations, debug, TLS, the load
   configuration, the delay imports and .NET, Save the structure, the screenshot `v11-`.
6. `.wwbindiff`: the tree of the differences, the comments and the bookmarks. Done (section 6.4):
   the comparison of a file with its `.wwbin`, the notes in a `.wwbindiff` that opens in a tab, a
   `.wwbin` opened alone, the screenshot `v12-`.
7. The stable interface for other languages (section 6.9) and its one example, the Mandelbrot set
   (section 6.10).

### 6.7 Plugins of others: the hybrid

The decision of the customer of 04.11.2026: two kinds of plugins, one of them the main.

1. **Native plugins with the SDK of a release, the main way.** Every example is one: text and
   Markdown, images, binary structures. Every release of wview publishes what its build was:
   `wview-sdk-<version>.zip` (`wview/sdk/pack.py`, its README `wview/sdk/README.md`) is the tree
   the release itself is built in: W# in `framework/` (with the fields it takes from the workspace
   of WinWeaver written out), wwrt and wview in `crates/`, the plugins and the template `wv-hello`
   in `plugins/`, `Cargo.lock`, the flags (`.cargo/config.toml`) and the compiler
   (`rust-toolchain.toml` with the version of the release). rustc names the symbols of `wwrt.dll` by
   the versions, the features and the flags of its crates and by their sources, and cargo hashes a
   path dependency relative to the root of the workspace only when it is inside it, else by its
   absolute path. In the repository W# is outside `wview/`, so a plugin built from a copy of the
   repository layout in another folder missed the imports of `wwrt.dll` (the hash of `ww` was
   `Csl8FSR6xLGnH` in one build and `CsluDd2lLouDL` in the other, the ones of egui alike); in the
   tree every crate is inside its root, and a plugin built in a copy of it anywhere loads. A plugin
   made with it draws with egui as the plugins of the repository do and is built again for every
   release, since the loader refuses it otherwise (finding 5); CI builds the template of the SDK in
   another folder and loads it into wview.
2. **A stable interface for data, for other languages.** A C ABI of versioned structures (section
   6.9): a plugin in C, C++, Zig, Go, Delphi or Rust of any version gives the host a picture and the
   controls around it, and the host draws them and hands the plugin the keys and the mouse. It
   outlives the releases. One example shows it: the Mandelbrot set (section 6.10).

Besides them a `.wwformat` of wv-binary (section 6.4) adds a binary format without code: data,
read with the limits of every untrusted input (rule 8). A plugin nobody vouches for could later run
as WebAssembly in a sandbox, behind the same stable interface and hosted by a native plugin
(`wv-wasm`), so that wview carries a runtime of WebAssembly only where that plugin is installed.

A plugin of others, of either kind, has the rights of the program (finding 9), and one in C can
crash it. The page of the plugins names where each one comes from, and a setting may allow only
the plugins whose SHA-256 or signature (Authenticode, checked with `WinVerifyTrust`) it lists.

### 6.8 The keys and the mouse of a view

Both kinds of plugins need the same rules, so they are part of phase 2:

- **Focus.** A click on a view gives it the focus of the keyboard; Tab and Escape give it back to
  the window. A view in focus gets the keys it declares first (the arrows, the page keys, Home and
  End, letters, everything); the others go to the window, so Ctrl+O, Ctrl+W, Ctrl+Tab and the menu
  keep working.
- **The mouse.** The position in the pixels of the view, the buttons, the wheel (with Ctrl, Shift
  and Alt), the movement while a button is down, also outside the view until the button is up, the
  double click, and the pointer leaving the view.
- **The cursor.** A view names its cursor (the default, a cross, a hand, a closed hand while
  dragging, a loupe), the window sets it.
- **A frame on demand.** A view that is still at work (a picture drawn in steps) asks for another
  frame; an idle one costs nothing.

### 6.9 The stable interface

`wview.h`, a header of C and its version, and a crate of Rust over it for authors who write Rust
of another toolchain. A plugin exports one function, `wv_plugin_get`, which gives a table of
functions and data; a native plugin exports `ww_plugin` instead, so the loader tells them apart. In
short:

```c
#define WV_API_VERSION 1

typedef struct wv_format {
    const char *name;                 /* UTF-8, for the filters of Open */
    const char *const *extensions;    /* without the dot, lower case */
    uint32_t extension_count;
} wv_format;

typedef struct wv_key {
    const char *name;                 /* the W3C name of the key: "ArrowLeft", "PageUp", "+" */
    uint32_t pressed;                 /* 1 pressed, 0 released */
    uint32_t modifiers;               /* WV_CTRL | WV_SHIFT | WV_ALT */
} wv_key;

typedef struct wv_input {
    uint32_t size;                    /* sizeof(wv_input): newer hosts add fields at the end */
    uint32_t width, height;           /* the area of the view, pixels */
    float scale;                      /* pixels a point of the window */
    uint32_t focused, hovered;
    float mouse_x, mouse_y;           /* pixels of the view */
    uint32_t buttons;                 /* WV_LEFT | WV_RIGHT | WV_MIDDLE, held now */
    uint32_t clicks, double_clicks;   /* buttons clicked since the last frame */
    float drag_x, drag_y;             /* movement while a button was down */
    float wheel_x, wheel_y;           /* lines since the last frame */
    uint32_t modifiers;
    const wv_key *keys;               /* the keys of the view since the last frame */
    uint32_t key_count;
    const char *text;                 /* text typed since the last frame, UTF-8 */
} wv_input;

typedef struct wv_control {
    uint32_t id;
    uint32_t kind;                    /* WV_BUTTON, WV_TOGGLE, WV_SLIDER, WV_CHOICE, WV_LABEL */
    const char *label;
    double value, min, max, step;     /* a slider; a toggle 0 or 1; a choice its index */
    const char *const *choices;
    uint32_t choice_count;
} wv_control;

typedef struct wv_output {
    uint32_t size;
    uint32_t width, height, stride;   /* the picture, RGBA with 8 bits a channel */
    const uint8_t *pixels;            /* the plugin's memory, valid until its next call */
    uint32_t changed;                 /* the picture differs from the last one */
    const wv_control *controls;       /* the controls of the bar of the tab */
    uint32_t control_count;
    uint32_t keys;                    /* WV_KEYS_ARROWS | WV_KEYS_PAGES | WV_KEYS_LETTERS | ... */
    uint32_t cursor;                  /* WV_CURSOR_DEFAULT, _CROSS, _HAND, _GRAB, _ZOOM */
    uint32_t again;                   /* draw another frame: the plugin is still at work */
    const char *status;               /* a line of the status bar, UTF-8 */
} wv_output;

typedef struct wv_plugin {
    uint32_t api_version;             /* WV_API_VERSION the plugin was written for */
    const char *name;
    const wv_format *formats;
    uint32_t format_count;
    void *(*open)(const char *path, char *error, uint32_t error_size);
    void (*frame)(void *view, const wv_input *input, wv_output *output);
    void (*control)(void *view, uint32_t id, double value);
    void (*close)(void *view);
} wv_plugin;

__declspec(dllexport) const wv_plugin *wv_plugin_get(void);
```

The rules: every call comes from the thread of the window and every text is UTF-8; what the host
gives lives for the call, what the plugin gives lives until its next call on that view; a plugin
may run threads of its own; a host refuses an `api_version` it does not know and reads only the
fields a `size` covers, so both sides grow at the end. In the host the loader of wwrt wraps such a
DLL into an ordinary `Plugin` and `View` (`wwrt::stable`): the picture becomes a texture, the
controls egui widgets in the bar, the input of egui becomes `wv_input`.

### 6.10 The example of the stable interface: the Mandelbrot set

One plugin in C, `wv-mandelbrot`, built by CI with the compiler of Visual Studio. It opens a
`.mandel` file, a few lines of text with the centre, the zoom, the iterations and the palette of a
view, and saves the view back. Its controls in the bar: Iterations (a slider), Palette (a choice:
classic, fire, ice, grey), Smooth colours (a toggle), Zoom in, Zoom out, Reset. Its mouse: the wheel
zooms at the pointer, a drag moves the picture with the closed hand, Shift and a drag draws a
rectangle that a release zooms into, a double click centres and zooms in, the right button zooms
out. Its keys: the arrows move, + and - zoom, Page Up and Page Down double or halve the iterations,
P takes the next palette, Home resets, Escape drops a rectangle. It draws in steps: a coarse picture
at once, then finer rows while it asks for frames, so the window never waits; the status bar names
the centre, the zoom, the iterations and the time of the last picture.

## 7. Open questions

- Answered by CI on 04.11.2026 (the toolchain msvc): `wview.exe` 831 KB, `wwrt.dll` 23.1 MB,
  `std-<hash>.dll` 864 KB, `wv_text.dll` 53 KB, `wv_image.dll` 93 KB; the template of the SDK built
  in another folder loaded in 0.7 ms. The `std-<hash>.dll` of msvc imports no DLL of the CRT, but
  with the dynamic CRT every module of the family imported `VCRUNTIME140.dll`, which a clean
  Windows may lack; the family takes the CRT statically since then, as WinWeaver does
  (`wview/.cargo/config.toml`). The next run showed it (`wview/sdk/imports.py`): every module
  imports `std-<hash>.dll`, `wwrt.dll` and DLLs of Windows alone, at the price of about 100 KB in
  `wview.exe` (929 KB) and 200 KB in `wwrt.dll` (23.3 MB); the template of the SDK loaded in 0.9 ms.
- The language of the example of the stable interface: C proves that every language with a C FFI
  can write a plugin; it needs a compiler of C, which CI has (Visual Studio on the runner) and the
  computer of the developer has not (a toolchain there only on the word of its owner, AGENTS.md
  rule 6). Rust of another toolchain over the crate of `wview.h` needs no new tool but proves less.
- WinWeaver and wedit could move onto wwrt later; nothing in them needs it.

## 8. Building wview

On a computer of Windows with Rust, in `wview`:

```bat
cargo build --release --workspace
```

`target\release` then holds `wview.exe`, `wwrt.dll`, `wv_text.dll`, `wv_image.dll` and
`wv_binary.dll`. A folder to
copy anywhere: `wview.exe` and `wwrt.dll`, `std-<hash>.dll` of the toolchain
(`rustc --print sysroot`, then `bin` for gnu or `lib\rustlib\<target>\lib`), with gnu also
`libgcc_s_seh-1.dll` and `libwinpthread-1.dll` of its `bin`, and the plugins in `plugins\`. Then
`wview --plugins` lists them, `wview --probe FILE` opens a file without a window and `wview FILE`
shows it.
