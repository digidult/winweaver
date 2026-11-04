# wview

wview shows files. Every format is a plugin: a DLL in the folder `plugins` next to `wview.exe`.
Without a file the window lists the plugins and what each of them opens.

## Opening files

- **Open** (Ctrl+O), or a file dropped on the window, opens the file in a tab; `wview FILE...`
  opens files from the command line.
- A file of a format no plugin opens stays closed, and the status bar says so.
- Ctrl+W closes a tab, Ctrl+Tab goes to the next one.

## Text and Markdown

Markdown pages (`.md`) show as pages, text files (`.txt`, `.log`, `.ini`, `.cfg`, `.csv`) as they
are. Ctrl+F searches the text. **Settings**, **General**: *Wrap lines to the width of the view* and
*Width of pages* change how a text shows.

## Images

Every format the Windows of the computer decodes: BMP, PNG, JPEG, GIF, TIFF, ICO, and HEIF, AVIF,
WebP or the RAW of cameras where their extensions are installed.

- The wheel zooms at the pointer, a drag moves the image, a double click goes between the whole
  image and 100%.
- The bar: **Fit**, **100%**, **Zoom in**, **Zoom out**, and **Previous** and **Next**, the images
  of the same folder.
- A click on the image gives it the keys: Left and Right or Page Up and Page Down take the previous
  or the next image of the folder, Home and End the first and the last, + and - zoom, 0 fits the
  image, 1 shows 100%. Tab or Escape gives the keys back to the window.

## Settings

**Settings**, **Plugins** lists the plugins with their settings: the largest text file to open;
smooth pixels above 100% and the largest side of a decoded image. The settings are kept in
`wview.settings.json` next to the program.

## Command line

- `wview FILE...` opens the files in tabs.
- `wview --plugins` lists the plugins and their formats.
- `wview --probe FILE` opens a file and draws it without a window, with the times.
- `wview --help` lists the options.
