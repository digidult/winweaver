# Plugins with MIME types, menu items and other plugins inside: the design to approve

The customer's list of 05.11.2026, item 3: a plugin, besides the extensions of the files it opens,
names MIME types of its own, adds items of its own to the menu, and can show other plugins inside
its view; the example is a text or an image of a segment of an exe shown inside the view of the
exe.

## The native interface, version 2

The interface of the native plugins (`wwrt::plugin`, version 1 now) grows to version 2; the plugins
of a release are built with it anyway, the SDK and its template `wv-hello` change with it. The
stable interface (`wview.h`) stays at version 1: these parts come to it later as a version 2 of its
own, when a plugin of another language needs them.

- **MIME types.** `Format` gets `mimes`, the MIME types of the format beside its extensions:
  `text/plain`, `text/markdown`, `application/xml` of the plugin of texts; the types each decoder of
  WIC names (`IWICBitmapCodecInfo::GetMimeTypes`: `image/png`, `image/jpeg`...) of the plugin of
  images; `application/vnd.microsoft.portable-executable` of the binary structures (a description
  names its own in a field `mimes`). The host finds a plugin by a MIME type as it finds one by an
  extension.
- **Menu items.** `Plugin::menu` gives items (an id, a label, an icon) shown in the menu of the
  window in a submenu of the plugin, and `View::menu` items of the tab in front; a click comes back
  as `Plugin::act_menu` or `View::act_menu` with the `Host`. The plugin of binary structures, say:
  Descriptions of formats (the folder `plugins\formats` in Explorer) and, for its view, Compare with
  a structure and Save the structure.
- **Plugins inside a view.** `Plugin::open_bytes(name, mime, bytes, settings)` opens bytes in
  memory, not a file (a plugin that does not answers that it cannot), and the `Host` gets
  `open_inside(name, mime, bytes)`: the host picks the plugin of the MIME type and gives the view
  that asked a view of its own, which it draws in a part of its area with `ui.push_id` (two views
  of one kind keep apart), with the tools of the inner view as buttons over it; the inner view gets
  the keys and the mouse of its part by the usual rules. A view inside a view inside a view stops
  at a depth of 4, so an exe that holds itself cannot loop.
- **The type of some bytes.** `wwrt::sniff(bytes)` names the MIME type of common files by their
  first bytes: PNG, JPEG, GIF, BMP, ICO, TIFF, WebP, PE, ZIP, PDF, XML, and text of UTF-8 or UTF-16
  with or without a mark.

## The example: what an exe holds, shown inside it

In the view of a binary file a node whose bytes are a known type gets **Content** in the bar: the
right part shows them with the plugin of their type in place of the bytes (**Tree and bytes**
again goes back), with a line over it that names the type, the size and the plugin.

- The resources of PE: a `MANIFEST` (XML) and an `HTML` as text; a `BITMAP` as an image (the
  plugin adds the header of a BMP file it lacks); an `RCDATA` or any data whose first bytes are a
  PNG, a JPEG, a GIF or a text by `sniff`. Icons (`GROUP_ICON` with its `ICON` entries put together
  as an ICO file) come after.
- Any raw range of a description (a section, a stream of .NET) by `sniff`.
- `Plugin::open_bytes` of the plugins of the release: texts and Markdown from the bytes, images
  through a stream of WIC in memory (`IWICStream::InitializeFromMemory`), binary structures through
  the `Source` of bytes it has already.

## Tests and the screenshots

The tests open `notepad.exe` of the Windows they run on (read only): its manifest shown as text, an
image of its resources, the depth limit with a file that holds itself; a screenshot of the content
of a resource inside the view of the exe.

## Questions for the customer

1. The native interface goes to version 2 (plugins of a release are built again anyway), and the
   stable one stays at 1 for now: right?
2. The example as above (resources and raw ranges of an exe, with Content in the bar): right, or
   another place?
3. The menu items: a submenu of each plugin in the menu of the window, and the items of the view in
   front: enough?
