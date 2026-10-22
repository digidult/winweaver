# wedit

A small portable text editor in the look of WinWeaver, built on the W# framework: a window with
tabs, a monospace editor for `.txt` and any text file, a viewer of `.md` pages.

## Start

Windows 10 or 11, 64-bit. Nothing is installed: unpack the archive into a folder and run `wedit.exe`.
The program keeps its settings and files in that folder: `settings.json`, `drafts/` (unsaved
changes, restored at the next start), `logs/`, `themes/` and `lang/`. Drop a file on the window to
open it, or name files on the command line: `wedit notes.txt readme.md`.

## What it does

- **Text tabs**: Ctrl+N a new file, Ctrl+O open, Ctrl+S save, Ctrl+Shift+S save as, Ctrl+W close.
  Undo with Ctrl+Z. Find with Ctrl+F; F3 selects the next match, Shift+F3 the previous one. The
  status bar shows the line and the column. A file that ends its lines with CR LF keeps them.
- **Markdown pages**: a `.md` file opens as a page with the search of its text; Edit as text opens
  the same file in a text tab.
- **Drafts**: a change is kept two seconds later in `drafts/` and comes back at the next start; the
  tabs of the last session open again (Settings, General).
- **Settings** (Ctrl+,): the language, the theme (Light, Dark, as Windows shows its apps, or a
  theme file), its fonts, the renderer.
- **Themes**: Light and Dark are built in; a `.wwtheme` file in `themes/` adds one, and Edit the
  theme in the settings opens the editor.
- **Languages**: English is built in; a file `lang/<code>/ui.json` adds a language. The archive
  ships `lang/uk/ui.json`, Ukrainian, as the example: copy it, translate the values, keep the keys.

## Command line

```
wedit [FILE...]             open the window, with the files in tabs
wedit --renderer auto|opengl|directx [FILE...]
                            open the window, trying that renderer first (this start only)
wedit --smoke-window        open a small window, draw two frames, print what drew them
wedit --version             print the version
wedit --help                this text
```

## License

MIT, as WinWeaver and W#. Source: https://github.com/digidult/winweaver, `crates/wedit`.
