//! The build of `autounattend.xml` from a profile (`docs/spec/03-features.md` section 3).
//!
//! The runtime templates of `data/templates/` hold the fixed parts; this module fills their
//! `{{slot}}`s with the passes, the blocks of the rules in effect and the profile data, and embeds
//! the scripts and the profile into the `Extensions`. Everything the generator itself writes is
//! ASCII.
//!
//! The folder: `text` (the escapes), `params` (the placeholders and the parameter values), `ps`
//! (the script lines of the actions), `xml` (the pieces of the passes), `languages`, `name` (the
//! computer name template), `accounts` and `answer` (the build itself).

use crate::catalog::Rule;

mod accounts;
mod answer;
mod languages;
mod name;
mod params;
mod ps;
#[cfg(test)]
mod tests;
mod text;
mod xml;

pub use answer::build;
pub use name::{EXAMPLE_MAC, EXAMPLE_SERIAL, computer_name_parts, example_name};
pub use params::substitute;
pub use text::{ascii_json, ascii_text, base64, ps_quote, xml_escape};

pub(crate) use params::{fill, fill_value, list_values, params_of};
pub(crate) use ps::{join_blocks, reg_list_line, reg_path, reg_value, script_lines};
pub(crate) use text::ps_list;

pub(super) const TEMPLATE_XML: &str =
    include_str!("../../../../data/templates/autounattend.template.xml");
pub(super) const SETUP_SYSTEM: &str =
    include_str!("../../../../data/templates/Setup-System.runtime.ps1");
pub(super) const SETUP_USER: &str =
    include_str!("../../../../data/templates/Setup-User.runtime.ps1");
pub(super) const POST_OOBE: &str = include_str!("../../../../data/templates/Post-OOBE.runtime.ps1");
pub(super) const SECTION_DEFAULT_USER: &str =
    include_str!("../../../../data/templates/section-default-user.ps1");
pub(super) const SECTION_ACTIVE_SETUP: &str =
    include_str!("../../../../data/templates/section-active-setup.ps1");
pub(super) const SECTION_POST_OOBE_TASK: &str =
    include_str!("../../../../data/templates/section-post-oobe-task.ps1");
pub(super) const SECTION_COMPUTER_NAME: &str =
    include_str!("../../../../data/templates/section-computer-name.ps1");
const RUNTIME_VERSION_FILE: &str = include_str!("../../../../data/templates/VERSION");

/// The key of the mode `ask`: Windows Setup shows the key and edition dialog.
pub const ASK_KEY: &str = "00000-00000-00000-00000-00000";
/// The temporary computer name of a template; `section-computer-name.ps1` replaces it.
pub const TEMP_COMPUTER_NAME: &str = "WINWEAVER-TMP";
/// The longest `RunSynchronousCommand/Path` Windows Setup accepts.
pub const MAX_COMMAND: usize = 259;
pub(super) const ARCHITECTURES: [&str; 2] = ["amd64", "arm64"];
pub(super) const SCRIPTS_DIR: &str = r"C:\ProgramData\Unattend\Scripts";
pub(super) const SETUP_SYSTEM_NAME: &str = "Setup-System.ps1";
pub(super) const SETUP_USER_NAME: &str = "Setup-User.ps1";
pub(super) const POST_OOBE_NAME: &str = "Post-OOBE.ps1";

/// The version of the runtime templates.
pub fn runtime_version() -> &'static str {
    RUNTIME_VERSION_FILE.trim()
}

#[derive(Debug, thiserror::Error)]
pub enum RenderError {
    #[error("rule {rule}: {text}")]
    Rule { rule: String, text: String },
    #[error("{0}")]
    Profile(String),
    #[error("the command of {what} is {len} characters, more than {MAX_COMMAND}: {command}")]
    CommandTooLong {
        what: String,
        len: usize,
        command: String,
    },
    #[error("{0}")]
    Template(String),
}

/// The result of a build.
#[derive(Debug, Clone)]
pub struct Build {
    /// The answer file, CRLF, ASCII.
    pub xml: String,
    /// The embedded scripts by file name, in the order of the `Extensions`.
    pub scripts: Vec<(String, String)>,
    /// The rules in effect, in the order they were applied.
    pub rule_ids: Vec<String>,
    pub warnings: Vec<String>,
}

pub(super) fn err(rule: &Rule, text: impl Into<String>) -> RenderError {
    RenderError::Rule {
        rule: rule.id.clone(),
        text: text.into(),
    }
}
