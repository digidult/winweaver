//! The profile views: the tree on the left (`tree.rs` paints it) and the detail panel on the right,
//! in cards.

use eframe::egui::{self, CornerRadius, Margin, RichText, Stroke};
use serde_json::Value;
use winweaver_core::admx::CatalogFile;
use winweaver_core::catalog::{
    Action, Catalog, Group, ListNames, Param, ROOT_UNATTEND, Rule, check,
};
use winweaver_core::i18n::CatalogTexts;
use winweaver_core::resolver::State;

use crate::i18n::{tr, trf};
use crate::{Act, Form, ProfileDoc, ScriptKind, Search, Sel, Settings, UNKNOWN_GROUP, style, tree};

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
        winweaver_core::catalog::Mechanism::Policy => "Policy",
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
        Form::Install => "\u{1F4CB}",
        Form::Accounts => "\u{1F464}",
        Form::Languages => "\u{1F310}",
    }
}

/// The key of a row of the profile tree: what it selects and, for a rule, the group it is drawn
/// under, since a policy of two imports shows in both trees.
type Key = (Sel, Option<String>);

/// The rows of the profile tree under a group, in the order they are shown.
#[allow(clippy::too_many_arguments)]
fn tree_rows(
    rows: &mut Vec<tree::Row<Key>>,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    doc: &ProfileDoc,
    words: &[String],
    gid: &str,
    depth: usize,
    inline: bool,
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
    let open = !words.is_empty() || doc.expanded.contains(gid);
    let check = if total > 0 && on == total {
        tree::Check::On
    } else if on > 0 {
        tree::Check::Mixed
    } else {
        tree::Check::Off
    };
    rows.push(tree::Row {
        key: (Sel::Group(gid.to_owned()), None),
        depth,
        title: gtitle(texts, group).to_owned(),
        right: format!("{on}/{total}"),
        check,
        open: Some(open),
        muted: false,
        icon: None,
        switch: None,
    });
    if !open {
        return;
    }
    if gid == ROOT_UNATTEND && words.is_empty() {
        for form in Form::ALL {
            rows.push(tree::Row {
                key: (Sel::Form(form), None),
                depth: depth + 1,
                title: tr(form.title()).to_owned(),
                right: String::new(),
                check: tree::Check::None,
                open: None,
                muted: false,
                icon: Some(form_icon(form)),
                switch: None,
            });
        }
    }
    for child in catalog.child_groups(Some(gid)) {
        tree_rows(
            rows,
            catalog,
            texts,
            doc,
            words,
            &child.id,
            depth + 1,
            inline,
        );
    }
    let group_selected = matches!(&doc.sel, Sel::Group(g) if g == gid);
    for rule in catalog.rules_in(gid) {
        if !matches(texts, rule, words) {
            continue;
        }
        let linked = linked_rule(doc, rule).is_some();
        let chosen = doc.profile.is_on(&rule.id);
        let blocked = matches!(doc.resolution.state(&rule.id), State::Blocked { .. });
        let check = if linked {
            tree::Check::Linked
        } else if chosen && blocked {
            tree::Check::Blocked
        } else if chosen {
            tree::Check::On
        } else {
            tree::Check::Off
        };
        let switch =
            (inline && group_selected && rule.params.is_empty() && !linked).then_some(chosen);
        rows.push(tree::Row {
            key: (Sel::Rule(rule.id.clone()), Some(gid.to_owned())),
            depth: depth + 1,
            title: rtitle(texts, rule).to_owned(),
            right: if switch.is_some() {
                String::new()
            } else {
                param_summary(catalog, doc, rule)
            },
            check,
            open: None,
            muted: blocked,
            icon: None,
            switch,
        });
    }
}

/// Whether the saved choice of an option whose catalog is not loaded is on.
fn unknown_on(choice: &Value) -> bool {
    choice.get("on").and_then(Value::as_bool).unwrap_or(false)
}

/// The group of the options of the profile whose catalog is not loaded, at the end of the tree:
/// their ids, not to be switched.
fn unknown_rows(rows: &mut Vec<tree::Row<Key>>, doc: &ProfileDoc, words: &[String]) {
    let unknown = &doc.profile.unknown;
    let shown: Vec<(&String, &Value)> = unknown
        .iter()
        .filter(|(id, _)| {
            let id = id.to_lowercase();
            words.iter().all(|w| id.contains(w.as_str()))
        })
        .collect();
    if unknown.is_empty() || shown.is_empty() {
        return;
    }
    let open = !words.is_empty() || doc.expanded.contains(UNKNOWN_GROUP);
    rows.push(tree::Row {
        key: (Sel::Group(UNKNOWN_GROUP.to_owned()), None),
        depth: 0,
        title: tr("Unknown options").to_owned(),
        right: unknown.len().to_string(),
        check: tree::Check::None,
        open: Some(open),
        muted: false,
        icon: None,
        switch: None,
    });
    if !open {
        return;
    }
    for (id, choice) in shown {
        rows.push(tree::Row {
            key: (Sel::Unknown(id.clone()), None),
            depth: 1,
            title: id.clone(),
            right: if unknown_on(choice) {
                tr("on")
            } else {
                tr("off")
            }
            .to_owned(),
            check: tree::Check::None,
            open: None,
            muted: true,
            icon: None,
            switch: None,
        });
    }
}

#[allow(clippy::too_many_arguments)]
pub fn tree_view(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    doc: &ProfileDoc,
    search: &Search,
    inline: bool,
    show_unknown: bool,
    acts: &mut Vec<Act>,
) {
    let words = search.words();
    let mut rows = Vec::new();
    for root in catalog.child_groups(None) {
        tree_rows(&mut rows, catalog, texts, doc, &words, &root.id, 0, inline);
    }
    if show_unknown {
        unknown_rows(&mut rows, doc, &words);
    }
    if !words.is_empty() && rows.is_empty() {
        ui.weak(tr("Nothing matches the search."));
    }
    let selected = rows
        .iter()
        .position(|r| {
            r.key.0 == doc.sel
                && doc
                    .focus
                    .as_ref()
                    .is_none_or(|g| r.key.1.as_ref() == Some(g))
        })
        .or_else(|| rows.iter().position(|r| r.key.0 == doc.sel));
    let scroll = if doc.reveal {
        tree::Scroll::Center
    } else if doc.follow {
        tree::Scroll::Near
    } else {
        tree::Scroll::None
    };
    if selected.is_none() && scroll != tree::Scroll::None {
        acts.push(Act::Revealed);
    }
    let id = egui::Id::new(("profile-tree", doc.draft_key.as_str()));
    let out = tree::show(ui, id, &rows, selected, scroll);
    for event in out.events {
        match event {
            tree::Event::Select((sel, group)) => acts.push(Act::SelectIn(sel, group)),
            tree::Event::Expand((Sel::Group(g), _), open) => acts.push(Act::Expand(g, open)),
            tree::Event::Toggle((Sel::Rule(id), _)) => {
                let on = !doc.profile.is_on(&id);
                acts.push(Act::Toggle(id, on));
            }
            tree::Event::Switch((Sel::Rule(id), _), on) => acts.push(Act::Toggle(id, on)),
            tree::Event::Toggle((Sel::Group(g), _)) => {
                let rules = catalog.rules_under(&g);
                let all = !rules.is_empty() && rules.iter().all(|r| doc.resolution.shown_on(&r.id));
                acts.push(Act::SetGroup(g, !all));
            }
            _ => {}
        }
    }
    if out.moved {
        acts.push(Act::Follow);
    }
    if out.scrolled {
        acts.push(Act::Revealed);
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
            .on_disabled_hover_text(trf(
                "Set by the built-in rule {rule}",
                &[("rule", &by_title)],
            ));
        return;
    }
    let mut on = doc.profile.is_on(&rule.id);
    if ui.checkbox(&mut on, label).changed() {
        acts.push(Act::Toggle(rule.id.clone(), on));
    }
}

/// The head of the detail panel: the card colour across the width.
pub(crate) fn header(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    let p = style::palette(ui);
    egui::Frame::new()
        .fill(p.card)
        .inner_margin(Margin {
            left: 18,
            right: 18,
            top: 14,
            bottom: 12,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            add(ui);
        });
    ui.painter().hline(
        ui.min_rect().x_range(),
        ui.cursor().top(),
        Stroke::new(1.0, p.border),
    );
}

/// A card of the detail panel, with its margins.
pub(crate) fn block(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    egui::Frame::new()
        .outer_margin(Margin {
            left: 18,
            right: 18,
            top: 10,
            bottom: 0,
        })
        .show(ui, |ui| {
            style::card(ui, add);
        });
}

/// A part of the description: a card that folds.
fn section(ui: &mut egui::Ui, id: &str, title: &str, open: bool, add: impl FnOnce(&mut egui::Ui)) {
    let p = style::palette(ui);
    egui::Frame::new()
        .outer_margin(Margin {
            left: 18,
            right: 18,
            top: 0,
            bottom: 6,
        })
        .fill(p.card)
        .stroke(Stroke::new(1.0, p.border))
        .corner_radius(CornerRadius::same(6))
        .inner_margin(Margin::symmetric(12, 6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            // The state of the setting is part of the id, so a change of it shows at once.
            egui::CollapsingHeader::new(style::strong(ui, title, 13.0))
                .id_salt((id, open))
                .default_open(open)
                .show(ui, add);
        });
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
        if style::icon_button(ui, "\u{1F4CB}", tr("Copy the id"), true, None).clicked() {
            ui.ctx().copy_text(id.to_owned());
        }
    });
}

fn profile_detail(ui: &mut egui::Ui, doc: &ProfileDoc) {
    let p = &doc.profile;
    header(ui, |ui| {
        ui.label(style::strong(ui, p.name.as_str(), 18.0));
        ui.horizontal_wrapped(|ui| {
            style::chip(
                ui,
                &trf("Catalog {version}", &[("version", &p.catalog)]),
                true,
            );
        });
        if !p.comment.is_empty() {
            ui.add_space(4.0);
            style::selectable_text(ui, "profile-comment", &p.comment);
        }
    });
    block(ui, |ui| {
        style::section_title(ui, tr("Installation"));
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
    });
    block(ui, |ui| {
        style::section_title(ui, tr("Languages"));
        ui.label(trf(
            "Display {ui}, system {system}, user {user}, input {input}",
            &[
                ("ui", &p.languages.ui_language),
                ("system", &p.languages.system_locale),
                ("user", &p.languages.user_locale),
                ("input", &p.languages.input.join(", ")),
            ],
        ));
    });
    block(ui, |ui| {
        style::section_title(ui, tr("Accounts"));
        for a in &p.accounts {
            ui.label(format!("{} ({})", a.name, a.group));
        }
    });
    ui.add_space(10.0);
    egui::Frame::new()
        .inner_margin(Margin::symmetric(18, 0))
        .show(ui, |ui| {
            ui.weak(tr("Select a form, a group or a rule in the tree."));
        });
}

/// The button that looks for the catalogs of the unknown options among the imports.
fn load_catalogs_button(ui: &mut egui::Ui, acts: &mut Vec<Act>) {
    if ui
        .button(tr("Load catalogs"))
        .on_hover_text(tr(
            "Looks for these options in the imported catalogs that are not loaded and loads only the ones that have them",
        ))
        .clicked()
    {
        acts.push(Act::LoadCatalogs);
    }
}

/// The group of the options of catalogs that are not loaded.
fn unknown_group_detail(ui: &mut egui::Ui, doc: &ProfileDoc, acts: &mut Vec<Act>) {
    let unknown = &doc.profile.unknown;
    header(ui, |ui| {
        ui.label(style::strong(ui, tr("Unknown options"), 18.0));
        ui.horizontal_wrapped(|ui| {
            style::chip(ui, &trf("{n} options", &[("n", &unknown.len())]), true);
        });
        ui.add_space(8.0);
        load_catalogs_button(ui, acts);
    });
    block(ui, |ui| {
        style::selectable_text(
            ui,
            "unknown-summary",
            tr(
                "These options of the profile belong to catalogs of policy templates that are not loaded. They stay in the profile unchanged and are left out of the build. Load catalogs looks for them in the imported catalogs that are not loaded and loads only the ones that have them.",
            ),
        )
    });
    block(ui, |ui| {
        style::section_title(ui, tr("Options"));
        for (id, choice) in unknown {
            ui.horizontal(|ui| {
                ui.label(RichText::new(id).monospace());
                ui.weak(if unknown_on(choice) {
                    tr("on")
                } else {
                    tr("off")
                });
                if ui.small_button(tr("open")).clicked() {
                    acts.push(Act::Select(Sel::Unknown(id.clone())));
                }
            });
        }
    });
}

/// An option of the profile whose catalog is not loaded: its id and its saved choice.
fn unknown_detail(ui: &mut egui::Ui, doc: &ProfileDoc, id: &str, acts: &mut Vec<Act>) {
    let Some(choice) = doc.profile.unknown.get(id) else {
        return;
    };
    header(ui, |ui| {
        ui.label(style::strong(ui, tr("Unknown option"), 18.0));
        id_line(ui, id);
        ui.horizontal_wrapped(|ui| {
            let on = unknown_on(choice);
            style::chip(ui, if on { tr("On") } else { tr("Off") }, on);
        });
        ui.add_space(8.0);
        load_catalogs_button(ui, acts);
    });
    block(ui, |ui| {
        style::selectable_text(
            ui,
            ("unknown-option", id),
            tr(
                "This option belongs to a catalog of policy templates that is not loaded. It stays in the profile unchanged and is left out of the build.",
            ),
        )
    });
    if let Some(params) = choice.get("params").and_then(Value::as_object)
        && !params.is_empty()
    {
        block(ui, |ui| {
            style::section_title(ui, tr("Parameters"));
            for (name, value) in params {
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new(name).monospace());
                    ui.weak(value_text(value));
                });
            }
        });
    }
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
    let rules = catalog.rules_under(id);
    let on = rules
        .iter()
        .filter(|r| doc.resolution.shown_on(&r.id))
        .count();
    header(ui, |ui| {
        ui.label(style::strong(ui, gtitle(texts, group), 18.0));
        id_line(ui, &group.id);
        ui.horizontal_wrapped(|ui| {
            style::chip(
                ui,
                &trf(
                    "{n} rules, {on} in effect",
                    &[("n", &rules.len()), ("on", &on)],
                ),
                true,
            );
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            if ui.button(tr("Turn all on")).clicked() {
                acts.push(Act::SetGroup(id.to_owned(), true));
            }
            if ui.button(tr("Turn all off")).clicked() {
                acts.push(Act::SetGroup(id.to_owned(), false));
            }
        });
    });
    let summary = gsummary(texts, group);
    if !summary.is_empty() {
        block(ui, |ui| {
            style::selectable_text(ui, ("group-summary", id), summary)
        });
    }
    block(ui, |ui| {
        style::section_title(ui, tr("This PC, this selection"));
        ui.add_space(4.0);
        this_pc_buttons(ui, acts);
    });
    block(ui, |ui| {
        style::section_title(ui, tr("Rules of this group"));
        for rule in catalog.rules_in(id) {
            ui.horizontal(|ui| {
                rule_checkbox(ui, catalog, texts, doc, rule, rtitle(texts, rule), acts);
                if ui.small_button(tr("open")).clicked() {
                    acts.push(Act::Select(Sel::Rule(rule.id.clone())));
                }
            });
        }
    });
}

/// The commands of This PC for the selected rule or group.
fn this_pc_buttons(ui: &mut egui::Ui, acts: &mut Vec<Act>) {
    ui.horizontal_wrapped(|ui| {
        for (text, act) in [
            (tr("Check on this PC"), Act::CheckOnPc),
            (tr("Read into the profile"), Act::ReadSelection),
            (tr("Apply to this PC..."), Act::PlanNative(false)),
            (tr("Return to Windows defaults..."), Act::PlanNative(true)),
            (
                tr("Save an apply script..."),
                Act::SaveScript(ScriptKind::Apply),
            ),
            (
                tr("Save a return-to-defaults script..."),
                Act::SaveScript(ScriptKind::Revert),
            ),
            (tr("Save a registry file..."), Act::SaveRegFile(false)),
            (
                tr("Save a return-to-defaults registry file..."),
                Act::SaveRegFile(true),
            ),
        ] {
            if ui.button(text).clicked() {
                acts.push(act);
            }
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

/// What a rule writes: the registry values as a table of key, name, value and the default of
/// Windows; the other actions as one line each under it.
fn writes(ui: &mut egui::Ui, rule: &Rule, acts: &mut Vec<Act>) {
    let p = style::palette(ui);
    let mut rows = Vec::new();
    let mut others = Vec::new();
    for action in &rule.actions {
        match action {
            Action::Reg {
                path,
                name,
                value,
                default,
                ..
            } => rows.push((
                path.as_str(),
                name.clone(),
                value_text(value),
                default_text(default),
            )),
            Action::RegRemove {
                path,
                name,
                default,
            } => rows.push((
                path.as_str(),
                name.clone(),
                tr("removed").to_owned(),
                default_text(default),
            )),
            Action::RegList { path, param, .. } => rows.push((
                path.as_str(),
                String::new(),
                trf("the items of the parameter {param}", &[("param", param)]),
                String::new(),
            )),
            other => others.push(describe(other)),
        }
    }
    if !rows.is_empty() {
        let room = (ui.available_width() - 3.0 * 16.0).max(240.0);
        let widths = [room * 0.4, room * 0.22, room * 0.22, room * 0.16];
        let cell = |ui: &mut egui::Ui, text: &str, mono: bool, muted: bool, width: f32| {
            ui.scope(|ui| {
                ui.set_max_width(width);
                let mut rich = RichText::new(text);
                rich = if mono {
                    rich.font(style::mono(ui, 12.0))
                } else {
                    rich.font(style::font(ui, 12.5))
                };
                if muted {
                    rich = rich.color(p.muted);
                }
                style::copy_label(ui, rich, true);
            });
        };
        egui::Grid::new(("writes", rule.id.as_str()))
            .num_columns(4)
            .spacing([16.0, 6.0])
            .show(ui, |ui| {
                for (title, width) in [tr("Key"), tr("Name"), tr("Value"), tr("Windows default")]
                    .into_iter()
                    .zip(widths)
                {
                    ui.scope(|ui| {
                        ui.set_max_width(width);
                        ui.add(
                            egui::Label::new(style::strong(ui, title, 12.0).color(p.muted)).wrap(),
                        );
                    });
                }
                ui.end_row();
                for (path, name, value, default) in &rows {
                    ui.horizontal(|ui| {
                        cell(ui, path, true, false, widths[0] - 24.0);
                        let open = ui
                            .small_button("\u{2197}")
                            .on_hover_text(tr("Open in the registry"));
                        if open.clicked() {
                            let value = (!name.is_empty()).then(|| name.clone());
                            acts.push(Act::OpenRegistry((*path).to_owned(), value));
                        }
                    });
                    cell(ui, name, false, false, widths[1]);
                    cell(ui, value, false, false, widths[2]);
                    cell(ui, default, false, true, widths[3]);
                    ui.end_row();
                }
            });
    }
    for line in others {
        ui.add_space(4.0);
        style::copy_label(ui, RichText::new(line).font(style::mono(ui, 12.0)), true);
    }
}

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

/// The same policy in the trees of other imports and the policies of the same name in other
/// catalogs, as links that select them.
fn other_catalogs(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    rule: &Rule,
    acts: &mut Vec<Act>,
) {
    if !rule.id.starts_with("admx.") {
        return;
    }
    let aliases: Vec<&str> = catalog.alias_groups(&rule.id).collect();
    let name = policy_name(&rule.id);
    let off = rule.id.ends_with(".off");
    let similar: Vec<&Rule> = catalog
        .rules()
        .iter()
        .filter(|r| {
            r.id.starts_with("admx.")
                && r.id != rule.id
                && r.id.ends_with(".off") == off
                && policy_name(&r.id) == name
        })
        .collect();
    if aliases.is_empty() && similar.is_empty() {
        return;
    }
    block(ui, |ui| {
        style::section_title(ui, tr("In other catalogs"));
        ui.add_space(4.0);
        for group in aliases {
            ui.horizontal_wrapped(|ui| {
                ui.weak(tr("The same policy in"));
                if ui.link(import_of(catalog, texts, group)).clicked() {
                    acts.push(Act::ShowIn(rule.id.clone(), group.to_owned()));
                }
            });
        }
        for other in similar {
            ui.horizontal_wrapped(|ui| {
                if ui.link(rtitle(texts, other)).clicked() {
                    acts.push(Act::Select(Sel::Rule(other.id.clone())));
                }
                ui.weak(import_of(catalog, texts, &other.group));
            });
        }
    });
}

/// Links that open the loaded imports holding a rule in their catalog tabs at it.
pub(crate) fn catalog_links(
    ui: &mut egui::Ui,
    imports: &[CatalogFile],
    rule: &str,
    acts: &mut Vec<Act>,
) {
    for f in imports
        .iter()
        .filter(|f| f.rules.iter().any(|r| r.id == rule))
    {
        if ui
            .link(trf("Show in the catalog {name}", &[("name", &f.name)]))
            .clicked()
        {
            acts.push(Act::OpenCatalogAt(f.id.clone(), rule.to_owned()));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn rule_detail(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    imports: &[CatalogFile],
    doc: &ProfileDoc,
    settings: &Settings,
    id: &str,
    acts: &mut Vec<Act>,
) {
    let Some(rule) = catalog.rule(id) else {
        return;
    };
    header(ui, |ui| {
        ui.label(style::strong(ui, rtitle(texts, rule), 18.0));
        id_line(ui, &rule.id);
        ui.horizontal_wrapped(|ui| {
            style::chip(ui, tr(mechanism_title(Catalog::mechanism(rule))), true);
            style::chip(ui, phase_title(rule.phase), false);
            style::chip(ui, tr(level_title(rule.level)), false);
            if rule.default {
                style::chip(ui, tr("on by default"), false);
            }
        });
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            let mut on = doc.profile.is_on(&rule.id);
            if switch(ui, &mut on).changed() {
                acts.push(Act::Toggle(rule.id.clone(), on));
            }
            ui.label(if on { tr("On") } else { tr("Off") });
            ui.add_space(8.0);
            match doc.resolution.state(&rule.id) {
                State::On => {
                    ui.label(style::muted(ui, tr("in effect"), 12.0));
                }
                State::Off => match doc.resolution.linked_by(&rule.id) {
                    Some(by) => {
                        ui.label(style::muted(ui, tr("set by the built-in rule"), 12.0));
                        let title = catalog.rule(by).map_or(by, |r| rtitle(texts, r));
                        if ui.link(title).clicked() {
                            acts.push(Act::Select(Sel::Rule(by.to_owned())));
                        }
                    }
                    None => {
                        ui.label(style::muted(ui, tr("off"), 12.0));
                    }
                },
                State::Blocked { by } => {
                    ui.label(style::muted(ui, tr("blocked: needs"), 12.0));
                    let title = catalog.rule(by).map_or(by.as_str(), |r| rtitle(texts, r));
                    if ui.link(title).clicked() {
                        acts.push(Act::Select(Sel::Rule(by.clone())));
                    }
                }
            }
        });
    });
    if !rule.params.is_empty() {
        block(ui, |ui| {
            style::section_title(ui, tr("Parameters"));
            ui.add_space(4.0);
            egui::Grid::new(("params", rule.id.as_str()))
                .num_columns(3)
                .spacing([14.0, 8.0])
                .show(ui, |ui| {
                    for (name, param) in &rule.params {
                        param_editor(ui, catalog, texts, doc, rule, name, param, acts);
                        ui.end_row();
                    }
                });
        });
    }
    block(ui, |ui| {
        style::section_title(ui, tr("This PC, this rule"));
        ui.add_space(4.0);
        this_pc_buttons(ui, acts);
        catalog_links(ui, imports, &rule.id, acts);
    });
    other_catalogs(ui, catalog, texts, rule, acts);
    rule_description(ui, catalog, texts, rule, settings, true, acts);
}

/// The description of a rule under its title, folded with Ctrl+D: what it does, the risk, what it
/// writes, its dependencies (links that select them when `links`), versions and the reference.
pub(crate) fn rule_description(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    rule: &Rule,
    settings: &Settings,
    links: bool,
    acts: &mut Vec<Act>,
) {
    let open = settings.desc_open;
    let parts = &settings.desc_sections;
    ui.add_space(6.0);
    egui::Frame::new()
        .inner_margin(Margin {
            left: 20,
            right: 18,
            top: 8,
            bottom: 4,
        })
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                style::section_title(ui, tr("Description"));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (icon, hint) = if open {
                        ("\u{25BE}", tr("Fold the description (Ctrl+D)"))
                    } else {
                        ("\u{25B8}", tr("Show the description (Ctrl+D)"))
                    };
                    if style::icon_button(ui, icon, hint, true, None).clicked() {
                        acts.push(Act::Fold);
                    }
                });
            });
        });
    if !open {
        return;
    }
    section(ui, "desc-what", tr("What it does"), parts.what, |ui| {
        style::selectable_text(ui, ("what", rule.id.as_str()), rsummary(texts, rule));
        if !rule.effect.is_empty() {
            style::selectable_text(ui, ("effect", rule.id.as_str()), &rule.effect);
        }
    });
    if !rule.risk.is_empty() {
        section(ui, "desc-risk", tr("Risk"), parts.risk, |ui| {
            style::selectable_text(ui, ("risk", rule.id.as_str()), &rule.risk);
        });
    }
    section(
        ui,
        "desc-writes",
        tr("What it writes"),
        parts.writes,
        |ui| writes(ui, rule, acts),
    );
    if !rule.requires.is_empty() || !rule.conflicts.is_empty() {
        section(
            ui,
            "desc-deps",
            tr("Dependencies and conflicts"),
            parts.deps,
            |ui| {
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
                            if !links {
                                ui.label(title);
                            } else if ui.link(title).clicked() {
                                acts.push(Act::Select(Sel::Rule(other.clone())));
                            }
                        });
                    }
                }
            },
        );
    }
    if !rule.versions.is_empty() || !rule.rollback.is_empty() || !rule.verify.is_empty() {
        section(
            ui,
            "desc-more",
            tr("Versions, check and rollback"),
            parts.more,
            |ui| {
                for (i, text) in [&rule.versions, &rule.verify, &rule.rollback]
                    .into_iter()
                    .enumerate()
                {
                    if !text.is_empty() {
                        style::selectable_text(ui, ("more", rule.id.as_str(), i), text);
                    }
                }
            },
        );
    }
    if !rule.doc.is_empty() {
        section(ui, "desc-ref", tr("Reference"), parts.reference, |ui| {
            if ui.link(rule.doc.as_str()).clicked() {
                acts.push(Act::OpenDoc(rule.doc.clone()));
            }
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
    ui.label(ptitle(texts, rule, name));
    match param {
        Param::Enum { values, .. } => {
            let shown = values
                .iter()
                .find(|v| check::same_value(&v.value, &current))
                .map_or_else(|| current.to_string(), |v| v.title.clone());
            egui::ComboBox::from_id_salt(format!("{}.{name}", rule.id))
                .selected_text(shown)
                .width((ui.available_width() - 120.0).max(160.0))
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
                        .desired_rows(6)
                        .desired_width((ui.available_width() - 24.0).clamp(360.0, 760.0))
                        .hint_text(hint),
                )
                .changed()
            {
                let items: Vec<Value> = if text.is_empty() {
                    Vec::new()
                } else {
                    text.split('\n')
                        .map(|l| Value::from(l.trim_end_matches('\r')))
                        .collect()
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
                        .desired_width((ui.available_width() - 24.0).clamp(240.0, 520.0)),
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
    ui.label(style::muted(
        ui,
        trf("default: {value}", &[("value", &param.default_value())]),
        11.0,
    ));
}

pub fn detail_view(
    ui: &mut egui::Ui,
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    imports: &[CatalogFile],
    doc: &ProfileDoc,
    settings: &Settings,
    acts: &mut Vec<Act>,
) {
    egui::ScrollArea::vertical()
        .auto_shrink(false)
        .show(ui, |ui| match &doc.sel {
            Sel::None => profile_detail(ui, doc),
            Sel::Group(id) if id == UNKNOWN_GROUP => unknown_group_detail(ui, doc, acts),
            Sel::Group(id) => group_detail(ui, catalog, texts, doc, id, acts),
            Sel::Unknown(id) => unknown_detail(ui, doc, id, acts),
            Sel::Rule(id) => rule_detail(ui, catalog, texts, imports, doc, settings, id, acts),
            Sel::Form(form) => {
                egui::Frame::new()
                    .inner_margin(Margin::symmetric(18, 12))
                    .show(ui, |ui| {
                        crate::forms::form_view(ui, doc, *form, &crate::i18n::current(), acts);
                    });
            }
        });
}
