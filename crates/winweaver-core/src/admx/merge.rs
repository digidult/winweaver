//! The catalog files merged into the catalog and into its translations.

use std::collections::HashSet;

use crate::catalog::{Catalog, CatalogError, Group, Rule};
use crate::i18n::{CatalogTexts, GroupText, RuleText};

use super::CatalogFile;

/// The catalog with the rules and groups of the catalog files added; a rule or group whose id is
/// already there is left out (the first file wins), and the notes say so.
pub fn merge(
    base: &Catalog,
    files: &[CatalogFile],
) -> Result<(Catalog, Vec<String>), CatalogError> {
    let mut groups: Vec<Group> = base.groups().to_vec();
    let mut rules: Vec<Rule> = base.rules().to_vec();
    let mut group_ids: HashSet<String> = groups.iter().map(|g| g.id.clone()).collect();
    let mut rule_ids: HashSet<String> = rules.iter().map(|r| r.id.clone()).collect();
    let mut notes = Vec::new();
    let mut aliases = Vec::new();
    for f in files {
        let mut skipped = 0;
        for g in &f.groups {
            if group_ids.insert(g.id.clone()) {
                groups.push(g.clone());
            }
        }
        let own_rules: HashSet<&str> = f
            .rules
            .iter()
            .map(|r| r.id.as_str())
            .filter(|id| !rule_ids.contains(*id))
            .collect();
        for r in &f.rules {
            if !own_rules.contains(r.id.as_str()) || !group_ids.contains(&r.group) {
                if rule_ids.contains(&r.id) && group_ids.contains(&r.group) {
                    aliases.push((r.group.clone(), r.id.clone()));
                }
                skipped += 1;
                continue;
            }
            let mut r = r.clone();
            r.conflicts.retain(|c| own_rules.contains(c.as_str()));
            rule_ids.insert(r.id.clone());
            rules.push(r);
        }
        if skipped > 0 {
            notes.push(format!(
                "{}: {skipped} rules belong to an import before it and show in both trees",
                f.name
            ));
        }
    }
    let catalog = Catalog::from_parts(base.version(), groups, rules)?.with_aliases(aliases);
    Ok((catalog, notes))
}

/// Adds the texts of the catalog files to the translations of the catalog. The Disabled rule
/// `<id>.off` of a policy is titled `<title>: <word>` with the word of the translation file
/// (`_disabled`); without the word it keeps its English title.
pub fn merge_texts(texts: &mut [CatalogTexts], files: &[CatalogFile]) {
    for f in files {
        let ids: HashSet<&str> = f.rules.iter().map(|r| r.id.as_str()).collect();
        for (code, lang) in &f.texts {
            let Some(t) = texts.iter_mut().find(|t| &t.code == code) else {
                continue;
            };
            for (id, title) in &lang.groups {
                t.groups.entry(id.clone()).or_insert_with(|| GroupText {
                    title: Some(title.clone()),
                    summary: None,
                });
            }
            for (id, (title, summary)) in &lang.rules {
                let summary = (!summary.is_empty()).then(|| summary.clone());
                t.rules.entry(id.clone()).or_insert_with(|| RuleText {
                    title: Some(title.clone()),
                    summary: summary.clone(),
                    ..RuleText::default()
                });
                let off = format!("{id}.off");
                if !t.disabled.is_empty() && ids.contains(off.as_str()) {
                    let title = format!("{title}: {}", t.disabled);
                    t.rules.entry(off).or_insert_with(|| RuleText {
                        title: Some(title),
                        summary,
                        ..RuleText::default()
                    });
                }
            }
        }
    }
}
