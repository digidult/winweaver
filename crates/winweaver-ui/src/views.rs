//! The profile views: the tree on the left and the detail panel on the right.

use eframe::egui::{self, RichText};
use serde_json::Value;
use winweaver_core::catalog::{
    Action, Catalog, Group, ListNames, Param, ROOT_UNATTEND, Rule, check,
};
use winweaver_core::i18n::CatalogTexts;
use winweaver_core::resolver::State;

use crate::i18n::{tr, trf};
use crate::{Act, ProfileDoc, ScriptKind, Search, Sel, Settings};

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

fn ptitle<'a>(texts: Option<&'a CatalogTexts>, rule: &'a Rule, name: &'a str) -> &'a str {
    match texts {
        Some(t) => t.param_title(rule, name),
        None => rule.params.get(name).map_or(name, |p| p.title()),
    }
}

/// Whether a rule matches every word of the search.
fn matches(texts: Option<&CatalogTexts>, rule: &Rule, words: &[String]) -> bool {
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
    let hay = hay.to_lowercase();
    words.iter().all(|w| hay.contains(w.as_str()))
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
        let t = ui.ctx().animate_bool_responsive(response.id, *on);
        let visuals = ui.style().interact_selectable(&response, *on);
        let radius = rect.height() / 2.0;
        ui.painter().rect_filled(rect, radius, visuals.bg_fill);
        let x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), t);
        ui.painter().circle_filled(
            egui::pos2(x, rect.center().y),
            radius * 0.72,
            visuals.fg_stroke.color,
        );
    }
    response
}

fn mechanism_title(m: winweaver_core::catalog::Mechanism) -> &'static str {
    match m {
        winweaver_core::catalog::Mechanism::Unattend => "Answer file",
        winweaver_core::catalog::Mechanism::Policy => "Policy",
        winweaver_core::catalog::Mechanism::Other => "Other",
    }
}

fn phase_title(p: winweaver_core::catalog::Phase) -> &'static str {
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

fn level_title(l: winweaver_core::catalog::Level) -> &'static str {
    use winweaver_core::catalog::Level;
    match l {
        Level::Optional => "optional",
        Level::Baseline => "baseline",
        Level::Recommended => "recommended",
        Level::Risky => "risky",
    }
}

fn twisty(ui: &mut egui::Ui, open: bool) -> bool {
    let (_rect, response) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::click());
    egui::collapsing_header::paint_default_icon(ui, if open { 1.0 } else { 0.0 }, &response);
    response.clicked()
}

pub fn tree_view(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    doc: &ProfileDoc,
    search: &Search,
    inline: bool,
    acts: &mut Vec<Act>,
) {
    let words = search.words();
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| {
            for root in catalog.child_groups(None) {
                group_rows(ui, catalog, texts, doc, &words, &root.id, inline, acts);
            }
        });
}

#[allow(clippy::too_many_arguments)]
fn group_rows(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    doc: &ProfileDoc,
    words: &[String],
    gid: &str,
    inline: bool,
    acts: &mut Vec<Act>,
) {
    let Some(group) = catalog.group(gid) else {
        return;
    };
    let rules = catalog.rules_under(gid);
    if !words.is_empty() && !rules.iter().any(|r| matches(texts, r, words)) {
        return;
    }
    let total = rules.len();
    let on = rules
        .iter()
        .filter(|r| doc.resolution.shown_on(&r.id))
        .count();
    let expanded = !words.is_empty() || doc.expanded.contains(gid);
    ui.horizontal(|ui| {
        if twisty(ui, expanded) {
            acts.push(Act::Expand(gid.to_owned(), !expanded));
        }
        let mut checked = total > 0 && on == total;
        let mixed = on > 0 && on < total;
        if ui
            .add(egui::Checkbox::without_text(&mut checked).indeterminate(mixed))
            .changed()
        {
            acts.push(Act::SetGroup(gid.to_owned(), checked));
        }
        let selected = matches!(&doc.sel, Sel::Group(id) if id == gid);
        if ui
            .selectable_label(selected, gtitle(texts, group))
            .clicked()
        {
            acts.push(Act::Select(Sel::Group(gid.to_owned())));
        }
        ui.weak(format!("{on}/{total}"));
    });
    if expanded {
        ui.indent(gid, |ui| {
            if gid == ROOT_UNATTEND && words.is_empty() {
                for form in crate::forms::Form::ALL {
                    ui.horizontal(|ui| {
                        ui.add_space(14.0);
                        let selected = matches!(&doc.sel, Sel::Form(f) if *f == form);
                        if ui.selectable_label(selected, tr(form.title())).clicked() {
                            acts.push(Act::Select(Sel::Form(form)));
                        }
                    });
                }
            }
            for child in catalog.child_groups(Some(gid)) {
                group_rows(ui, catalog, texts, doc, words, &child.id, inline, acts);
            }
            for rule in catalog.rules_in(gid) {
                if matches(texts, rule, words) {
                    rule_row(ui, catalog, texts, doc, rule, inline, acts);
                }
            }
        });
    }
}

/// The built-in rule that writes the values of an imported policy chosen off, when one does.
fn linked_rule<'a>(doc: &'a ProfileDoc, rule: &Rule) -> Option<&'a str> {
    doc.resolution
        .linked_by(&rule.id)
        .filter(|_| !doc.profile.is_on(&rule.id))
}

/// The check box of a rule. A linked policy (a built-in rule in effect writes its values) is drawn
/// checked and cannot be changed here; its tip names the built-in rule.
fn rule_checkbox(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    doc: &ProfileDoc,
    rule: &Rule,
    label: &str,
    acts: &mut Vec<Act>,
) {
    if let Some(by) = linked_rule(doc, rule) {
        let by_title = catalog.rule(by).map_or(by, |r| rtitle(texts, r));
        let mut shown = true;
        ui.add_enabled(false, egui::Checkbox::new(&mut shown, label))
            .on_disabled_hover_text(trf("Set by the built-in rule {rule}", &[("rule", &by_title)]));
        return;
    }
    let mut on = doc.profile.is_on(&rule.id);
    if ui.checkbox(&mut on, label).changed() {
        acts.push(Act::Toggle(rule.id.clone(), on));
    }
}

fn rule_row(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    doc: &ProfileDoc,
    rule: &Rule,
    inline: bool,
    acts: &mut Vec<Act>,
) {
    ui.horizontal(|ui| {
        ui.add_space(14.0);
        rule_checkbox(ui, catalog, texts, doc, rule, tr(""), acts);
        let state = doc.resolution.state(&rule.id);
        let title = rtitle(texts, rule);
        let text = match state {
            State::Blocked { .. } => RichText::new(title).weak(),
            _ => RichText::new(title),
        };
        let selected = matches!(&doc.sel, Sel::Rule(id) if id == &rule.id);
        let response = ui.selectable_label(selected, text);
        let clicked = response.clicked();
        if let State::Blocked { by } = state {
            response.on_hover_text(trf("Blocked: needs {by}", &[("by", by)]));
        }
        if clicked {
            acts.push(Act::Select(Sel::Rule(rule.id.clone())));
        }
        if !rule.params.is_empty() {
            ui.weak(trf("{n} parameters", &[("n", &rule.params.len())]));
        } else if inline
            && linked_rule(doc, rule).is_none()
            && matches!(&doc.sel, Sel::Group(g) if g == &rule.group)
        {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let mut on = doc.profile.is_on(&rule.id);
                if switch(ui, &mut on).on_hover_text(tr("On or off")).changed() {
                    acts.push(Act::Toggle(rule.id.clone(), on));
                }
            });
        }
    });
}

pub fn detail_view(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    doc: &ProfileDoc,
    settings: &Settings,
    acts: &mut Vec<Act>,
) {
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| match &doc.sel {
            Sel::None => profile_detail(ui, doc),
            Sel::Group(id) => group_detail(ui, catalog, texts, doc, id, acts),
            Sel::Rule(id) => rule_detail(ui, catalog, texts, doc, settings, id, acts),
            Sel::Form(form) => {
                crate::forms::form_view(ui, doc, *form, &crate::i18n::current(), acts);
            }
        });
}

fn profile_detail(ui: &mut egui::Ui, doc: &ProfileDoc) {
    let p = &doc.profile;
    ui.heading(&p.name);
    ui.weak(trf("Catalog {version}", &[("version", &p.catalog)]));
    if !p.comment.is_empty() {
        ui.label(&p.comment);
    }
    ui.add_space(8.0);
    ui.strong(tr("Installation"));
    ui.label(trf(
        "Edition {edition}, product key {key}, time zone {zone}, computer name {name}, accounts {accounts}, disk {disk}",
        &[
            ("edition", &p.install.edition),
            ("key", &p.install.product_key_mode),
            ("zone", &p.install.time_zone),
            ("name", &p.install.computer_name_mode),
            ("accounts", &p.install.account_mode),
            ("disk", &p.install.disk_mode),
        ],
    ));
    ui.strong(tr("Languages"));
    ui.label(trf(
        "Display {ui}, system {system}, user {user}, input {input}",
        &[
            ("ui", &p.languages.ui_language),
            ("system", &p.languages.system_locale),
            ("user", &p.languages.user_locale),
            ("input", &p.languages.input.join(", ")),
        ],
    ));
    ui.strong(tr("Accounts"));
    for a in &p.accounts {
        ui.label(format!("{} ({})", a.name, a.group));
    }
    ui.add_space(8.0);
    ui.weak(tr("Select a form, a group or a rule in the tree."));
}

fn group_detail(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    doc: &ProfileDoc,
    id: &str,
    acts: &mut Vec<Act>,
) {
    let Some(group) = catalog.group(id) else {
        return;
    };
    ui.heading(gtitle(texts, group));
    ui.monospace(&group.id);
    let rules = catalog.rules_under(id);
    let on = rules
        .iter()
        .filter(|r| doc.resolution.shown_on(&r.id))
        .count();
    ui.weak(trf(
        "{n} rules, {on} in effect",
        &[("n", &rules.len()), ("on", &on)],
    ));
    ui.horizontal(|ui| {
        if ui.button(tr("Turn all on")).clicked() {
            acts.push(Act::SetGroup(id.to_owned(), true));
        }
        if ui.button(tr("Turn all off")).clicked() {
            acts.push(Act::SetGroup(id.to_owned(), false));
        }
    });
    let summary = gsummary(texts, group);
    if !summary.is_empty() {
        ui.add_space(6.0);
        ui.label(summary);
    }
    ui.add_space(8.0);
    this_pc_card(ui, acts);
    ui.add_space(8.0);
    ui.strong(tr("Rules of this group"));
    for rule in catalog.rules_in(id) {
        ui.horizontal(|ui| {
            rule_checkbox(ui, catalog, texts, doc, rule, rtitle(texts, rule), acts);
            if ui.small_button(tr("open")).clicked() {
                acts.push(Act::Select(Sel::Rule(rule.id.clone())));
            }
        });
    }
}

/// The commands of This PC for the selected rule or group.
fn this_pc_card(ui: &mut egui::Ui, acts: &mut Vec<Act>) {
    ui.add_space(8.0);
    ui.strong(tr("This PC, this selection"));
    ui.horizontal_wrapped(|ui| {
        if ui.button(tr("Check on this PC")).clicked() {
            acts.push(Act::CheckOnPc);
        }
        if ui.button(tr("Read into the profile")).clicked() {
            acts.push(Act::ReadSelection);
        }
        if ui.button(tr("Save an apply script...")).clicked() {
            acts.push(Act::SaveScript(ScriptKind::Apply));
        }
        if ui
            .button(tr("Save a return-to-defaults script..."))
            .clicked()
        {
            acts.push(Act::SaveScript(ScriptKind::Revert));
        }
        if ui.button(tr("Apply to this PC...")).clicked() {
            acts.push(Act::PlanNative(false));
        }
        if ui.button(tr("Return to Windows defaults...")).clicked() {
            acts.push(Act::PlanNative(true));
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

fn rule_detail(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    doc: &ProfileDoc,
    settings: &Settings,
    id: &str,
    acts: &mut Vec<Act>,
) {
    let Some(rule) = catalog.rule(id) else {
        return;
    };
    ui.label(RichText::new(rtitle(texts, rule)).size(18.0).strong());
    ui.monospace(&rule.id);
    ui.horizontal(|ui| {
        ui.weak(tr(mechanism_title(Catalog::mechanism(rule))));
        ui.weak(phase_title(rule.phase));
        ui.weak(tr(level_title(rule.level)));
        if rule.default {
            ui.weak(tr("on by default"));
        }
    });
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        let mut on = doc.profile.is_on(&rule.id);
        if ui.checkbox(&mut on, tr("On")).changed() {
            acts.push(Act::Toggle(rule.id.clone(), on));
        }
        match doc.resolution.state(&rule.id) {
            State::On => ui.weak(tr("in effect")),
            State::Off => match doc.resolution.linked_by(&rule.id) {
                Some(by) => {
                    ui.weak(tr("set by the built-in rule"));
                    let title = catalog.rule(by).map_or(by, |r| rtitle(texts, r));
                    if ui.link(title).clicked() {
                        acts.push(Act::Select(Sel::Rule(by.to_owned())));
                    }
                    ui.weak(tr(""))
                }
                None => ui.weak(tr("off")),
            },
            State::Blocked { by } => {
                ui.weak(tr("blocked: needs"));
                let title = catalog.rule(by).map_or(by.as_str(), |r| rtitle(texts, r));
                if ui.link(title).clicked() {
                    acts.push(Act::Select(Sel::Rule(by.clone())));
                }
                ui.weak(tr(""))
            }
        };
    });
    if !rule.params.is_empty() {
        ui.add_space(6.0);
        ui.strong(tr("Parameters"));
        for (name, param) in &rule.params {
            param_editor(ui, catalog, texts, doc, rule, name, param, acts);
        }
    }
    this_pc_card(ui, acts);
    if !settings.desc_open {
        ui.add_space(6.0);
        ui.weak(tr("Description folded (Ctrl+D)"));
        return;
    }
    ui.add_space(10.0);
    egui::CollapsingHeader::new(tr("What it does"))
        .id_salt("desc-what")
        .default_open(true)
        .show(ui, |ui| {
            ui.label(rsummary(texts, rule));
            if !rule.effect.is_empty() {
                ui.label(&rule.effect);
            }
        });
    if !rule.risk.is_empty() {
        egui::CollapsingHeader::new(tr("Risk"))
            .id_salt("desc-risk")
            .default_open(true)
            .show(ui, |ui| {
                ui.label(&rule.risk);
            });
    }
    egui::CollapsingHeader::new(tr("What it writes"))
        .id_salt("desc-writes")
        .show(ui, |ui| {
            for action in &rule.actions {
                ui.monospace(describe(action));
            }
        });
    if !rule.requires.is_empty() || !rule.conflicts.is_empty() {
        egui::CollapsingHeader::new(tr("Dependencies and conflicts"))
            .id_salt("desc-deps")
            .show(ui, |ui| {
                for (label, list) in [
                    (tr("Needs"), &rule.requires),
                    (tr("Conflicts with"), &rule.conflicts),
                ] {
                    for other in list {
                        ui.horizontal(|ui| {
                            ui.weak(label);
                            let title = catalog
                                .rule(other)
                                .map_or(other.as_str(), |r| rtitle(texts, r));
                            if ui.link(title).clicked() {
                                acts.push(Act::Select(Sel::Rule(other.clone())));
                            }
                        });
                    }
                }
            });
    }
    if !rule.versions.is_empty() || !rule.rollback.is_empty() || !rule.verify.is_empty() {
        egui::CollapsingHeader::new(tr("Versions, check and rollback"))
            .id_salt("desc-more")
            .show(ui, |ui| {
                for text in [&rule.versions, &rule.verify, &rule.rollback] {
                    if !text.is_empty() {
                        ui.label(text);
                    }
                }
            });
    }
    if !rule.doc.is_empty() {
        egui::CollapsingHeader::new(tr("Reference"))
            .id_salt("desc-ref")
            .show(ui, |ui| {
                ui.weak(trf("Documentation: {doc}", &[("doc", &rule.doc)]));
            });
    }
}

#[allow(clippy::too_many_arguments)]
fn param_editor(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    doc: &ProfileDoc,
    rule: &Rule,
    name: &str,
    param: &Param,
    acts: &mut Vec<Act>,
) {
    let current = doc
        .profile
        .param(catalog, &rule.id, name)
        .unwrap_or(Value::Null);
    ui.horizontal(|ui| {
        ui.label(ptitle(texts, rule, name));
        match param {
            Param::Enum { values, .. } => {
                let shown = values
                    .iter()
                    .find(|v| check::same_value(&v.value, &current))
                    .map_or_else(|| current.to_string(), |v| v.title.clone());
                egui::ComboBox::from_id_salt(format!("{}.{name}", rule.id))
                    .selected_text(shown)
                    .show_ui(ui, |ui| {
                        for v in values {
                            let chosen = check::same_value(&v.value, &current);
                            if ui.selectable_label(chosen, &v.title).clicked() && !chosen {
                                acts.push(Act::SetParam(
                                    rule.id.clone(),
                                    name.to_owned(),
                                    v.value.clone(),
                                ));
                            }
                        }
                    });
            }
            Param::Int { min, max, .. } => {
                let mut v = current.as_i64().unwrap_or(*min);
                if ui
                    .add(egui::DragValue::new(&mut v).range(*min..=*max))
                    .changed()
                {
                    acts.push(Act::SetParam(
                        rule.id.clone(),
                        name.to_owned(),
                        Value::from(v),
                    ));
                }
            }
            Param::List { .. } => {
                let pairs = rule.actions.iter().any(|a| {
                    matches!(a, Action::RegList { param, names: ListNames::Pairs, .. } if param == name)
                });
                let mut text = current
                    .as_array()
                    .map(|items| {
                        items
                            .iter()
                            .map(|v| v.as_str().unwrap_or_default())
                            .collect::<Vec<_>>()
                            .join("\n")
                    })
                    .unwrap_or_default();
                let hint = if pairs {
                    tr("One name=value per line")
                } else {
                    tr("One item per line")
                };
                if ui
                    .add(
                        egui::TextEdit::multiline(&mut text)
                            .id_salt(format!("{}.{name}", rule.id))
                            .desired_rows(4)
                            .desired_width(320.0)
                            .hint_text(hint),
                    )
                    .changed()
                {
                    let items: Vec<Value> = if text.is_empty() {
                        Vec::new()
                    } else {
                        text.split('\n').map(|l| Value::from(l.trim_end_matches('\r'))).collect()
                    };
                    acts.push(Act::SetParam(
                        rule.id.clone(),
                        name.to_owned(),
                        Value::Array(items),
                    ));
                }
            }
            Param::Text { .. } => {
                let mut s = current.as_str().unwrap_or("").to_owned();
                if ui
                    .add(
                        egui::TextEdit::singleline(&mut s)
                            .id_salt(format!("{}.{name}", rule.id))
                            .desired_width(240.0),
                    )
                    .changed()
                {
                    acts.push(Act::SetParam(
                        rule.id.clone(),
                        name.to_owned(),
                        Value::from(s),
                    ));
                }
            }
        }
        ui.weak(trf("default {value}", &[("value", &param.default_value())]));
    });
}
