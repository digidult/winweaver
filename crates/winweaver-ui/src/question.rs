//! The questions of WinWeaver over the window before an act that is hard to take back. The
//! shell shows one (`ShellAct::Ask`); a yes runs its act, a no (or Escape) drops it. The
//! questions of the window itself (a tab closed with its changes, the recent files forgotten)
//! come from the shell.

#[cfg(feature = "mcp")]
use crate::McpAct;
use crate::i18n::{tr, trf};
use crate::{Act, CatalogAct, ImportAct, Question, RegistryAct};

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

/// Before a catalog file whose rules run code of their own is imported.
pub fn import_code(name: &str, rules: usize, act: Act) -> Question {
    Question::new(
        tr("Import a catalog with code"),
        trf(
            "{name} runs code of its own: {n} rules with scripts, commands, programs or settings of the answer file. A profile that turns them on runs them as SYSTEM during the installation or on this PC. Import it only from a source you trust.",
            &[("name", &name), ("n", &rules)],
        ),
        tr("Import"),
        act,
    )
}

/// Before the token of the MCP server is renewed.
#[cfg(feature = "mcp")]
pub fn renew_token() -> Question {
    Question::new(
        tr("Renew the token"),
        tr(
            "Renew the token of the MCP server? The clients set up with the old one lose their access until they get the new one.",
        ),
        tr("Renew"),
        McpAct::RenewConfirmed,
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

/// Before a group of a catalog file goes with every group and policy under it.
pub fn remove_group(title: &str, id: String) -> Question {
    Question::new(
        tr("Remove the group"),
        trf(
            "Remove the group {name} with all it holds? It leaves the file at the next save.",
            &[("name", &title)],
        ),
        tr("Remove"),
        CatalogAct::EditRemoveGroup(id),
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
