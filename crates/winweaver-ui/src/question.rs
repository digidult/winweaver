//! A question over the window before an act that is hard to take back. `App::ask` shows it; a yes
//! runs its act, a no (or Escape) drops it.

use crate::i18n::{tr, trf};
use crate::{Act, CatalogAct, FileAct, ImportAct, McpAct, NavAct, RegistryAct};

/// A question over the window: its title, its text, the label of the yes and the act a yes runs.
pub struct Question {
    pub title: String,
    pub text: String,
    pub yes: String,
    pub act: Act,
}

impl Question {
    pub fn new(title: &str, text: impl Into<String>, yes: &str, act: impl Into<Act>) -> Self {
        Self {
            title: title.to_owned(),
            text: text.into(),
            yes: yes.to_owned(),
            act: act.into(),
        }
    }

    /// Before an import is removed with its catalog file.
    pub fn remove_import(name: &str, id: String) -> Self {
        Self::new(
            tr("Remove the import"),
            trf(
                "Remove {name}? Its catalog file is deleted from the imports folder, and the options of profiles from it become unknown.",
                &[("name", &name)],
            ),
            tr("Remove"),
            ImportAct::Remove(id),
        )
    }

    /// Before a tab with unsaved changes is closed.
    pub fn close_discard(title: &str, tab: usize) -> Self {
        Self::new(
            tr("Discard the changes"),
            trf(
                "Close {title} and discard its unsaved changes?",
                &[("title", &title)],
            ),
            tr("Discard"),
            NavAct::CloseTab(tab, true),
        )
    }

    /// Before the list of the recent files is cleared.
    pub fn forget_recent() -> Self {
        Self::new(
            tr("Forget the recent files"),
            tr("Forget the list of the recent files? The files stay where they are."),
            tr("Forget"),
            FileAct::ForgetRecent,
        )
    }

    /// Before the token of the MCP server is renewed.
    pub fn renew_token() -> Self {
        Self::new(
            tr("Renew the token"),
            tr("Renew the token of the MCP server? The clients set up with the old one lose their access until they get the new one."),
            tr("Renew"),
            McpAct::RenewToken,
        )
    }

    /// Before the rights of a registry key are written.
    pub fn change_rights(path: &str) -> Self {
        Self::new(
            tr("Change the rights"),
            trf(
                "Change the rights of {path}? A wrong access list can shut Windows or a program out of the key.",
                &[("path", &path)],
            ),
            tr("Change"),
            RegistryAct::RightsSave,
        )
    }

    /// Before a policy is removed from the catalog being edited.
    pub fn remove_policy(id: String) -> Self {
        Self::new(
            tr("Remove the policy"),
            trf(
                "Remove the policy {id} from the catalog? It leaves the file at the next save.",
                &[("id", &id)],
            ),
            tr("Remove"),
            CatalogAct::EditRemove(id),
        )
    }
}

impl From<Question> for Act {
    fn from(question: Question) -> Self {
        Act::Ask(Box::new(question))
    }
}
