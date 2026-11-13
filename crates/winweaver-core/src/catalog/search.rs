//! The full search of the catalog: the text a search looks through for a rule holds every field of
//! the rule in English and in the language of a translation (its texts, search words, parameters
//! and the titles of their values), the titles of the groups above it, and what its actions name:
//! registry keys in the forms of a catalog and of regedit (`HKLM:\X`, `HKEY_LOCAL_MACHINE\X`,
//! `HKLM\X`), value names, values (a number in decimal and hex too), types, the settings, elements
//! and commands of the answer file, the lines of scripts, services, features, capabilities, apps
//! and programs. A search word matches when the text holds it, ignoring case.

use std::collections::HashMap;

use serde_json::Value;

use super::{Action, Catalog, Param, Rule};
use crate::i18n::{CatalogTexts, RuleText};
use crate::registry;

/// The texts of the rules of a catalog for a search in one language, in lower case, by rule id.
#[derive(Debug, Default)]
pub struct SearchIndex {
    texts: HashMap<String, String>,
}

impl SearchIndex {
    /// The index of a catalog with the translation of a language (`None` for English).
    pub fn new(catalog: &Catalog, texts: Option<&CatalogTexts>) -> Self {
        Self::of_rules(
            catalog.rules(),
            |rule| group_titles(catalog, texts, &rule.group),
            |rule| texts.and_then(|t| t.rules.get(&rule.id)),
        )
    }

    /// The index of some rules, with the titles of the groups above each and its translation.
    pub fn of_rules<'a, 't>(
        rules: impl IntoIterator<Item = &'a Rule>,
        groups: impl Fn(&Rule) -> Vec<String>,
        translation: impl Fn(&Rule) -> Option<&'t RuleText>,
    ) -> Self {
        Self {
            texts: rules
                .into_iter()
                .map(|r| (r.id.clone(), rule_text(r, translation(r), &groups(r))))
                .collect(),
        }
    }

    /// Whether the rule holds every word of a search (the words in lower case).
    pub fn matches(&self, id: &str, words: &[String]) -> bool {
        words.is_empty()
            || self
                .texts
                .get(id)
                .is_some_and(|t| words.iter().all(|w| t.contains(w.as_str())))
    }

    /// The text of a rule for a search, in lower case; empty for a rule the catalog has not.
    pub fn text(&self, id: &str) -> &str {
        self.texts.get(id).map_or("", String::as_str)
    }
}

/// The titles of a group and of the groups above it, in English and in the translation.
fn group_titles(catalog: &Catalog, texts: Option<&CatalogTexts>, group: &str) -> Vec<String> {
    let mut out = Vec::new();
    for id in catalog.path_of(group) {
        if let Some(g) = catalog.group(id) {
            out.push(g.title.clone());
            let own = texts.and_then(|t| t.groups.get(id));
            if let Some(t) = own.and_then(|t| t.title.as_deref()) {
                out.push(t.to_owned());
            }
        }
    }
    out
}

/// The text of a rule for a search, in lower case: its fields, the texts of its translation, the
/// titles of its groups and what its actions name.
pub fn rule_text(rule: &Rule, translation: Option<&RuleText>, groups: &[String]) -> String {
    let mut out = String::with_capacity(1024);
    let mut add = |text: &str| {
        if !text.is_empty() {
            out.push_str(text);
            out.push('\n');
        }
    };
    for text in [
        &rule.id,
        &rule.title,
        &rule.summary,
        &rule.effect,
        &rule.risk,
        &rule.versions,
        &rule.rollback,
        &rule.verify,
        &rule.note,
    ] {
        add(text);
    }
    for tag in &rule.tags {
        add(tag);
    }
    for g in groups {
        add(g);
    }
    for (name, param) in &rule.params {
        add(name);
        add(param.title());
        match param {
            Param::Enum { values, .. } => {
                for v in values {
                    add(&value_text(&v.value));
                    add(&v.title);
                }
            }
            Param::Text { default, .. } => add(default),
            Param::List { default, .. } => default.iter().for_each(|d| add(d)),
            Param::Int { .. } => {}
        }
    }
    for action in &rule.actions {
        action_texts(action, &mut add);
    }
    if let Some(t) = translation {
        for text in t.all_texts() {
            add(text);
        }
    }
    out.to_lowercase()
}

/// A value of JSON as a search finds it: a text as it is, a number in decimal and in hex, the
/// items of a list.
fn value_text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => match n.as_u64() {
            Some(u) => format!("{u} 0x{u:x}"),
            None => n.to_string(),
        },
        Value::Array(items) => items.iter().map(value_text).collect::<Vec<_>>().join("\n"),
        other => other.to_string(),
    }
}

/// A registry key of a catalog in its forms: as written (`HKLM:\X`), as regedit names it
/// (`HKEY_LOCAL_MACHINE\X`) and with the short name of its hive (`HKLM\X`).
fn key_forms(path: &str, add: &mut impl FnMut(&str)) {
    add(&registry::key_forms(path));
}

/// What an action names, for a search.
fn action_texts(action: &Action, add: &mut impl FnMut(&str)) {
    match action {
        Action::XmlPeCommand {
            command,
            description,
        }
        | Action::XmlSpecializeCommand {
            command,
            description,
        } => {
            add(command);
            add(description);
        }
        Action::XmlSetting {
            pass,
            component,
            path,
            value,
        } => {
            for t in [pass, component, path, value] {
                add(t);
            }
        }
        Action::XmlOobe { element, value } => {
            add(element);
            add(value);
        }
        Action::Ps { script } => script.lines().iter().for_each(|l| add(l)),
        Action::Reg {
            path,
            name,
            kind,
            value,
            why,
            ..
        } => {
            key_forms(path, add);
            add(name);
            add(kind.type_name());
            add(&value_text(value));
            add(why);
        }
        Action::RegRemove { path, name, .. } => {
            key_forms(path, add);
            add(name);
        }
        Action::RegList {
            path, prefix, kind, ..
        } => {
            key_forms(path, add);
            add(prefix);
            add(kind.type_name());
        }
        Action::Service { name, .. } => add(name),
        Action::Exe { file, args } => {
            add(file);
            args.iter().for_each(|a| add(a));
        }
        Action::Feature { name, state, .. } => {
            add(name);
            add(state);
        }
        Action::Capability { pattern } => add(pattern),
        Action::Appx { names } => names.iter().for_each(|n| add(n)),
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    /// A rule is found by its registry key in every form, its value name, its id and its titles in
    /// English and in the language of the index; the index is made once a language.
    #[test]
    fn a_rule_is_found_by_every_field_and_its_registry_keys() {
        let catalog = Catalog::embedded().unwrap();
        let texts = CatalogTexts::embedded().unwrap();
        let ru = texts.iter().find(|t| t.code == "ru").unwrap();
        let index = catalog.search_index(Some(ru));
        assert!(
            Arc::ptr_eq(&index, &catalog.search_index(Some(ru))),
            "made once"
        );
        let english = catalog.search_index(None);
        assert!(
            !Arc::ptr_eq(&index, &english),
            "another language, another index"
        );
        let index = catalog.search_index(Some(ru));
        let (rule, path, name) = catalog
            .rules()
            .iter()
            .find_map(|r| {
                r.actions.iter().find_map(|a| match a {
                    Action::Reg { path, name, .. } if path.starts_with("HKLM:\\") => {
                        Some((r, path.clone(), name.clone()))
                    }
                    _ => None,
                })
            })
            .unwrap();
        let rest = path.strip_prefix("HKLM:\\").unwrap().to_lowercase();
        let words = [
            format!("hklm\\{rest}"),
            format!("hkey_local_machine\\{rest}"),
            path.to_lowercase(),
            name.to_lowercase(),
            rule.id.clone(),
            rule.title.to_lowercase(),
        ];
        for word in words {
            assert!(
                index.matches(&rule.id, std::slice::from_ref(&word)),
                "{} by {word}",
                rule.id
            );
        }
        if let Some(title) = ru.rules.get(&rule.id).and_then(|t| t.title.as_deref()) {
            assert!(index.matches(&rule.id, &[title.to_lowercase()]));
        }
        assert!(!index.matches(&rule.id, &["no-such-word-anywhere".to_owned()]));
        // A value of a parameter and the title of a group.
        let with_values = catalog
            .rules()
            .iter()
            .find(|r| r.params.values().any(|p| matches!(p, Param::Enum { .. })))
            .unwrap();
        let Some(Param::Enum { values, .. }) = with_values
            .params
            .values()
            .find(|p| matches!(p, Param::Enum { .. }))
        else {
            unreachable!()
        };
        let word = values[0].title.to_lowercase();
        assert!(english.matches(&with_values.id, &[word]));
        let group = catalog
            .group(&with_values.group)
            .unwrap()
            .title
            .to_lowercase();
        assert!(english.matches(&with_values.id, &[group]));
    }
}
