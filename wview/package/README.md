# wview

A viewer of files: Markdown pages, text and XML files, programs and libraries of Windows, and every
image format the Windows it runs on
decodes (BMP, PNG, JPEG, GIF, TIFF, ICO, and HEIF, AVIF, WebP or the RAW of cameras where their
extensions are installed), with WebP, TGA, Netpbm, QOI, Radiance HDR and farbfeld where it has no
decoder. Every format is a plugin, a DLL in the folder `plugins`.

## Start

Unpack the folder anywhere and start `wview.exe`; nothing is installed. Open a file with Open
(Ctrl+O), drop it on the window or name it on the command line; a file of an extension no
plugin knows, or of none, opens by what it holds. The menu has the guide; the
settings are kept in `wview.settings.json` next to the program.

## The folder

- `wview.exe`, the program.
- `wwrt.dll`, the runtime the program and its plugins share, and `std-<hash>.dll`, the standard
  library of Rust: they stay next to `wview.exe`.
- `plugins\wv_text.dll` (Markdown and text), `plugins\wv_image.dll` (images, the animations of
  GIF, the pages of TIFF and ICO) and `plugins\wv_binary.dll` (programs and libraries of Windows
  as the tree of their structure with their bytes, saved as `.wwbin` on demand and compared with
  one, the notes of a comparison in a `.wwbindiff`, the manifest, the icons and the bitmaps
  inside a program shown by the plugins of texts and images; more formats as `.wwformat`
  descriptions in `plugins\formats`).

A plugin of another release of wview is not loaded: the page of the plugins and `wview --plugins`
name the reason.

## Plugins of one's own

The SDK of the same release (`wview-sdk-<version>.zip`) makes native ones: its README tells how.
A plugin in another language (C, C++, Zig, Go, Delphi, or Rust of any toolchain with the crate
`stable\rust` of the SDK) is written to the stable interface, `stable\wview.h` of the SDK, and
loads into every later release; `plugins\wv_mandelbrot.dll` is
its example, in C: open `examples\seahorse.mandel`.

## Command line

- `wview FILE...` opens the files in tabs.
- `wview --plugins` lists the plugins and their formats.
- `wview --probe FILE` opens a file and draws it without a window, with the times.
- `wview --help` lists the options.
