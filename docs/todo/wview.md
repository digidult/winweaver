# wview: what stays to come

The current state of wview and its plugins is in `../plugins.md`; these are the parts planned.

## The host

- The stable interface 2 (`wview.h`): the MIME types of the formats, the items of the menu and
  bytes in memory, when a plugin of another language needs them; it stays at version 1 now.
- Later, a plugin nobody vouches for run as WebAssembly in a sandbox behind the stable interface,
  hosted by a native plugin `wv-wasm`, so that wview carries a runtime of WebAssembly only where
  that plugin is installed.

## The plugins

- Binary structures: more formats as descriptions; more views of the tree.

## The family

WinWeaver and wedit could move onto wwrt later; nothing in them needs it now.
