//! A document in a tab: what the window asks of it, and the search of a tab.

use std::path::Path;

/// The searches a tab remembers.
pub const SEARCH_HISTORY: usize = 20;

/// The search of a tab, at the right end of its navigation row or its command bar.
#[derive(Default)]
pub struct Search {
    pub open: bool,
    pub text: String,
    /// The field takes the focus at the next frame.
    pub focus: bool,
}

impl Search {
    /// The lower case words of the search text.
    pub fn words(&self) -> Vec<String> {
        self.text
            .split_whitespace()
            .map(str::to_lowercase)
            .collect()
    }
}

/// Whether a text holds every word of a search, ignoring case.
pub fn matches_words(hay: &str, words: &[String]) -> bool {
    let hay = hay.to_lowercase();
    words.iter().all(|w| hay.contains(w.as_str()))
}

/// Puts a search first in the history of the searches; `true` when the history changed.
pub fn remember_search(history: &mut Vec<String>, text: &str) -> bool {
    let text = text.trim();
    if text.is_empty() {
        return false;
    }
    if history.first().is_some_and(|h| h == text) {
        return false;
    }
    history.retain(|h| h != text);
    history.insert(0, text.to_owned());
    history.truncate(SEARCH_HISTORY);
    true
}

/// What every tab answers, so the window draws it without knowing the document.
pub trait Document {
    /// The title of the tab: the name of its file, else `name`.
    fn title(&self) -> String {
        match self.path() {
            Some(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            None => self.name(),
        }
    }

    /// The title of a document that has no file.
    fn name(&self) -> String {
        String::new()
    }

    /// Whether the document has changes its file does not.
    fn dirty(&self) -> bool {
        false
    }

    /// The file of the document: the hint of the tab, and opened again at the next start.
    fn path(&self) -> Option<&Path> {
        None
    }

    /// The search of the tab, for the tabs that have one.
    fn search(&mut self) -> Option<&mut Search> {
        None
    }

    /// The icon of the tab.
    fn icon(&self) -> &'static str {
        ""
    }

    /// The hint of the search field of the tab, empty for a tab without a search.
    fn search_hint(&self) -> &'static str {
        ""
    }

    /// Whether the tab has commands in the command bar.
    fn has_tools(&self) -> bool {
        true
    }

    /// Whether the tab has a navigation row: a history, a breadcrumb and the star.
    fn has_nav(&self) -> bool {
        false
    }
}
