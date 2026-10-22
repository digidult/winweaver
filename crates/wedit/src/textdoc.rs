//! A text file in a tab: the text editor of egui with its undo, the matches of the search, the
//! position of the cursor, the draft of unsaved changes, the save with the line ends the file
//! came with.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use ww::eframe::egui::{self, text::CCursor, text::CCursorRange, text_edit::TextEditState};
use ww::i18n::tr;
use ww::{Draft, Search, dialogs, session, style};

/// A draft is written this long after the last change.
const DRAFT_DELAY: Duration = Duration::from_secs(2);

pub struct TextDoc {
    pub text: String,
    pub path: Option<PathBuf>,
    pub dirty: bool,
    /// The file ends its lines with CR LF, kept at the save.
    pub crlf: bool,
    pub search: Search,
    /// The name of the draft file of the unsaved changes.
    pub draft_key: String,
    /// When the text last changed and has no draft yet.
    pub changed: Option<Instant>,
    /// The line and the column of the cursor, from 1.
    pub line: usize,
    pub col: usize,
    /// The characters to select at the next frame: a match of the search.
    select: Option<(usize, usize)>,
    /// The start of the last match, so that the next one follows it.
    last_match: Option<usize>,
}

impl TextDoc {
    pub fn new(text: String, path: Option<PathBuf>) -> Self {
        let crlf = text.contains("\r\n");
        let text = if crlf {
            text.replace("\r\n", "\n")
        } else {
            text
        };
        let draft_key = match &path {
            Some(p) => session::key_for_path(p),
            None => session::new_key(),
        };
        Self {
            text,
            path,
            dirty: false,
            crlf,
            search: Search::default(),
            draft_key,
            changed: None,
            line: 1,
            col: 1,
            select: None,
            last_match: None,
        }
    }

    /// The title of the tab: the name of the file, else Untitled.
    pub fn title(&self) -> String {
        match &self.path {
            Some(p) => p
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_else(|| p.display().to_string()),
            None => format!("{}.txt", tr("Untitled")),
        }
    }

    /// The id of the editor widget, one a tab.
    fn id(&self) -> egui::Id {
        egui::Id::new(("wedit-text", &self.draft_key))
    }

    /// The draft of a modified text: two seconds after the last change, or at once when the
    /// window closes.
    pub fn draft_due(&mut self, all: bool) -> Option<Draft> {
        if !self.dirty {
            return None;
        }
        let due = match self.changed {
            Some(t) => all || t.elapsed() >= DRAFT_DELAY,
            None => false,
        };
        if !due {
            return None;
        }
        self.changed = None;
        Some(Draft {
            key: self.draft_key.clone(),
            path: self.path.clone(),
            document: serde_json::Value::String(self.text.clone()),
        })
    }

    /// Writes the text to a file, with the line ends it came with.
    pub fn save_to(&mut self, path: &Path) -> Result<(), String> {
        let text = if self.crlf {
            self.text.replace('\n', "\r\n")
        } else {
            self.text.clone()
        };
        dialogs::write_file(path, text.as_bytes())?;
        self.path = Some(path.to_path_buf());
        self.dirty = false;
        self.changed = None;
        Ok(())
    }

    /// Selects the next (or the previous) match of a text after the last one, around the end,
    /// ignoring case; the start of the match, `None` when the text has none.
    pub fn find(&mut self, needle: &str, forward: bool) -> Option<usize> {
        let hay: Vec<char> = self.text.chars().collect();
        let pat: Vec<char> = needle.chars().collect();
        if pat.is_empty() || pat.len() > hay.len() {
            self.last_match = None;
            return None;
        }
        let same = |a: char, b: char| a.to_lowercase().eq(b.to_lowercase());
        let starts: Vec<usize> = (0..=hay.len() - pat.len())
            .filter(|&i| pat.iter().enumerate().all(|(k, c)| same(hay[i + k], *c)))
            .collect();
        let (first, last) = match (starts.first(), starts.last()) {
            (Some(f), Some(l)) => (*f, *l),
            _ => {
                self.last_match = None;
                return None;
            }
        };
        let next = match (self.last_match, forward) {
            (None, true) => first,
            (None, false) => last,
            (Some(at), true) => starts.iter().copied().find(|&s| s > at).unwrap_or(first),
            (Some(at), false) => starts
                .iter()
                .rev()
                .copied()
                .find(|&s| s < at)
                .unwrap_or(last),
        };
        self.last_match = Some(next);
        self.select = Some((next, next + pat.len()));
        Some(next)
    }

    /// The editor: a monospace text field over the whole tab, scrolled both ways.
    pub fn view(&mut self, ui: &mut egui::Ui) {
        let p = style::palette(ui);
        let id = self.id();
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(p.card))
            .show(ui, |ui| {
                if let Some((start, end)) = self.select.take() {
                    let mut state = TextEditState::load(ui.ctx(), id).unwrap_or_default();
                    state.cursor.set_char_range(Some(CCursorRange::two(
                        CCursor::new(start),
                        CCursor::new(end),
                    )));
                    state.store(ui.ctx(), id);
                    ui.memory_mut(|m| m.request_focus(id));
                }
                egui::ScrollArea::both()
                    .id_salt(("wedit-scroll", &self.draft_key))
                    .auto_shrink(false)
                    .show(ui, |ui| {
                        let output = egui::TextEdit::multiline(&mut self.text)
                            .id(id)
                            .font(egui::TextStyle::Monospace)
                            .desired_width(f32::INFINITY)
                            .desired_rows(40)
                            .lock_focus(true)
                            .show(ui);
                        if output.response.changed() {
                            self.dirty = true;
                            self.changed = Some(Instant::now());
                            self.last_match = None;
                        }
                        if let Some(range) = output.cursor_range {
                            (self.line, self.col) = position(&self.text, range.primary.index);
                        }
                    });
            });
    }
}

/// The line and the column of a character index, from 1.
fn position(text: &str, index: usize) -> (usize, usize) {
    let mut line = 1;
    let mut col = 1;
    for c in text.chars().take(index) {
        if c == '\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_matches_go_around_and_ignore_case() {
        let mut doc = TextDoc::new("Abc abc\r\nABC".to_owned(), None);
        assert!(doc.crlf);
        assert_eq!(doc.text, "Abc abc\nABC");
        assert_eq!(doc.find("abc", true), Some(0));
        assert_eq!(doc.find("abc", true), Some(4));
        assert_eq!(doc.find("abc", true), Some(8));
        assert_eq!(doc.find("abc", true), Some(0));
        assert_eq!(doc.find("abc", false), Some(8));
        assert_eq!(doc.find("zzz", true), None);
        assert_eq!(position(&doc.text, 9), (2, 2));
    }
}
