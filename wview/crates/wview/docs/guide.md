# wview

wview shows files. Every format is a plugin: a DLL in the folder `plugins` next to `wview.exe`.
Without a file the window lists the plugins and what each of them opens.

## Opening files

- **Open** (Ctrl+O), or a file dropped on the window, opens the file in a tab; `wview FILE...`
  opens files from the command line.
- A file of an extension no plugin names, or of none, opens by what it holds: a text, an image, a
  program.
- A file of a format no plugin opens stays closed, and the status bar says so.
- Ctrl+W closes a tab, Ctrl+Tab goes to the next one.
- The menu holds the items of the tab in front and a submenu of every plugin that has items of its
  own.

## Text and Markdown

Markdown pages (`.md`) show as pages, text files (`.txt`, `.log`, `.ini`, `.cfg`, `.csv`) as they
are. Ctrl+F searches the text. **Settings**, **General**: *Wrap lines to the width of the view* and
*Width of pages* change how a text shows.

XML files (`.xml`, `.manifest`) show as texts too, and a text of UTF-16 with its mark reads as well
as one of UTF-8.

A file larger than **Largest file read whole** of the settings (64 MB at first) opens in parts: its
lines are counted while it is read, with their numbers on the left; the wheel, the arrows, Page Up
and Page Down, Home, End and the bar on the right move through it, Left and Right move sideways. The
search lists the lines that hold every word, and a click on one goes to it.

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
- Besides the headers: the resources (their types, names and languages, a level read when it
  opens), the exceptions of x64, the certificates, the relocations, the debug directory with the
  path of the PDB, TLS, the load configuration, the delay imports and the header of .NET with the
  streams of its metadata.
- **Save the structure** writes the tree as a `.wwbin` file, next to the file by default: every
  field with its value, every range of raw bytes with its SHA-256, the file with its size and
  SHA-256, and none of its bytes.
- **Compare with a structure** compares the file with a `.wwbin` saved of it (of another version
  of the file, say): the tree of the changes, every one with its value before and after, the nodes
  matched by their names and keys, so a new section moves no other; **All nodes** or **Changed
  only**. A node selected takes a comment and a bookmark; **Save the comparison** writes them with
  both files to a `.wwbindiff`, which opens the comparison in a tab of its own. A `.wwbin` opens
  alone as its tree.
- **Content** in the bar, when the node selected holds a file of a type another plugin shows (the
  manifest of a program, its icons, bitmaps and HTML pages, a picture or a text among its data,
  another program inside it): the right side shows it with that plugin, with its own buttons over
  it, and follows the selection; **Tree and bytes** goes back. In the menu:
  **Open the content in a tab**, **Save the node as a file** (an icon as an ICO file, else its
  bytes) and **Read every node**.
- A group of icons (`GROUP_ICON`) shows as one icon of every size, its sizes as pages, and saves as
  one ICO file.
- More formats come as descriptions, `.wwformat` files in the folder `plugins\formats`; Settings,
  Plugins lists the formats read and the files that did not read.
- **Binary structures**, **Folder of the descriptions** in the menu opens the folder
  `plugins\formats` in Explorer, made when it is not there.

## The Mandelbrot set

The example of a plugin in another language than Rust: C, through the stable interface of wview.
A `.mandel` file, a few lines with the centre, the zoom, the iterations, the palette and the smooth
colours, opens as the picture of the set; the archive holds `examples\seahorse.mandel`.

- Over the picture: **Iterations**, **Palette** (classic, fire, ice, grey), **Smooth colours**,
  **Zoom in**, **Zoom out**, **Reset**, and **Save**, which writes the view back to its file.
- The wheel zooms at the pointer, a drag moves the picture, Shift and a drag draws a rectangle that
  the release zooms into, a double click centres and zooms in, the right button zooms out.
- After a click on the picture: the arrows move it, + and - zoom, Page Up and Page Down double or
  halve the iterations, P takes the next palette, Home resets, Escape drops a rectangle.
- The picture comes in steps, coarse at once and finer while it is drawn; the status bar names the
  centre, the zoom, the iterations and the time of the last picture.

## Settings

**Settings**, **Plugins** lists the plugins with their settings: the largest text file read whole, a
larger one opening in parts; smooth pixels above 100%, the orientation of EXIF, an animation played
when it opens and the largest image decoded whole (64, 256 or 1024 megapixels); the bytes in a row,
the block of a binary file and the cache of its blocks. The settings are kept in
`wview.settings.json` next to the program.

## Command line

- `wview FILE...` opens the files in tabs.
- `wview --plugins` lists the plugins and their formats.
- `wview --probe FILE` opens a file and draws it without a window, with the times.
- `wview --help` lists the options.
