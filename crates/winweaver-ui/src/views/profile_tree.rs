//! The tree of a profile: its rows (groups, rules, the forms, the unknown options) and the view that paints them.

use super::*;
use crate::ShellAct;

/// The key of a row of the profile tree: what it selects and, for a rule, the group it is drawn
/// under, since a policy of two imports shows in both trees.
pub(crate) type Key = (Sel, Option<String>);

/// The rows of the profile tree under a group, in the order they are shown.
#[allow(clippy::too_many_arguments)]
pub(crate) fn tree_rows(
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
pub(crate) fn unknown_on(choice: &Value) -> bool {
    choice.get("on").and_then(Value::as_bool).unwrap_or(false)
}

/// The group of the options of the profile whose catalog is not loaded, at the end of the tree:
/// their ids, not to be switched.
pub(crate) fn unknown_rows(rows: &mut Vec<tree::Row<Key>>, doc: &ProfileDoc, words: &[String]) {
    let unknown = &doc.profile.unknown;
    let shown: Vec<(&String, &Value)> = unknown
        .iter()
        .filter(|(id, _)| matches_words(id, words))
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
    let scroll = tree::Scroll::wanted(doc.reveal, doc.follow);
    if selected.is_none() && scroll != tree::Scroll::None {
        acts.push(Act::Shell(ShellAct::Revealed));
    }
    let id = egui::Id::new(("profile-tree", doc.draft_key.as_str()));
    let out = tree::show(ui, id, &rows, selected, scroll);
    for event in out.events {
        match event {
            tree::Event::Select((sel, group)) => {
                acts.push(Act::Profile(ProfileAct::SelectIn(sel, group)))
            }
            tree::Event::Expand((Sel::Group(g), _), open) => {
                acts.push(Act::Profile(ProfileAct::Expand(g, open)))
            }
            tree::Event::Toggle((Sel::Rule(id), _)) => {
                let on = !doc.profile.is_on(&id);
                acts.push(Act::Profile(ProfileAct::Toggle(id, on)));
            }
            tree::Event::Switch((Sel::Rule(id), _), on) => {
                acts.push(Act::Profile(ProfileAct::Toggle(id, on)))
            }
            tree::Event::Toggle((Sel::Group(g), _)) => {
                let rules = catalog.rules_under(&g);
                let all = !rules.is_empty() && rules.iter().all(|r| doc.resolution.shown_on(&r.id));
                acts.push(Act::Profile(ProfileAct::SetGroup(g, !all)));
            }
            _ => {}
        }
    }
    if out.moved {
        acts.push(Act::Shell(ShellAct::Follow));
    }
    if out.scrolled {
        acts.push(Act::Shell(ShellAct::Revealed));
    }
}

/// The built-in rule that writes the values of an imported policy chosen off, when one does.
pub(crate) fn linked_rule<'a>(doc: &'a ProfileDoc, rule: &Rule) -> Option<&'a str> {
    doc.resolution
        .linked_by(&rule.id)
        .filter(|_| !doc.profile.is_on(&rule.id))
}

/// The check box of a rule. A linked policy (a built-in rule in effect writes its values) is drawn
/// checked and cannot be changed here; its tip names the built-in rule.
pub(crate) fn rule_checkbox(
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
        acts.push(Act::Profile(ProfileAct::Toggle(rule.id.clone(), on)));
    }
}
