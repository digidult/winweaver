//! The catalog files merged into the catalog and into its translations.

use std::collections::HashSet;

use crate::catalog::{Catalog, CatalogError, Group, Rule};
use crate::i18n::{CatalogTexts, RuleText};

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
            // A rule of the file may name rules of the file and of the built-in catalog.
            let mut r = r.clone();
            let named = |c: &String| own_rules.contains(c.as_str()) || base.rule(c).is_some();
            r.conflicts.retain(named);
            r.requires.retain(named);
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

/// Adds the texts of the catalog files to the translations of the catalog; a text the catalog has
/// already stays. The Disabled rule `<id>.off` of a policy without a text of its own is titled
/// `<title>: <word>` with the word of the translation file (`_disabled`); without the word it keeps
/// its English title.
pub fn merge_texts(texts: &mut [CatalogTexts], files: &[CatalogFile]) {
    for f in files {
        let ids: HashSet<&str> = f.rules.iter().map(|r| r.id.as_str()).collect();
        for (code, lang) in &f.texts {
            let Some(t) = texts.iter_mut().find(|t| &t.code == code) else {
                continue;
            };
            for (id, text) in &lang.groups {
                t.groups.entry(id.clone()).or_insert_with(|| text.clone());
            }
            for (id, text) in &lang.rules {
                t.rules.entry(id.clone()).or_insert_with(|| text.clone());
                let off = format!("{id}.off");
                let derived = !t.disabled.is_empty()
                    && ids.contains(off.as_str())
                    && !lang.rules.contains_key(&off);
                if let Some(title) = text.title.as_deref().filter(|_| derived) {
                    let title = format!("{title}: {}", t.disabled);
                    t.rules.entry(off).or_insert_with(|| RuleText {
                        title: Some(title),
                        summary: text.summary.clone(),
                        ..RuleText::default()
                    });
                }
            }
        }
    }
}
