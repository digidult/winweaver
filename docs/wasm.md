# wwlite in a browser (WebAssembly)

Item 18 of the customer's list of 24.10.2026 was the study below; item 6 of the list of 25.10.2026
(the second part) built the edition. Rust does not run on the customer's PC (AGENTS.md, rule 6), so
the code was checked and built in CI only; nobody has opened the page in a browser yet.

## 0. What was built

The job `build-web` of CI builds wwlite for `wasm32-unknown-unknown` with the renderer `glow` alone
(WebGL), runs the wasm-bindgen of the version `Cargo.lock` names and uploads `wwlite-web-<version>`:
`wwlite.html`, the edition by default in one file (item 10 of the list of 28.10.2026), the README of
wwlite and `files/` with `index.html` (`crates/wwlite/web/index.html`), `wwlite.js` and
`wwlite_bg.wasm`. `crates/wwlite/web/single.py` makes the one file of `files/`: the glue inlined as a
module script, the module packed with gzip into base64 and unpacked by the browser
(`DecompressionStream`), so it opens from the disk too; it refuses a glue that imports a snippet file.
The folder `files/` is served over http only (a page of it opened from `file://` cannot load the
module). The menu Guide of a page shows `data/docs/guide-web.txt` (Ukrainian:
`data/lang/uk/guide-web.txt`), the guide for someone who knows only the browser edition.

The three blockers of the customer's words, as they were resolved:

1. No logs. `ww::store::log_line` writes the log of the window to the console of the browser
   (`console.log`) and the log of eframe goes there through `WebLogger`; no file is written.
2. No file system. `ww::store` is the one door to the files of the window: natively the files next
   to the executable as before; in a browser the `localStorage` of the page under the key `ww/<path>`
   (the settings `wwlite.settings.json`, the drafts, a theme saved from the fonts of the settings). A
   file to open comes by its bytes: `ShellAct::Open` runs `rfd::AsyncFileDialog` in a task of the page
   (the kinds of `App::open_filters`), a drop on the page is read with `bytes_async`, and the next
   frame hands the file to `App::open_bytes` as `ShellAct::OpenBytes(name, bytes)`; wwlite opens a
   profile, an answer file with its profile or an older profile that way, in a tab without a path. A
   save is a download (`ww::store::download`: a Blob and a link clicked, one click): Save and Save as
   give the profile, Build gives `autounattend.xml` alone (the scripts are embedded in it; the copies
   beside it stay a thing of Windows), Save as of a theme its file. `ww::web::start` replaces
   `ww::run` (eframe's `WebRunner` in the canvas `wwlite`), `winweaver_ui::run_web` calls it from the
   `main` of wwlite. The sync dialogs of rfd, `App::open_dialog`, the title bar colours and the
   fallback of renderers are native only.
3. The lists from the system. A build for wasm32 embeds `data/resources/locales.json`, the input
   languages and keyboard layouts of a Windows 11 (build 26300) read once through the Windows API by a
   PowerShell script that only reads, with the English display names; `resources::locales` takes it
   when no reader is named, under `cfg(target_arch = "wasm32")` alone, so a build for Windows never
   carries it (AGENTS.md, rule 3 and section 6; `02-documents.md` section 3). The regions of the user
   region come the same way from `data/resources/regions.json` (249 nations with their GeoIDs and
   English names, read on 28.10.2026), so the field of the rule `default-user.region` has its list
   in a browser too. Against the native read the lists lack the names in the language of the user's
   Windows and anything a newer Windows adds.

The keys (item 16 of the list of 28.10.2026): a page takes Alt+N for New, Alt+W for Close, Alt+PgDn and Alt+PgUp for the next and the previous tab and Alt+Shift+PgUp/PgDn to move a tab, since a browser keeps Ctrl+N, Ctrl+W, Ctrl+Tab and Ctrl+Shift+PgUp/PgDn for itself (`ww::keys`), and Alt+I for the guide, since F1 opens the help of the browser (`commands::keys::GUIDE`); the page stops the browser from
acting on F2, F3, F7, F9, Ctrl+F, Ctrl+D and Ctrl+B (a listener of `index.html`), eframe does on
Ctrl+O, Ctrl+S and Ctrl+,. What stays open:
nothing asks before the page closes with unsaved changes (the drafts in the storage of the page come
back at the next start), the menu has no recent files and no Exit (a page keeps no paths and cannot
close its tab), the tabs of the last session have no meaning there, the
fonts are the ones of egui (no Segoe UI), the input languages show their English names, and the host
of the page is the customer's decision (section 5). The size of the module is read from the artifact.

## 0a. winweaverweb: WinWeaver in a browser

The list of 09.11.2026 (the second, item 1): `crates/winweaverweb` builds the window of WinWeaver
with the feature `full` of winweaver-ui, without its features `pc` (This PC, the registry of a PC,
the native apply, the accounts of a PC) and `mcp` (the MCP server), for `wasm32-unknown-unknown`
with `glow`. CI (the job `build-web`) and `build-all.cmd` make `winweaverweb-<version>`:
`winweaverweb.html` (one file, `single.py`), `files/` (the page for a server), its `README.md` and
`manifest.py`.

- **Every file of WinWeaver**: Open and a drop take a profile, an answer file, a diff, bookmarks,
  a catalog file, a zip archive of policy templates (`admx::read_archive_bytes`), a users file, a
  Group Policy file, a registry file, a Markdown page, an audit report (`app/web.rs`,
  `App::open_bytes`). A save goes through the file layer of W# (`ww_base::files`): the file
  dialog of a browser (`dialogs::FileDialog`) gives a path under `ww_base::files::DOWNLOAD`, which
  the layer of the page turns into a download, so Save, Save as, Build, the scripts and the
  registry files of a profile work as natively.
- **A catalog opened is imported** (the list of 10.11.2026): a catalog file or an archive of
  templates is loaded at once, as WinWeaver loads the catalog files next to it: saved among the
  imports in the storage of the page, its options in the profiles, its tab in front; one of the
  same id replaces the import. A catalog the storage refuses stays loaded for this page only. The
  editor of a catalog saves a download of its file and keeps an import in the page as well.
- **A file asked for a purpose** (the other side of Compare with a file, the entries a Group
  Policy file adds, the accounts of a users file for a profile, a catalog or an archive of the
  menu Catalogs, an audit report, a Group Policy file of its menu): `WinWeaver::pick_for` asks the
  dialog of the page (rfd in a task of the page) and `take_picked`, after each frame, hands the
  bytes to the purpose. The native dialog answers at once; a page's answers later.
- **The storage of the page**: the settings, the drafts and the bookmarks (`bookmarks.wwmarks`
  through the same layer) stay in `localStorage`; an import saved there too while it fits (a
  browser keeps about 5 MB a site).
- **The folder of the page over http**: `ww::store::preload` reads `files.json` next to the page
  (a list of relative paths, which `manifest.py` writes for `imports/`, `themes/` and
  `lang/<code>/ui.json`) and every file it names before the window starts; the store and the file
  layer read them before the storage of the page, so the catalogs of `imports/` load, the themes
  and the translations are offered. What blocked it: a browser cannot list a folder of a server and
  a page reads files only as tasks, so the list is a file and the files are read before the start;
  a page opened from `file://` reads none (the one-file edition has the built-in ones).
- **Not there**: the menus and the buttons of what needs this PC or its disk are hidden
  (`act::needs_pc`, `act::offered`): This PC, the registry tab and every link to it, the apply of
  a registry file, the accounts of a PC, the Group Policy files of this Windows, Read again, the
  import of the templates of this Windows or of a folder, the Open of a theme file, the section
  Files of the settings and, from W#, the choice of the renderer. An act of them is refused with a
  line; no MCP. The menu This PC is named Scripts there: it saves the scripts and the registry
  files and opens an audit report. The guide is the one of WinWeaver.

## The study of 24.10.2026

**It can be done.** eframe 0.36 runs an `eframe::App` in a browser through `eframe::WebRunner`, and
`ww::Window` is such an App. The core that builds `autounattend.xml` is pure Rust with its data
embedded. A web wwlite can open a `.wwprofile`, an older JSON profile or an answer file with an
embedded profile from the bytes the browser gives, and save the profile and `autounattend.xml`
as downloads, with the built-in catalog.

The blockers (all resolved, section 0):

1. The clock. `SystemTime::now()` and `Instant::now()` panic on `wasm32-unknown-unknown` [12], and `ww`
   calls them at the start (the first log line), for a new document and at every change.
2. Files. `ww` opens and saves by path through the synchronous `rfd::FileDialog`, which the web lacks,
   and keeps the settings, drafts and logs next to the executable.
3. The input languages. Their list comes from Windows at run time; without it every build of an answer
   file fails. The web needs that list embedded, which the rule that the lists come from the system forbids today.
4. The start. `ww::run` uses `NativeOptions` and `run_native`, which the web build of eframe lacks.
5. The profile views of `winweaver-ui` name the acts of This PC, the registry, catalogs and users
   files; wwlite has to separate them in any case, for Windows too.

Effort: once the native wwlite exists, two medium changes of `ww` and small ones of `ww-base` (W#,
now in supakov/ww), the
core, the start and CI. The separation of the views (part of wwlite) is the largest piece.

## 1. eframe 0.36 on the web

- `WebRunner::new().start(canvas, WebOptions, app_creator)` is async and runs in
  `wasm_bindgen_futures::spawn_local`; eframe_template calls it from `main` under
  `#[cfg(target_arch = "wasm32")]` [1][3]. The wasm32 build of eframe has no `run_native` and no
  `NativeOptions` [2].
- Renderer: the feature `wgpu` prefers WebGPU with WebGL as the fallback (since 0.34.1); `glow` draws
  with WebGL and makes a smaller `.wasm` [1][7]. The feature `wgpu` of `ww` builds wgpu with the `dx12`
  backend only (`crates/ww/Cargo.toml:16,22` of W#), which has nothing to draw with in a browser, so the web
  takes `glow`. `WebOptions::webgl_context_option` is `BestFirst` by default [4].
- `eframe::Frame` does not implement `HasWindowHandle` on wasm32 [5].
- Build: trunk (`trunk build --release`, the site in `dist/`, `data-wasm-opt` in `index.html`) [3][6].
  eframe_template is a bin crate without `crate-type = ["cdylib"]`; a cdylib is needed only for
  wasm-pack or wasm-bindgen by hand.
- A dropped file carries its name and bytes from the eframe web backend and a path only from
  egui-winit [8]; 0.36.0 also keeps the `web_sys::File` in `DroppedFile` [7]. Whether `bytes` is still
  filled at once: not checked.
- rfd 0.17: the synchronous `FileDialog` is not exported on wasm; `AsyncFileDialog::pick_file` uses an
  `<input type="file">` and opens at once while the click is a user activation, else it shows an
  overlay with Ok. `save_file` returns at once, and `FileHandle::write` shows an overlay with the link
  "click here to download your file", a second click [9][10][11]. A download through `web-sys` (a
  `Blob`, `Url::create_object_url_with_blob`, an `<a download>` clicked) takes one click.

## 2. What `ww` uses that the web lacks

| Place | What | Change | Size |
|---|---|---|---|
| `ww-base/src/clock.rs:3,7`, `ww/src/session.rs:50-58`, `ww/src/doc.rs:74,102` | `SystemTime::now`, `Instant::now` | `web_time::{SystemTime, Instant}`, which re-exports `std::time` elsewhere [12] | small |
| `ww/src/run.rs:81-146`, `ww/src/renderer.rs:136-141` | `NativeOptions`, `run_native`, the fallback of renderers | native only; a module `ww::web` with `start::<A>(canvas_id, make)` | medium |
| `shell/mod.rs:265,268`, `shell/window.rs:125-129,150-158`, `shell/theme.rs:94-103`, `widgets.rs:81`, `lib.rs:38` | open by `PathBuf`, the sync `rfd::FileDialog` | sync dialogs native only; `App::open_bytes(name, bytes)`; the async pick in `spawn_local` into a slot the frame polls; Save as a download | medium |
| `shell/window.rs:292-294` | dropped files by path | by bytes when there is no path | small |
| `settings.rs:130,188-189`, `session.rs:65-90,136-150`, `shell/mod.rs:449-450`, `themes.rs:138,186` | settings, drafts, logs and themes as files; on wasm32 `std::fs` compiles and every call fails, so each change would show "Settings not saved" | a store: files natively, `localStorage` on the web; logs to the console; embedded themes only | (in the row above) |
| `shell/theme.rs:163-183` | `frame.window_handle()` for the title bar | native only | small |
| `style.rs:303-308,410-415` | fonts from `C:\Windows\Fonts` | nothing breaks: the fonts of egui (`default_fonts`) are the fallback. Segoe UI may not be shipped; an open font could be embedded | small |
| `folders.rs:20-42`, `coloricons.rs:50,163`, `Cargo.toml:30-42` | Explorer, DirectWrite, DWM | already `cfg(windows)`; the web hides Open the folder, the recent files, Exit and the renderer choice | small |

There is no `std::thread` in `ww`, `ww-base` or the core (the threads `this-pc` and `registry-search`
live in `winweaver-ui`). Chromium keeps Ctrl+N, Ctrl+W and Ctrl+Tab for itself (other browsers not
checked), so the web needs other keys for New, Close and the tabs, and a `beforeunload` question while
a tab has changes.

## 3. winweaver-core

- By reading, the core compiles for wasm32 but for one crate: `getrandom` 0.4 (`Cargo.toml:31`, used
  only by `userstore.rs:186-187`) needs its feature `wasm_js` on `wasm32-unknown-unknown`, to be turned
  on in the binary, not in a library [13]. The web binary names it for its target, or users files go
  behind a feature of the core. Whether 0.4 fails to compile or only fails at run time without it: not
  checked. `zip` (deflate through miniz_oxide), `quick-xml`, `sha2` and the RustCrypto crates are pure
  Rust; nothing has C code. `ahash` asks for getrandom 0.3 in `Cargo.lock`, a lock of 0.22.1 without
  `ww`; whether that reaches a web build is not checked (eframe_template names no getrandom [3]).
- `std::fs`, `std::env` (`pol.rs:252`) and `clock` (`users.rs:125`) compile and lie off the path of
  wwlite.
- From bytes: `jsonfile::decode_bom` (`ww-base/src/jsonfile.rs:75`), then `Profile::parse(text, name)`
  (`profile.rs:313`), `import::kind_of` with `from_winkickoff` (`import.rs:25,46`) or
  `import::profile_from_answer_file(text, name)` (`import.rs:129`). Saving is `Profile::to_json`
  (`profile.rs:318`) and `render::answer::build` (`render/answer.rs:120`), whose `Build::xml`
  (`render/mod.rs:88`) is the file. None of them reads a path. An answer file opens only when it holds
  an embedded profile (`import.rs:130-134`); a foreign one does not,
  as on Windows. The size limit is checked on the bytes before the decode (rule 8).
- The input languages: `resources::locales()` (`resources.rs:121-134`) is empty without a reader.
  `validate` then skips its check (`validate.rs:349`), but `render/languages.rs:31-33` fails with
  "Windows does not know the input language" for every item, and a profile needs at least one. A
  snapshot `data/resources/locales.json` written on the Windows CI runner through
  `winweaver_win::locales` (the form of `tests/data/locales.json`) and embedded for the web only
  removes it; its names come in the language of the runner.
- `CatalogTexts::embedded()` (`i18n.rs:105`) links all eight translations (1.9 MB); wwlite parses
  the Ukrainian one alone with `CatalogTexts::parse`.

## 4. Size

On disk: `data/catalog` 2.28 MB (rules and groups 395 KB, `lang/uk.json` 256 KB, all translations
1.89 MB), `data/lang` 722 KB (`uk/ui.json` 75 KB), `data/templates` 73 KB (24 KB for the answer file),
presets 47 KB, time zones 7 KB. A web wwlite in English and Ukrainian embeds about 807 KB of text,
141 KB after gzip -9 (all the data of WinWeaver: 3.24 MB, 737 KB).

Published builds, read from their servers [14]: eframe_template (wgpu, a tiny app,
`data-wasm-opt="2"`) 7.8 MB, 3.1 MB with gzip; the egui web demo 11.7 MB, 4.4 MB with gzip. glow is
smaller than wgpu by an amount not measured. An estimate for wwlite with glow: 5-9 MB of wasm, 2-3.5 MB
as GitHub Pages serves it with gzip. A build gives the real number.

## 5. Plan

Decisions of the customer first: the embedded list of languages for the web; the host (Pages of a
private repository needs GitHub Pro, Team or Enterprise, and the site is public [15]; the repository is
public since 21.11.2026; else the CI
artifact on any static server, since a page opened from `file://` cannot load its wasm module in
Chromium); whether drafts of profiles with passwords may stay in `localStorage`.

1. `web-time` in `ww-base` and `ww`, three files. Small.
2. `ww` for wasm32: `run.rs` and `renderer.rs` native only, `ww::web::start` with `WebRunner` and glow,
   the title bar and the sync dialogs behind `cfg(not(target_arch = "wasm32"))`, and
   `[target.'cfg(target_arch = "wasm32")'.dependencies]` with `wasm-bindgen-futures`, `web-sys`
   (Storage, Blob, Url, HtmlAnchorElement, console) and `web-time`. Medium.
3. The files of `ww` on the web: `App::open_bytes`, the async pick, Save as a download, settings and
   drafts in `localStorage`, logs to the console, the items without a meaning hidden, other keys,
   `beforeunload`. Medium.
4. A program of texts alone in the browser, the first proof of steps 1-3. Small.
5. The core: getrandom for the web binary (or a feature `users`), the snapshot of the languages from the
   Windows runner, the Ukrainian catalog texts alone. Small.
6. wwlite natively (the customer's plan): the profile views without `winweaver-win` and
   `winweaver-mcp`. They name acts of other subsystems (`views/mod.rs:14-17`,
   `views/detail.rs:256-277`, `forms.rs:351,358`), so they move into a crate of their own or behind a
   feature. Large, part of wwlite anyway.
7. The web start of wwlite: `main.rs` with `ww::run` natively and `ww::web::start` under
   `cfg(target_arch = "wasm32")`; `ww` with `default-features = false`, `glow` and `wgpu` natively and
   `glow` alone on wasm32 (per target in `Cargo.toml`, or the trunk attributes
   `data-cargo-no-default-features data-cargo-features="glow"` [16]); `index.html` with
   `<link data-trunk rel="rust" data-wasm-opt="z">`. Built with `rustup target add
   wasm32-unknown-unknown` and `trunk build --release --public-url ./`. Small.
8. CI in the Linux job (the cheap minutes): `cargo clippy -p wwlite --target wasm32-unknown-unknown
   -- -D warnings`, then trunk, `dist/` as the artifact `wwlite-web`; on `main`, if Pages is chosen,
   `actions/upload-pages-artifact` and `actions/deploy-pages` (eframe_template pushes to `gh-pages` with
   `JamesIves/github-pages-deploy-action` [6]). Small.

## 6. Building the page by hand

The same steps as the job `build-web`, on any computer with Rust (never the customer's work PC,
AGENTS.md rule 6). The first build takes 10-30 minutes and about 3 GB in `target/`; a build after a
change takes a minute or two.

On Windows, `build-all.cmd` in the root of the repository does steps 4 to 9 itself, with the archives
of the other programs (AGENTS.md, section 5): the page is `dist/wwlite-web/wwlite.html`, the folder
for a server `dist/wwlite-web/files`, and the archive `dist/wwlite-web-<version>.zip`. The steps below
are the same by hand.

1. Install Rust with rustup (https://rustup.rs). On Windows the build scripts of the crates need a
   linker for the computer itself: either the Visual Studio Build Tools with the workload "Desktop
   development with C++" for the default toolchain, or the toolchain that carries its own linker:
   `rustup default stable-x86_64-pc-windows-gnu`.
2. Add the target of the browser: `rustup target add wasm32-unknown-unknown`.
3. Get the sources: `git clone https://github.com/digidult/winweaver` and go into the folder.
4. Read the version of wasm-bindgen the build uses: the line after `name = "wasm-bindgen"` in
   `Cargo.lock` (0.2.129 on 28.10.2026). The tool must have the same version as the crate.
5. Install the tool of that version: `cargo install wasm-bindgen-cli --version 0.2.129 --locked`
   (a few minutes), or unpack the binary of its release from
   https://github.com/wasm-bindgen/wasm-bindgen/releases.
6. Build the module:
   `cargo build --release -p wwlite --no-default-features --features glow --target wasm32-unknown-unknown`.
7. Write the page next to it:
   `wasm-bindgen --target web --no-typescript --out-dir dist-web/wwlite-web target/wasm32-unknown-unknown/release/wwlite.wasm`.
8. Copy `crates/wwlite/web/index.html` into `dist-web/wwlite-web`.
9. Make the one file: `python crates/wwlite/web/single.py dist-web/wwlite-web dist-web/wwlite.html`.
10. Open `dist-web/wwlite.html` with a double click: it is the whole edition, to copy anywhere or
    put on any static host. The folder with `index.html` works only over http:
    `python -m http.server 8080 --directory dist-web/wwlite-web`, then `http://localhost:8080/`.

## Sources

1. https://docs.rs/eframe/0.36.2/eframe/ (features, the web renderer, the WebRunner example)
2. https://docs.rs/eframe/0.36.2/wasm32-unknown-unknown/eframe/index.html
3. https://github.com/emilk/eframe_template (`Cargo.toml`, `src/main.rs`, `index.html`, README)
4. https://docs.rs/eframe/0.36.2/wasm32-unknown-unknown/eframe/struct.WebOptions.html
5. https://docs.rs/eframe/0.36.2/wasm32-unknown-unknown/eframe/struct.Frame.html
6. https://github.com/emilk/eframe_template/blob/main/.github/workflows/pages.yml
7. https://github.com/emilk/egui/blob/main/crates/eframe/CHANGELOG.md (0.34.1, 0.36.0)
8. https://doc.servo.org/egui/struct.DroppedFile.html (its egui version not checked)
9. https://docs.rs/rfd/0.17.2/rfd/ (platforms: "WASM32 (async only)")
10. https://github.com/PolyMeilex/rfd/blob/master/src/lib.rs and `src/file_dialog.rs` (master, not the tag
    0.17.2)
11. https://github.com/PolyMeilex/rfd/blob/master/src/backend/wasm.rs
12. https://docs.rs/web-time/latest/web_time/
13. https://docs.rs/getrandom/0.4.3/getrandom/
14. `Content-Length` of https://emilk.github.io/eframe_template/eframe_template_bg.wasm and
    https://www.egui.rs/egui_demo_app_bg.wasm, plain and with `Accept-Encoding: gzip`
15. https://docs.github.com/en/pages/getting-started-with-github-pages/creating-a-github-pages-site
16. https://github.com/trunk-rs/trunk/blob/main/guide/src/assets/index.md
