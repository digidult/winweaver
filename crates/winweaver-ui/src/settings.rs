//! The settings of WinWeaver beyond the ones of the window (`docs/spec/02-documents.md` section
//! 7): the same `settings.json` next to the executable, the fields of the window from WW and the
//! fields below flattened beside them.

use serde::{Deserialize, Serialize};

pub use ww::settings::ThemeChoice;

/// The settings of the window with the fields of WinWeaver.
pub type Settings = ww::Settings<Extra>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Extra {
    /// Whether the description panel is shown.
    pub desc_open: bool,
    /// The guide opens at the start; the window clears it after the first start.
    pub show_guide_at_start: bool,
    /// The rules of the selected group show a switch at the end of their rows.
    pub inline_switches: bool,
    /// The port and the token of the MCP server of an earlier version: read once to make
    /// `mcp/default.wwmcp`, never written again.
    #[serde(skip_serializing)]
    pub mcp_port: u16,
    #[serde(skip_serializing)]
    pub mcp_token: String,
    /// The imports of policy templates that are read and loaded, by id; `None` in the settings of
    /// an earlier version means every file but the hidden ones.
    pub loaded_imports: Option<Vec<String>>,
    /// The imports an earlier version hid: read once into `loaded_imports`, never written again.
    #[serde(skip_serializing)]
    pub hidden_imports: Vec<String>,
    /// The options of catalogs that are not loaded are left out of the tree.
    pub hide_unknown: bool,
    pub desc_sections: DescSections,
    /// The rules that write a key of a registry file are shown open at first.
    pub writers_open: bool,
    /// The outline of a registry file is a tree of its keys, as regedit shows them; off, a list in
    /// the order of the file.
    pub regfile_tree: bool,
    /// The Group Policy files of this Windows open in their tabs when this PC is read.
    pub pol_with_pc: bool,
    /// A click on a catalog in the menu Catalogs opens its tab, and the menu lists the loaded ones
    /// alone; off (the default), a click loads or unloads it and every catalog file is listed.
    pub catalog_menu_opens: bool,
}

impl Default for Extra {
    fn default() -> Self {
        Self {
            desc_open: true,
            show_guide_at_start: true,
            inline_switches: true,
            mcp_port: 8765,
            mcp_token: String::new(),
            loaded_imports: None,
            hidden_imports: Vec::new(),
            hide_unknown: false,
            desc_sections: DescSections::default(),
            writers_open: true,
            regfile_tree: true,
            pol_with_pc: false,
            catalog_menu_opens: false,
        }
    }
}

impl ww::settings::Extra for Extra {
    const FORMAT: &'static str = "winweaver-settings";
}

/// The parts of the description of an option open at first; folded ones open with a click.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DescSections {
    pub what: bool,
    pub risk: bool,
    pub writes: bool,
    pub deps: bool,
    pub more: bool,
    pub reference: bool,
    /// The coloured stripe of each part inside its card; the tint of the card stays always.
    pub stripes: bool,
}

impl Default for DescSections {
    fn default() -> Self {
        Self {
            what: true,
            risk: true,
            writes: true,
            deps: false,
            more: false,
            reference: false,
            stripes: true,
        }
    }
}
