//! The profile views: the tree on the left (`tree.rs` paints it) and the detail panel on the right,
//! in cards.

use eframe::egui::{self, CornerRadius, Margin, RichText, Stroke};
use serde_json::Value;
use winweaver_core::admx::CatalogFile;
use winweaver_core::catalog::{
    Action, Catalog, Group, ListNames, Param, Pick, ROOT_UNATTEND, Rule, check,
};
use winweaver_core::i18n::CatalogTexts;
use winweaver_core::resolver::State;

use crate::i18n::{tr, trf};
use crate::{
    Act, Form, ProfileAct, ProfileDoc, Search, Sel, Settings, UNKNOWN_GROUP, icons, matches_words,
    style, tree,
};
#[cfg(feature = "full")]
use crate::{CatalogAct, ImportAct, PcAct, RegistryAct, ScriptKind, TreeKey};

mod detail;
mod profile_tree;
mod rule;
#[cfg(test)]
mod tests;

pub(crate) use detail::*;
pub(crate) use profile_tree::*;
pub(crate) use rule::*;
pub(crate) use ww::style::{block, header};

pub fn rtitle<'a>(texts: Option<&'a CatalogTexts>, rule: &'a Rule) -> &'a str {
    texts.map_or(rule.title.as_str(), |t| t.rule_title(rule))
}

pub fn rsummary<'a>(texts: Option<&'a CatalogTexts>, rule: &'a Rule) -> &'a str {
    texts.map_or(rule.summary.as_str(), |t| t.rule_summary(rule))
}

pub fn gtitle<'a>(texts: Option<&'a CatalogTexts>, group: &'a Group) -> &'a str {
    texts.map_or(group.title.as_str(), |t| t.group_title(group))
}

pub fn gsummary<'a>(texts: Option<&'a CatalogTexts>, group: &'a Group) -> &'a str {
    texts.map_or(group.summary.as_str(), |t| t.group_summary(group))
}

/// Where a link to a rule leads from the view it is drawn in.
#[derive(Clone, Copy)]
pub(crate) enum RuleLink<'a> {
    /// The profile tab in front: the rule is selected there and its tree opens to it.
    Profile,
    /// The catalog tab in front: a policy of its file is selected in its tree, another rule in the
    /// profile.
    #[cfg(feature = "full")]
    Catalog(&'a CatalogFile),
    /// Any other tab: the rule is shown in the first profile tab.
    #[cfg(feature = "full")]
    Elsewhere,
    /// The lifetime of the catalog, kept for the light edition.
    #[cfg(not(feature = "full"))]
    #[allow(dead_code)]
    Never(std::marker::PhantomData<&'a ()>),
}

impl RuleLink<'_> {
    /// What a click on a link to the rule `id` does.
    pub(crate) fn act(self, id: &str) -> Act {
        let id = id.to_owned();
        match self {
            RuleLink::Profile => Act::Profile(ProfileAct::Reveal(Sel::Rule(id))),
            #[cfg(feature = "full")]
            RuleLink::Catalog(file) if file.rules.iter().any(|r| r.id == id) => {
                Act::Catalog(CatalogAct::Reveal(TreeKey::Rule(id)))
            }
            #[cfg(feature = "full")]
            RuleLink::Catalog(_) | RuleLink::Elsewhere => {
                Act::Profile(ProfileAct::ShowInProfile(String::new(), id))
            }
            #[cfg(not(feature = "full"))]
            RuleLink::Never(_) => Act::Profile(ProfileAct::Reveal(Sel::Rule(id))),
        }
    }
}

/// A piece of a text: plain words, or a rule of the catalog or a key of the registry it names.
enum Piece<'a> {
    Text(String),
    Rule(&'a Rule),
    Key(String),
}

/// The pieces of a text, the ids of rules and the paths of the registry apart from the words.
fn pieces<'a>(catalog: &'a Catalog, text: &str) -> Vec<Piece<'a>> {
    const AROUND: [char; 10] = [',', '.', ';', ':', ')', '(', '!', '?', '"', '\''];
    let mut out = Vec::new();
    let mut plain = String::new();
    for word in text.split_inclusive(char::is_whitespace) {
        let start = word.len() - word.trim_start_matches(['(', '"', '\'']).len();
        let end = word.trim_end().trim_end_matches(AROUND).len().max(start);
        let core = &word[start..end];
        let key = (core.starts_with("HK") || core.starts_with("DU:")) && core.contains('\\');
        let found = match catalog.rule(core) {
            Some(rule) => Some(Piece::Rule(rule)),
            None if key => winweaver_core::registry::normalize(core).map(Piece::Key),
            None => None,
        };
        match found {
            Some(piece) => {
                plain.push_str(&word[..start]);
                if !plain.is_empty() {
                    out.push(Piece::Text(std::mem::take(&mut plain)));
                }
                out.push(piece);
                plain.push_str(&word[end..]);
            }
            None => plain.push_str(word),
        }
    }
    if !plain.is_empty() {
        out.push(Piece::Text(plain));
    }
    out
}

/// A text with links to the rules of the catalog and the keys of the registry it names.
pub(crate) fn linked_text(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    text: &str,
    color: egui::Color32,
    link: RuleLink,
    acts: &mut Vec<Act>,
) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for piece in pieces(catalog, text) {
            match piece {
                Piece::Text(t) => {
                    ui.label(RichText::new(t).color(color));
                }
                Piece::Rule(rule) => {
                    let shown = ui.link(rtitle(texts, rule)).on_hover_text(&rule.id);
                    if shown.clicked() {
                        acts.push(link.act(&rule.id));
                    }
                }
                #[cfg(feature = "full")]
                Piece::Key(path) => {
                    if ui.link(path.as_str()).clicked() {
                        acts.push(Act::Registry(RegistryAct::Open(path.clone(), None)));
                    }
                }
                #[cfg(not(feature = "full"))]
                Piece::Key(path) => {
                    ui.label(RichText::new(path).color(color));
                }
            }
        }
    });
}

fn ptitle<'a>(texts: Option<&'a CatalogTexts>, rule: &'a Rule, name: &'a str) -> &'a str {
    match texts {
        Some(t) => t.param_title(rule, name),
        None => rule.params.get(name).map_or(name, |p| p.title()),
    }
}

/// Whether a rule matches every word of the search.
pub(crate) fn matches(texts: Option<&CatalogTexts>, rule: &Rule, words: &[String]) -> bool {
    if words.is_empty() {
        return true;
    }
    let mut hay = format!(
        "{} {} {} {} {}",
        rule.id,
        rule.title,
        rule.summary,
        rule.tags.join(" "),
        rtitle(texts, rule)
    );
    hay.push(' ');
    hay.push_str(rsummary(texts, rule));
    matches_words(&hay, words)
}

/// A small on and off switch.
fn switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let height = ui.spacing().interact_size.y * 0.75;
    let (rect, mut response) =
        ui.allocate_exact_size(egui::vec2(height * 1.9, height), egui::Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    if ui.is_rect_visible(rect) {
        let p = style::palette(ui);
        let t = ui.ctx().animate_bool_responsive(response.id, *on);
        let radius = rect.height() / 2.0;
        let track = if *on { p.check } else { p.off };
        ui.painter().rect_filled(rect, radius, track);
        if response.has_focus() {
            ui.painter().rect_stroke(
                rect.expand(2.0),
                radius + 2.0,
                Stroke::new(1.5, p.accent),
                egui::StrokeKind::Outside,
            );
        }
        let x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), t);
        ui.painter()
            .circle_filled(egui::pos2(x, rect.center().y), radius * 0.72, p.check_mark);
    }
    response
}

pub(crate) fn mechanism_title(m: winweaver_core::catalog::Mechanism) -> &'static str {
    match m {
        winweaver_core::catalog::Mechanism::Unattend => "Answer file",
        winweaver_core::catalog::Mechanism::Registry => "Registry",
        winweaver_core::catalog::Mechanism::Script => "Script",
        winweaver_core::catalog::Mechanism::Other => "Other",
    }
}

pub(crate) fn phase_title(p: winweaver_core::catalog::Phase) -> &'static str {
    use winweaver_core::catalog::Phase;
    match p {
        Phase::Windowspe => "windowsPE",
        Phase::SpecializeXml | Phase::Specialize => "specialize",
        Phase::OobeXml => "oobeSystem",
        Phase::DefaultUser => "default user",
        Phase::UserFirstLogon => "first sign-in",
        Phase::PostOobe => "after OOBE",
    }
}

pub(crate) fn level_title(l: winweaver_core::catalog::Level) -> &'static str {
    use winweaver_core::catalog::Level;
    match l {
        Level::Optional => "optional",
        Level::Baseline => "baseline",
        Level::Recommended => "recommended",
        Level::Risky => "risky",
    }
}

/// The parameter of a rule in a few words, for the right end of its row.
fn param_summary(catalog: &Catalog, doc: &ProfileDoc, rule: &Rule) -> String {
    let Some((name, param)) = rule.params.iter().next() else {
        return String::new();
    };
    let value = doc
        .profile
        .param(catalog, &rule.id, name)
        .unwrap_or(Value::Null);
    let text = match param {
        Param::Enum { values, .. } => values
            .iter()
            .find(|v| check::same_value(&v.value, &value))
            .map_or_else(|| value.to_string(), |v| v.title.clone()),
        Param::List { .. } => {
            let n = value.as_array().map_or(0, |a| {
                a.iter()
                    .filter(|v| v.as_str().is_some_and(|t| !t.trim().is_empty()))
                    .count()
            });
            trf("{n} items", &[("n", &n)])
        }
        _ => match &value {
            Value::String(s) => s.clone(),
            other => other.to_string(),
        },
    };
    let mut short: String = text.chars().take(24).collect();
    if text.chars().count() > 24 {
        short.push_str("...");
    }
    short
}

fn form_icon(form: Form) -> &'static str {
    match form {
        Form::Install => icons::CLIPBOARD,
        Form::Accounts => icons::PERSON,
        Form::Languages => icons::GLOBE,
    }
}

/// The colours of the parts of the description of an option, added to the themes: a stripe on
/// the left of each part and a faint tint of its card, so that the eye finds a part without
/// reading its title. The part `desc-<kind>` takes the colour `section-<kind>`.
pub const SECTION_COLORS: [style::ExtraColor; 6] = [
    style::ExtraColor {
        name: "section-what",
        label: "Part of the description: what it does",
        light: egui::Color32::from_rgb(0x3b, 0x82, 0xf6),
        dark: egui::Color32::from_rgb(0x60, 0xa5, 0xfa),
    },
    style::ExtraColor {
        name: "section-risk",
        label: "Part of the description: risk",
        light: egui::Color32::from_rgb(0xdc, 0x26, 0x26),
        dark: egui::Color32::from_rgb(0xf8, 0x71, 0x71),
    },
    style::ExtraColor {
        name: "section-writes",
        label: "Part of the description: what it writes",
        light: egui::Color32::from_rgb(0x0d, 0x94, 0x88),
        dark: egui::Color32::from_rgb(0x2d, 0xd4, 0xbf),
    },
    style::ExtraColor {
        name: "section-deps",
        label: "Part of the description: dependencies and conflicts",
        light: egui::Color32::from_rgb(0x7c, 0x3a, 0xed),
        dark: egui::Color32::from_rgb(0xa7, 0x8b, 0xfa),
    },
    style::ExtraColor {
        name: "section-more",
        label: "Part of the description: versions, check and rollback",
        light: egui::Color32::from_rgb(0xd9, 0x77, 0x06),
        dark: egui::Color32::from_rgb(0xfb, 0xbf, 0x24),
    },
    style::ExtraColor {
        name: "section-ref",
        label: "Part of the description: reference",
        light: egui::Color32::from_rgb(0x64, 0x74, 0x8b),
        dark: egui::Color32::from_rgb(0x94, 0xa3, 0xb8),
    },
];

/// A part of the description: a card that folds, with the colour of its kind on its left edge.
fn section(ui: &mut egui::Ui, id: &str, title: &str, open: bool, add: impl FnOnce(&mut egui::Ui)) {
    let p = style::palette(ui);
    let kind = id.strip_prefix("desc-").unwrap_or(id);
    let color = style::extra(ui, &format!("section-{kind}"));
    let shown = egui::Frame::new()
        .outer_margin(Margin {
            left: 18,
            right: 18,
            top: 0,
            bottom: 6,
        })
        .fill(p.card.lerp_to_gamma(color, 0.05))
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin {
            left: 16,
            right: 12,
            top: 6,
            bottom: 6,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            // The state of the setting is part of the id, so a change of it shows at once.
            egui::CollapsingHeader::new(style::strong(ui, title, 13.0))
                .id_salt((id, open))
                .default_open(open)
                .show(ui, add);
        });
    let rect = shown.response.rect;
    let stripe = egui::Rect::from_min_size(rect.min, egui::vec2(4.0, rect.height()));
    let corners = CornerRadius {
        nw: 6,
        sw: 6,
        ne: 0,
        se: 0,
    };
    ui.painter().rect_filled(stripe, corners, color);
}

/// The id of an item, selectable, with a button that copies it.
pub(crate) fn id_line(ui: &mut egui::Ui, id: &str) {
    let p = style::palette(ui);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 2.0;
        style::copy_label(
            ui,
            RichText::new(id).font(style::mono(ui, 12.0)).color(p.muted),
            false,
        );
        if style::icon_button(ui, icons::CLIPBOARD, tr("Copy the id"), true, None).clicked() {
            ui.ctx().copy_text(id.to_owned());
        }
    });
}

pub(crate) fn describe(action: &Action) -> String {
    match action {
        Action::Reg {
            path,
            name,
            kind,
            value,
            ..
        } => format!("reg  {path}\\{name} = {value} ({kind:?})"),
        Action::RegRemove { path, name, .. } => format!("reg-remove  {path}\\{name}"),
        Action::RegList {
            path,
            param,
            names,
            additive,
            ..
        } => {
            let add = if *additive { ", additive" } else { "" };
            format!("reg-list  {path} from {param} ({names:?}{add})")
        }
        Action::Service { name, start, .. } => format!("service  {name}: start type {start}"),
        Action::Exe { file, args } => format!("exe  {file} {}", args.join(" ")),
        Action::Feature { name, state, .. } => format!("feature  {name}: {state}"),
        Action::Capability { pattern } => format!("capability  {pattern}"),
        Action::Appx { names } => format!("appx  {}", names.join(", ")),
        Action::Ps { script } => format!("ps  {} lines of script", script.lines().len()),
        Action::XmlSetting {
            pass,
            component,
            path,
            value,
        } => format!("xml-setting  {pass} {component} {path} = {value}"),
        Action::XmlOobe { element, value } => format!("xml-oobe  {element} = {value}"),
        Action::XmlPeCommand { command, .. } | Action::XmlSpecializeCommand { command, .. } => {
            format!("{}  {command}", action.kind())
        }
    }
}

fn value_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn default_text(default: &Option<Value>) -> String {
    match default {
        None => String::new(),
        Some(Value::String(s)) if s == "absent" => tr("absent").to_owned(),
        Some(Value::String(s)) if s == "unknown" => tr("unknown").to_owned(),
        Some(v) => value_text(v),
    }
}

#[cfg(feature = "full")]
/// The name of a policy in its id, without the prefix and the `.off` of the Disabled state.
fn policy_name(id: &str) -> &str {
    let id = id.strip_suffix(".off").unwrap_or(id);
    id.rsplit('.').next().unwrap_or(id)
}

/// The import whose tree holds a group: the title of the group under the root of the policies.
fn import_of<'a>(catalog: &'a Catalog, texts: Option<&'a CatalogTexts>, group: &str) -> &'a str {
    catalog
        .path_of(group)
        .get(1)
        .and_then(|id| catalog.group(id))
        .map_or("", |g| gtitle(texts, g))
}
