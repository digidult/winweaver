# wview: what stays to come

The current state of wview and its plugins is in `../plugins.md`; these are the parts planned.

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
