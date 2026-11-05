# wview: what stays to come

The current state of wview and its plugins is in `../plugins.md`; these are the parts planned.

## A crate of Rust over `wview.h`

For authors who write a plugin of the stable interface in Rust of another toolchain than the one of
a release (`wview/stable/wview.h`, version 1).

- `wview/stable/rust`, the crate `wview-stable`: no dependency, edition 2021, a low rust-version,
  its own fields (no `.workspace = true`), so it copies out whole; a member of the workspace of
  wview, and of the tree of the SDK (`wview/sdk/pack.py`).
- `ffi`: the `#[repr(C)]` structures and the constants of `wview.h`. Over them a trait `View`
  (`frame` from an `Input` to a `Frame`: a picture, controls, the groups of keys, the cursor,
  `again`, a status; `control`), a box per view that keeps the texts and the arrays of controls
  until the next call, the functions of the table, and a macro `stable_plugin!` (the name, the
  formats, the type of the view, the function that opens a file) that exports `wv_plugin_get`.
- Tests on the functions of the table, and a test of wwrt that wraps the table the crate makes with
  `wwrt::stable::wrap`, so the two sides are checked against each other. No new example: the
  Mandelbrot set in C stays the one example of the stable interface.

## The host

- A file found by its first bytes as well as by its extension (`wwrt::mime::sniff` names the type
  of bytes inside a file already; a file opens by its extension, and a plugin of binary structures
  checks the magic of a description after the extension chose it).
- The stable interface 2 (`wview.h`): the MIME types of the formats, the items of the menu and
  bytes in memory, when a plugin of another language needs them; it stays at version 1 now.
- A setting that loads only the plugins whose SHA-256 or Authenticode signature it lists
  (`WinVerifyTrust`), and the page of the plugins naming where each one comes from.
- Later, a plugin nobody vouches for run as WebAssembly in a sandbox behind the stable interface,
  hosted by a native plugin `wv-wasm`, so that wview carries a runtime of WebAssembly only where
  that plugin is installed.

## The plugins

- Images: the crate `image` (in wwrt already for PNG) where WIC has no decoder.
- Text and Markdown: a file above 64 MB read by `BlockFile` with an index of its lines instead of
  whole.
- Binary structures: more formats as descriptions; more views of the tree; the icons and the
  cursors of the resources put together (a `GROUP_ICON` with its `ICON` entries as one ICO file of
  every size, now each image shows alone).

## The family

WinWeaver and wedit could move onto wwrt later; nothing in them needs it now.
