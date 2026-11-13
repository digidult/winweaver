//! A catalog file (`.wwcatalog`): the rules and groups of one import, read, checked, changed
//! and written.

use std::collections::HashSet;
use std::path::Path;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::catalog::{
    Action, Catalog, Group, Level, Param, Phase, ROOT_POLICIES, Rule, TREE_ROOTS, check, embedded,
};
use crate::i18n::{CatalogTexts, GroupText, RuleText};
use crate::jsonfile;

use super::rules::{cut, safe_key, safe_text, slug};
use super::{CATALOG_FORMAT, CATALOG_MAX, CATALOG_VERSION};

/// The texts of a catalog file in one language, in the form of the translation files of the
/// catalog: the texts of its groups and of its rules. A file of version 1 held a group as its
/// title and a rule as its title and summary; such a file is still read.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LangTexts {
    #[serde(default, deserialize_with = "groups_of")]
    pub groups: IndexMap<String, GroupText>,
    #[serde(default, deserialize_with = "rules_of")]
    pub rules: IndexMap<String, RuleText>,
}

fn groups_of<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<IndexMap<String, GroupText>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Either {
        Title(String),
        Text(GroupText),
    }
    let raw: IndexMap<String, Either> = IndexMap::deserialize(d)?;
    Ok(raw
        .into_iter()
        .map(|(id, e)| {
            let text = match e {
                Either::Title(title) => GroupText {
                    title: Some(title),
                    summary: None,
                },
                Either::Text(t) => t,
            };
            (id, text)
        })
        .collect())
}

fn rules_of<'de, D: serde::Deserializer<'de>>(
    d: D,
) -> Result<IndexMap<String, RuleText>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Either {
        Pair(String, String),
        Text(Box<RuleText>),
    }
    let raw: IndexMap<String, Either> = IndexMap::deserialize(d)?;
    Ok(raw
        .into_iter()
        .map(|(id, e)| {
            let text = match e {
                Either::Pair(title, summary) => RuleText {
                    title: Some(title),
                    summary: (!summary.is_empty()).then_some(summary),
                    ..RuleText::default()
                },
                Either::Text(t) => *t,
            };
            (id, text)
        })
        .collect())
}

/// The forms of a registry key a catalog writes: the machine, the default user profile, the user
/// of the first sign-in and the users.
const KEY_ROOTS: [&str; 4] = ["HKLM:\\", "DU:\\", "HKCU:\\", "HKU:\\"];

/// What `CatalogFile::copy_from` takes of another catalog file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyWhat<'a> {
    /// A policy (or its Disabled rule) with its Disabled rule `<id>.off`.
    Policy(&'a str),
    /// A group with the groups under it and their policies.
    Group(&'a str),
}

/// A catalog file: the rules and groups of one import of policy templates.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CatalogFile {
    pub format: String,
    pub version: u32,
    pub kind: String,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub windows: String,
    #[serde(default)]
    pub created: String,
    pub groups: Vec<Group>,
    pub rules: Vec<Rule>,
    /// Texts in other languages than English, by language code.
    #[serde(default)]
    pub texts: IndexMap<String, LangTexts>,
    /// Policies the import left out, and why.
    #[serde(default)]
    pub skipped: Vec<(String, String)>,
}

impl CatalogFile {
    /// Reads a catalog file (plain JSON) and checks every record.
    pub fn load(path: &Path) -> Result<Self, String> {
        let mut file: Self = jsonfile::read(path, CATALOG_MAX).map_err(|e| e.to_string())?;
        file.check()?;
        file.version = CATALOG_VERSION;
        Ok(file)
    }

    /// Reads the text of a catalog file under its name and checks every record, as `load`.
    pub fn parse(text: &str, name: &str) -> Result<Self, String> {
        if text.len() as u64 > CATALOG_MAX {
            return Err(format!("{name}: too large"));
        }
        let mut file: Self = jsonfile::parse(text, name).map_err(|e| e.to_string())?;
        file.check()?;
        file.version = CATALOG_VERSION;
        Ok(file)
    }

    /// The text of the file, as `save` writes it.
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        jsonfile::write_atomic(path, &self.to_json()).map_err(|e| e.to_string())
    }

    /// A new policy of the file, empty, in a group of it: the id `admx.<file>.<name>`, the phase of
    /// the machine, the level optional, as the editor and the MCP server start one.
    pub fn new_policy(&self, group: &str, name: &str) -> Result<Rule, String> {
        let id = format!("admx.{}.{}", slug(&self.id), slug(name));
        if self.rules.iter().any(|r| r.id == id) {
            return Err(format!("{id} is there already"));
        }
        if !self.groups.iter().any(|g| g.id == group) {
            return Err(format!("the file has no group {group}"));
        }
        Ok(Rule {
            id,
            group: group.to_owned(),
            phase: Phase::Specialize,
            title: cut(name.trim(), 300),
            level: Level::Optional,
            default: false,
            tags: Vec::new(),
            doc: String::new(),
            summary: String::new(),
            effect: String::new(),
            risk: String::new(),
            versions: String::new(),
            rollback: String::new(),
            verify: String::new(),
            note: String::new(),
            params: IndexMap::new(),
            requires: Vec::new(),
            conflicts: Vec::new(),
            actions: Vec::new(),
        })
    }

    /// Puts a policy into the file, a new one or in place of the one of its id, and checks the
    /// whole file; when the check fails the file stays as it was.
    pub fn put_rule(&mut self, rule: Rule) -> Result<(), String> {
        let mut next = self.clone();
        match next.rules.iter_mut().find(|r| r.id == rule.id) {
            Some(r) => *r = rule,
            None => next.rules.push(rule),
        }
        next.check()?;
        *self = next;
        Ok(())
    }

    /// Puts a policy into the file with its translations by language code (they replace the ones
    /// it had), as `put_rule`: the whole file is checked and a failure changes nothing.
    pub fn put_rule_with_texts(
        &mut self,
        rule: Rule,
        texts: IndexMap<String, RuleText>,
    ) -> Result<(), String> {
        let mut next = self.clone();
        next.set_rule_texts(&rule.id, texts);
        next.put_rule(rule)?;
        *self = next;
        Ok(())
    }

    /// Puts a group in place of the group of its id, with its translations by language code; the
    /// whole file is checked and a failure changes nothing.
    pub fn put_group(
        &mut self,
        group: Group,
        texts: IndexMap<String, GroupText>,
    ) -> Result<(), String> {
        let mut next = self.clone();
        let id = group.id.clone();
        match next.groups.iter_mut().find(|g| g.id == id) {
            Some(g) => *g = group,
            None => return Err(format!("the file has no group {id}")),
        }
        next.set_group_texts(&id, texts);
        next.check()?;
        *self = next;
        Ok(())
    }

    /// Takes a policy out of the file with its Disabled rule `<id>.off`, and the conflicts that
    /// name them.
    pub fn remove_rule(&mut self, id: &str) -> Result<(), String> {
        let base = id.strip_suffix(".off").unwrap_or(id).to_owned();
        let off = format!("{base}.off");
        let before = self.rules.len();
        self.rules.retain(|r| r.id != base && r.id != off);
        if self.rules.len() == before {
            return Err(format!("the file has no policy {id}"));
        }
        for r in &mut self.rules {
            r.conflicts.retain(|c| c != &base && c != &off);
        }
        Ok(())
    }

    /// Takes a group out of the file with every group under it, their policies, the conflicts
    /// that name those policies and the texts of all of them in other languages. The top group
    /// of the file (the one under the root of the policies) stays, since the file is the import;
    /// the count of the rules removed.
    pub fn remove_group(&mut self, id: &str) -> Result<usize, String> {
        let Some(group) = self.groups.iter().find(|g| g.id == id) else {
            return Err(format!("the file has no group {id}"));
        };
        if group.parent.as_deref() == Some(ROOT_POLICIES) || group.parent.is_none() {
            return Err("the top group of the file stays: remove the file instead".to_owned());
        }
        let mut gone: Vec<String> = vec![id.to_owned()];
        let mut i = 0;
        while i < gone.len() {
            let parent = gone[i].clone();
            gone.extend(
                self.groups
                    .iter()
                    .filter(|g| g.parent.as_deref() == Some(parent.as_str()))
                    .map(|g| g.id.clone()),
            );
            i += 1;
        }
        let rules: Vec<String> = self
            .rules
            .iter()
            .filter(|r| gone.contains(&r.group))
            .map(|r| r.id.clone())
            .collect();
        self.groups.retain(|g| !gone.contains(&g.id));
        self.rules.retain(|r| !rules.contains(&r.id));
        for r in &mut self.rules {
            r.conflicts.retain(|c| !rules.contains(c));
        }
        for texts in self.texts.values_mut() {
            texts.groups.retain(|g, _| !gone.contains(g));
            texts.rules.retain(|r, _| !rules.contains(r));
        }
        Ok(rules.len())
    }

    /// Copies a policy or a group of another catalog file into the group `into` of this one,
    /// so a catalog of only the policies needed can be made and passed on. A policy keeps its
    /// id (the same policy in two catalogs is one rule of the program, an alias in the other
    /// tree) and replaces a copy made before; a group is made again under `into` by its title,
    /// the groups under it too (a group of the same title there takes them), and the copied
    /// rules keep only the conflicts with rules of this file. Their texts in other languages
    /// come along. The whole file is checked and a failure changes nothing; the count of the
    /// policies copied.
    pub fn copy_from(
        &mut self,
        source: &CatalogFile,
        what: CopyWhat<'_>,
        into: &str,
    ) -> Result<usize, String> {
        let mut next = self.clone();
        if !next.groups.iter().any(|g| g.id == into) {
            return Err(format!("the file has no group {into}"));
        }
        // The groups of the source made again, each with the group of this file it became.
        let mut groups: Vec<(String, String)> = Vec::new();
        let rules: Vec<&Rule> = match what {
            CopyWhat::Policy(id) => {
                let base = id.strip_suffix(".off").unwrap_or(id);
                let off = format!("{base}.off");
                let found: Vec<&Rule> = source
                    .rules
                    .iter()
                    .filter(|r| r.id == base || r.id == off)
                    .collect();
                if found.is_empty() {
                    return Err(format!("the catalog has no policy {id}"));
                }
                found
            }
            CopyWhat::Group(gid) => {
                let Some(top) = source.groups.iter().find(|g| g.id == gid) else {
                    return Err(format!("the catalog has no group {gid}"));
                };
                let mut queue = vec![(top, into.to_owned())];
                while let Some((group, parent)) = queue.pop() {
                    let same = next.groups.iter().find(|x| {
                        x.parent.as_deref() == Some(parent.as_str()) && x.title == group.title
                    });
                    let made = match same {
                        Some(x) => x.id.clone(),
                        None => next.new_group(&parent, &group.title)?,
                    };
                    if let Some(g) = next.groups.iter_mut().find(|x| x.id == made) {
                        g.summary.clone_from(&group.summary);
                    }
                    for child in source
                        .groups
                        .iter()
                        .filter(|c| c.parent.as_deref() == Some(group.id.as_str()))
                    {
                        queue.push((child, made.clone()));
                    }
                    groups.push((group.id.clone(), made));
                }
                source
                    .rules
                    .iter()
                    .filter(|r| groups.iter().any(|(s, _)| *s == r.group))
                    .collect()
            }
        };
        let mut count = 0;
        for rule in rules {
            let mut copy = rule.clone();
            copy.group = groups
                .iter()
                .find(|(s, _)| *s == rule.group)
                .map_or_else(|| into.to_owned(), |(_, t)| t.clone());
            if !copy.id.ends_with(".off") {
                count += 1;
            }
            match next.rules.iter_mut().find(|r| r.id == copy.id) {
                Some(r) => *r = copy,
                None => next.rules.push(copy),
            }
            for (lang, texts) in &source.texts {
                if let Some(t) = texts.rules.get(&rule.id) {
                    let mine = next.texts.entry(lang.clone()).or_default();
                    mine.rules.insert(rule.id.clone(), t.clone());
                }
            }
        }
        for (from, to) in &groups {
            for (lang, texts) in &source.texts {
                if let Some(title) = texts.groups.get(from) {
                    let mine = next.texts.entry(lang.clone()).or_default();
                    mine.groups.insert(to.clone(), title.clone());
                }
            }
        }
        let ids: HashSet<String> = next.rules.iter().map(|r| r.id.clone()).collect();
        for rule in &mut next.rules {
            rule.conflicts.retain(|c| ids.contains(c));
        }
        next.check()?;
        *self = next;
        Ok(count)
    }

    /// A new group of the file under one of its groups; its id.
    pub fn new_group(&mut self, parent: &str, title: &str) -> Result<String, String> {
        let title = title.trim();
        if title.is_empty() || !safe_text(title, 200) {
            return Err("a title of 1 to 200 safe characters is needed".to_owned());
        }
        if !self.groups.iter().any(|g| g.id == parent) {
            return Err(format!("the file has no group {parent}"));
        }
        let base = format!("{parent}.{}", slug(title));
        let mut id = base.clone();
        let mut n = 2;
        while self.groups.iter().any(|g| g.id == id) {
            id = format!("{base}-{n}");
            n += 1;
        }
        let order = 1000 + self.groups.len() as u32;
        self.groups.push(Group {
            id: id.clone(),
            title: title.to_owned(),
            order,
            summary: String::new(),
            parent: Some(parent.to_owned()),
        });
        Ok(id)
    }

    /// The checks of a catalog file: the ids of an import, every group under a root of the tree or
    /// a group of the file, the references to rules of the file or of the built-in catalog, the
    /// checks of the built-in catalog for every rule (its mechanism and its root, its parameters,
    /// its placeholders, never a parameter in code), safe texts for the registry and the answer
    /// file and keys of the forms a catalog writes. A rule without actions yet (a policy the
    /// editor just made) is only checked for its place.
    pub fn check(&self) -> Result<(), String> {
        if self.format != CATALOG_FORMAT || self.version > CATALOG_VERSION || self.kind != "admx" {
            return Err("not a catalog file of this version".to_owned());
        }
        let groups: HashSet<&str> = self.groups.iter().map(|g| g.id.as_str()).collect();
        for g in &self.groups {
            if !g.id.starts_with("admx.") || !check::is_safe_id(&g.id) {
                return Err(format!("the group id {} is not an import id", g.id));
            }
            match g.parent.as_deref() {
                Some(p) if TREE_ROOTS.contains(&p) || groups.contains(p) => {}
                _ => return Err(format!("the group {} has no parent in the file", g.id)),
            }
            // The chain of parents reaches a root of the tree: a cycle would hang the tree.
            let mut next = g.parent.as_deref();
            let mut steps = 0;
            while let Some(p) = next.filter(|p| !TREE_ROOTS.contains(p)) {
                steps += 1;
                if steps > self.groups.len() {
                    return Err(format!("the group {} is inside itself", g.id));
                }
                next = self
                    .groups
                    .iter()
                    .find(|x| x.id == p)
                    .and_then(|x| x.parent.as_deref());
            }
        }
        let builtin = embedded::cached();
        let known = |id: &str| builtin.is_some_and(|c| c.rule(id).is_some());
        let ids: HashSet<&str> = self.rules.iter().map(|r| r.id.as_str()).collect();
        for r in &self.rules {
            if !r.id.starts_with("admx.") || !check::is_safe_id(&r.id) {
                return Err(format!("the rule id {} is not an import id", r.id));
            }
            if !groups.contains(r.group.as_str()) {
                return Err(format!("the rule {} has no group in the file", r.id));
            }
            if let Some(other) = r
                .requires
                .iter()
                .chain(&r.conflicts)
                .find(|c| !ids.contains(c.as_str()) && !known(c))
            {
                return Err(format!(
                    "the rule {} refers to the unknown rule {other}",
                    r.id
                ));
            }
            for a in &r.actions {
                check_action(r, a)?;
            }
            for p in r.params.values() {
                let texts: Vec<&str> = match p {
                    Param::Enum { values, .. } => {
                        values.iter().filter_map(|v| v.value.as_str()).collect()
                    }
                    Param::Text { default, .. } => vec![default.as_str()],
                    Param::List { default, .. } => default.iter().map(String::as_str).collect(),
                    Param::Int { .. } => Vec::new(),
                };
                if texts.iter().any(|t| !safe_text(t, 16383)) {
                    return Err(format!("the rule {} has an unsafe parameter text", r.id));
                }
            }
        }
        for (code, lang) in &self.texts {
            if code.len() > 16 || !check::is_safe_id(code) {
                return Err(format!("{code} is not a language code"));
            }
            let groups = lang
                .groups
                .values()
                .flat_map(|g| [&g.title, &g.summary])
                .filter_map(|t| t.as_deref());
            let mut texts = groups.chain(lang.rules.values().flat_map(RuleText::all_texts));
            if texts.any(|t| !readable(t)) {
                return Err(format!("a text in {code} holds a control character"));
            }
        }
        let catalog = self.as_catalog()?;
        let written = self.rules.iter().filter(|r| !r.actions.is_empty());
        match check::check_rules(&catalog, written).into_iter().next() {
            Some(problem) => Err(problem),
            None => Ok(()),
        }
    }

    /// The file as a catalog of its own: the roots of the tree, its groups and its rules, whose
    /// references to the built-in catalog are left out. What the checks of the built-in catalog
    /// read.
    fn as_catalog(&self) -> Result<Catalog, String> {
        let ids: HashSet<&str> = self.rules.iter().map(|r| r.id.as_str()).collect();
        let mut groups: Vec<Group> = match embedded::cached() {
            Some(c) => c
                .groups()
                .iter()
                .filter(|g| TREE_ROOTS.contains(&g.id.as_str()))
                .cloned()
                .collect(),
            None => Vec::new(),
        };
        groups.extend(self.groups.iter().cloned());
        let rules = self
            .rules
            .iter()
            .map(|r| {
                let mut r = r.clone();
                r.requires.retain(|c| ids.contains(c.as_str()));
                r.conflicts.retain(|c| ids.contains(c.as_str()));
                r
            })
            .collect();
        Catalog::from_parts("file", groups, rules).map_err(|e| e.to_string())
    }

    /// The rules of the file that run code of their own during the installation or on this PC:
    /// scripts, commands and programs, and settings of the answer file, which can run commands
    /// too. An import of such a file asks first, and an MCP client never writes them.
    pub fn code_rules(&self) -> Vec<&Rule> {
        self.rules.iter().filter(|r| r.runs_code()).collect()
    }

    /// Sets the translations of a rule of the file by language code; a language without a text
    /// for the rule leaves it out, and a language without any text leaves the file.
    pub fn set_rule_texts(&mut self, id: &str, texts: IndexMap<String, RuleText>) {
        for lang in self.texts.values_mut() {
            lang.rules.shift_remove(id);
        }
        for (code, text) in texts.into_iter().filter(|(_, t)| !t.is_empty()) {
            let lang = self.texts.entry(code).or_default();
            lang.rules.insert(id.to_owned(), text);
        }
        self.texts
            .retain(|_, l| !l.groups.is_empty() || !l.rules.is_empty());
    }

    /// Sets the translations of a group of the file by language code, as `set_rule_texts`.
    pub fn set_group_texts(&mut self, id: &str, texts: IndexMap<String, GroupText>) {
        for lang in self.texts.values_mut() {
            lang.groups.shift_remove(id);
        }
        let empty = GroupText::default();
        for (code, text) in texts.into_iter().filter(|(_, t)| *t != empty) {
            let lang = self.texts.entry(code).or_default();
            lang.groups.insert(id.to_owned(), text);
        }
        self.texts
            .retain(|_, l| !l.groups.is_empty() || !l.rules.is_empty());
    }

    /// The translation of the file into a language, in the form of the translations of the
    /// catalog; `None` for a language the file has no texts in.
    pub fn catalog_texts(&self, code: &str) -> Option<CatalogTexts> {
        let lang = self.texts.get(code)?;
        Some(CatalogTexts {
            code: code.to_owned(),
            groups: lang.groups.clone(),
            rules: lang.rules.clone(),
            ..CatalogTexts::default()
        })
    }
}

/// The checks of an action of a catalog file beyond the ones of the built-in catalog: the forms
/// of its registry keys and safe texts where the registry, a script or the answer file takes
/// them.
fn check_action(r: &Rule, a: &Action) -> Result<(), String> {
    let unsafe_text = || Err(format!("the rule {} writes an unsafe text", r.id));
    let texts: Vec<&str> = match a {
        Action::Reg {
            path, name, value, ..
        } => {
            let mut texts = vec![name.as_str()];
            match value {
                Value::Number(_) => {}
                Value::String(s) => texts.push(s),
                Value::Array(items) if items.iter().all(Value::is_string) => {
                    texts.extend(items.iter().filter_map(Value::as_str));
                }
                // The bytes of a binary value.
                Value::Array(items)
                    if items.iter().all(|v| v.as_u64().is_some_and(|b| b < 256)) => {}
                _ => {
                    return Err(format!(
                        "the rule {} writes a value of an unknown shape",
                        r.id
                    ));
                }
            }
            check_key(r, path)?;
            texts
        }
        Action::RegRemove { path, name, .. } => {
            check_key(r, path)?;
            vec![name.as_str()]
        }
        Action::RegList { path, prefix, .. } => {
            check_key(r, path)?;
            vec![prefix.as_str()]
        }
        Action::XmlSetting {
            pass,
            component,
            path,
            value,
        } => vec![pass.as_str(), component, path, value],
        Action::XmlOobe { element, value } => vec![element.as_str(), value],
        Action::XmlPeCommand {
            command,
            description,
        }
        | Action::XmlSpecializeCommand {
            command,
            description,
        } => vec![command.as_str(), description],
        Action::Service { name, .. } | Action::Feature { name, .. } => vec![name.as_str()],
        Action::Exe { file, args } => std::iter::once(file.as_str())
            .chain(args.iter().map(String::as_str))
            .collect(),
        Action::Capability { pattern } => vec![pattern.as_str()],
        Action::Appx { names } => names.iter().map(String::as_str).collect(),
        // The lines of a script may hold tabs; a line break inside a line is refused.
        Action::Ps { script } => {
            if script
                .lines()
                .iter()
                .any(|l| !safe_text(&l.replace('\t', " "), 16383))
            {
                return unsafe_text();
            }
            Vec::new()
        }
    };
    if texts.iter().any(|t| !safe_text(t, 16383)) {
        return unsafe_text();
    }
    Ok(())
}

/// A text of a translation: within the limit, no control character but a line break and a tab.
fn readable(text: &str) -> bool {
    text.chars().count() <= 16383
        && !text
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
}

/// A registry key of a catalog: one of its forms (`HKLM:\`, `DU:\`, `HKCU:\`, `HKU:\`) and a
/// safe key after it.
fn check_key(r: &Rule, path: &str) -> Result<(), String> {
    let key = KEY_ROOTS.iter().find_map(|root| path.strip_prefix(root));
    if key.is_some_and(safe_key) {
        Ok(())
    } else {
        Err(format!(
            "the rule {} writes an unsafe registry key or value name",
            r.id
        ))
    }
}
