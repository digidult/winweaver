//! The questions of WinWeaver over the window before an act that is hard to take back. The
//! shell shows one (`ShellAct::Ask`); a yes runs its act, a no (or Escape) drops it. The
//! questions of the window itself (a tab closed with its changes, the recent files forgotten)
//! come from the shell.

use crate::i18n::{tr, trf};
use crate::{CatalogAct, ImportAct, McpAct, Question, RegistryAct};

/// Before an import is removed with its catalog file.
pub fn remove_import(name: &str, id: String) -> Question {
    Question::new(
        tr("Remove the import"),
        trf(
            "Remove {name}? Its catalog file is deleted from the imports folder, and the options of profiles from it become unknown.",
            &[("name", &name)],
        ),
        tr("Remove"),
        ImportAct::Remove(id),
    )
}

/// Before the token of the MCP server is renewed.
pub fn renew_token() -> Question {
    Question::new(
        tr("Renew the token"),
        tr(
            "Renew the token of the MCP server? The clients set up with the old one lose their access until they get the new one.",
        ),
        tr("Renew"),
        McpAct::RenewToken,
    )
}

/// Before the rights of a registry key are written.
pub fn change_rights(path: &str) -> Question {
    Question::new(
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
pub fn remove_policy(id: String) -> Question {
    Question::new(
        tr("Remove the policy"),
        trf(
            "Remove the policy {id} from the catalog? It leaves the file at the next save.",
            &[("id", &id)],
        ),
        tr("Remove"),
        CatalogAct::EditRemove(id),
    )
}
