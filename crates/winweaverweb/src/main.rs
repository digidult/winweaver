//! winweaverweb, WinWeaver in a browser: the window of WinWeaver with every file it opens and
//! saves (profiles, answer files, diffs, catalogs and the imports of policy templates from an
//! archive, bookmarks, registry and Group Policy files, users files, themes), the scripts and the
//! registry files it writes, and none of what needs Windows or a server: no This PC, no registry
//! of a PC, no native apply, no MCP. Built for `wasm32-unknown-unknown` (the feature `glow`),
//! `web/index.html` loads the module next to it and `main` starts the window in the canvas
//! `winweaverweb` as a task of the page (`winweaver_ui::run_web`). Files come and go through the
//! browser (a dialog or a drop, downloads); the settings, the drafts and the bookmarks stay in the
//! storage of the page; the catalogs, the themes and the translations of the folder of the page
//! come over http (`docs/wasm.md`).

/// The id of the canvas of `web/index.html` the window draws in.
#[cfg(target_arch = "wasm32")]
const CANVAS_ID: &str = "winweaverweb";

/// The browser edition: the window in the canvas of the page, started when the module loads.
#[cfg(target_arch = "wasm32")]
fn main() {
    wasm_bindgen_futures::spawn_local(winweaver_ui::run_web(CANVAS_ID));
}

/// A native build has nothing to do: WinWeaver itself is the program for Windows.
#[cfg(not(target_arch = "wasm32"))]
fn main() -> std::process::ExitCode {
    eprintln!(
        "winweaverweb {} is the browser edition of WinWeaver: build it for wasm32-unknown-unknown (docs/wasm.md); winweaver.exe is the program for Windows",
        winweaver_core::BUILD
    );
    std::process::ExitCode::from(2)
}
