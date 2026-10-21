//! The chrome of the window around the documents (`docs/spec/01-ui.md` sections 1 to 5 and 13):
//! the tab row with the menu, the command bar of the active tab (its own commands, icons above
//! the labels), the navigation row where it means something (back and forward, the breadcrumb,
//! the star, the search of the tab), the message bar with its own height, and the status bar.

use eframe::egui::{self, Align, Align2, CornerRadius, Layout, Margin, RichText, Sense, Stroke};

use crate::commands::{Item, Menu, keys};
use crate::i18n::{tr, trf};
use crate::style::{self, Palette};
use crate::{
    Act, App, CatalogAct, DiffAct, Doc, FileAct, Form, ImportAct, MarksAct, McpAct, NavAct,
    PRESETS, PcAct, ProfileAct, RegistryAct, ScriptKind, Sel, Settings, SettingsAct,
    SettingsSection, Tab, TreeKey, UsersAct, icons, texts_for, views,
};
use winweaver_core::validate::{self, Severity, Target};

/// The height of a tab.
const TAB_HEIGHT: f32 = 32.0;
/// The title of a tab is cut at this width.
const TAB_TITLE_WIDTH: f32 = 200.0;
/// The parts of a tab from left to right: the padding, the icon, the dot of unsaved changes,
/// the gap after the title and the close button.
const TAB_PADDING: f32 = 12.0;
const TAB_ICON: f32 = 18.0;
const TAB_DOT: f32 = 10.0;
const TAB_GAP: f32 = 8.0;
const TAB_CLOSE: f32 = 20.0;
/// The width the navigation row keeps for the command bar until it is measured.
const TOOLS_WIDTH: f32 = 320.0;
/// The width of the search field, open and closed.
const SEARCH_OPEN_WIDTH: f32 = 240.0;
const SEARCH_CLOSED_WIDTH: f32 = 34.0;
/// The width of the star, the margin of the breadcrumb and its least room.
const STAR_WIDTH: f32 = 34.0;
const CRUMBS_MARGIN: f32 = 32.0;
const CRUMBS_MIN: f32 = 80.0;

/// A frame of a bar: the card colour with a line under it.
fn bar_frame(p: &Palette, margin: Margin) -> egui::Frame {
    egui::Frame::new().fill(p.card).inner_margin(margin)
}

mod bar;
mod nav;
mod status;
mod tabs;

impl App {
    /// The frame colours of the panels of the chrome.
    fn palette_now(ui: &egui::Ui) -> Palette {
        style::palette(ui)
    }

    /// Whether the active tab has commands in the command bar.
    pub(crate) fn has_tools(&self) -> bool {
        self.docs
            .get(self.active)
            .is_none_or(|d| d.kind().has_tools())
    }

    /// Whether the active tab has a navigation row: a history, a breadcrumb or a search.
    pub(crate) fn has_nav(&self) -> bool {
        self.docs
            .get(self.active)
            .is_some_and(|d| d.kind().has_nav())
    }

    /// Whether the active tab has something to search.
    pub(crate) fn has_search(&self) -> bool {
        self.docs
            .get(self.active)
            .is_some_and(|d| d.kind().has_search())
    }

    /// The hint of the search field of the active tab.
    pub(crate) fn search_hint_now(&self) -> &'static str {
        self.docs
            .get(self.active)
            .map_or("", |d| d.kind().search_hint())
    }

    /// The frames of the bars of the chrome.
    pub(crate) fn chrome_frames(
        ui: &egui::Ui,
    ) -> (egui::Frame, egui::Frame, egui::Frame, egui::Frame) {
        let p = Self::palette_now(ui);
        (
            egui::Frame::new().fill(p.bg).inner_margin(Margin {
                left: 6,
                right: 6,
                top: 4,
                bottom: 0,
            }),
            bar_frame(&p, Margin::symmetric(8, 4)),
            bar_frame(&p, Margin::symmetric(8, 4)),
            bar_frame(&p, Margin::symmetric(12, 3)),
        )
    }
}
