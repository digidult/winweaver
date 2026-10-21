//! WW (W#): a window with tabs for programs that analyse and manage a system. The look of the
//! window (`style`), the themes and fonts (`themes`), the translations (`i18n`), the painted tree
//! and list (`tree`, `list`), the modal window and the side panel (`widgets`), the Markdown viewer
//! (`mdview`), the renderers (`renderer`), the settings file (`settings`), the drafts and the log
//! (`session`), the file dialogs (`dialogs`), the folders next to the executable (`folders`), the
//! search of a tab and the trait of a document (`doc`), a question over the window (`question`),
//! the keys of the commands (`keys`), the icons (`icons`). The shell that puts them together comes
//! with the next step of `docs/framework.md`.

pub mod dialogs;
pub mod doc;
pub mod folders;
pub mod i18n;
pub mod icons;
pub mod keys;
pub mod list;
pub mod mdview;
pub mod question;
pub mod renderer;
pub mod session;
pub mod settings;
pub mod style;
pub mod themes;
pub mod tree;
pub mod widgets;

pub use doc::{Document, Search, matches_words, remember_search};
pub use question::Question;
pub use settings::{Extra, Settings, ThemeChoice};
