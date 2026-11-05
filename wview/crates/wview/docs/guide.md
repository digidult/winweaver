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
  of the same folder; for an animation **Play** (or **Pause**), **Previous frame** and **Next
  frame**; for a file of several pages (TIFF, ICO) **Previous page** and **Next page**.
- A click on the image gives it the keys: the arrows move an image larger than the window, else
  Left and Right take the previous or the next image of the folder, as Page Up and Page Down always
  do; Home and End the first and the last; + and - zoom by steps, 0 fits the image, 1 to 4 show
  100% to 400%; Space plays or pauses an animation, the comma and the full stop go to the frame or
  the page before or after. Tab or Escape gives the keys back to the window.
- A photo is turned the way its camera was held (the orientation of EXIF). A large image is
  decoded while its tab is open already and is drawn in parts at the detail the zoom needs; an
  image above the limit of the settings is decoded smaller, and the status bar says so.

## Binary files

Programs and libraries of Windows (`.exe`, `.dll`, `.sys` and the other files of the Portable
Executable) open as the tree of their structure: the headers, the sections, the imports and the
exports, every field with its value, its offset and its size.

- On the left the tree: a click or the arrows select a node, Right and Left open and close it. On
  the right the bytes of the file, 16 a row with their ASCII: the bytes of the node selected are
  marked, and a click on a byte selects the smallest node that holds it.
- **Text** in the bar shows the tree as a page, with the search of the tab (Ctrl+F); **Tree and
  bytes** goes back.
- The file stays on the disk: the view reads the blocks it shows. A node the file does not hold
  whole (a short or damaged file) has a warning sign and says why.
- More formats come as descriptions, `.wwformat` files in the folder `plugins\formats`; Settings,
  Plugins lists the formats read and the files that did not read.

## Settings

**Settings**, **Plugins** lists the plugins with their settings: the largest text file to open;
smooth pixels above 100%, the orientation of EXIF, an animation played when it opens and the largest
image decoded whole (64, 256 or 1024 megapixels); the bytes in a row, the block of a binary file and
the cache of its blocks. The settings are kept in `wview.settings.json` next to the program.

## Command line

- `wview FILE...` opens the files in tabs.
- `wview --plugins` lists the plugins and their formats.
- `wview --probe FILE` opens a file and draws it without a window, with the times.
- `wview --help` lists the options.
