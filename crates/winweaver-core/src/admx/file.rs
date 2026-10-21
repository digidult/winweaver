//! A catalog file (`.wwcatalog`): the rules and groups of one import, read, checked, changed
//! and written.

use std::collections::HashSet;
use std::path::Path;

use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::catalog::{Action, Group, Level, Param, Phase, ROOT_POLICIES, RegKind, Rule, check};
use crate::jsonfile;

use super::rules::{cut, safe_key, safe_text, slug};
use super::{CATALOG_FORMAT, CATALOG_MAX, CATALOG_VERSION};

/// The texts of an import in one language.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LangTexts {
    #[serde(default)]
    pub groups: IndexMap<String, String>,
    /// Titles and summaries of the rules.
    #[serde(default)]
    pub rules: IndexMap<String, (String, String)>,
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
        let file: Self = jsonfile::read(path, CATALOG_MAX).map_err(|e| e.to_string())?;
        file.check()?;
        Ok(file)
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        let text = serde_json::to_string(self).map_err(|e| e.to_string())?;
        jsonfile::write_atomic(path, &text).map_err(|e| e.to_string())
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

    /// The checks of a catalog file: only groups and rules of imports, only registry values in
    /// the machine or default user branch, safe texts, placeholders only for the parameters.
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
                Some(p) if p == ROOT_POLICIES || groups.contains(p) => {}
                _ => return Err(format!("the group {} has no parent in the file", g.id)),
            }
        }
        let ids: HashSet<&str> = self.rules.iter().map(|r| r.id.as_str()).collect();
        for r in &self.rules {
            if !r.id.starts_with("admx.") || !check::is_safe_id(&r.id) {
                return Err(format!("the rule id {} is not an import id", r.id));
            }
            if !groups.contains(r.group.as_str()) {
                return Err(format!("the rule {} has no group in the file", r.id));
            }
            if !r.requires.is_empty() || r.conflicts.iter().any(|c| !ids.contains(c.as_str())) {
                return Err(format!(
                    "the rule {} refers to rules outside the file",
                    r.id
                ));
            }
            if !matches!(r.phase, Phase::Specialize | Phase::DefaultUser) {
                return Err(format!(
                    "the rule {} has a phase an import cannot have",
                    r.id
                ));
            }
            for a in &r.actions {
                let (path, name) = match a {
                    Action::Reg {
                        path, name, value, ..
                    } => {
                        let placeholders: Vec<&str> =
                            value.as_str().map(check::placeholders).unwrap_or_default();
                        match value {
                            Value::String(s) if !placeholders.is_empty() => {
                                if placeholders.len() != 1
                                    || s != &format!("{{{}}}", placeholders[0])
                                    || !r.params.contains_key(placeholders[0])
                                {
                                    return Err(format!(
                                        "the rule {} has a value with a placeholder of its own",
                                        r.id
                                    ));
                                }
                            }
                            Value::String(s) if !safe_text(s, 16383) || s.contains('{') => {
                                return Err(format!("the rule {} writes an unsafe text", r.id));
                            }
                            Value::String(_) | Value::Number(_) => {}
                            _ => {
                                return Err(format!(
                                    "the rule {} writes a value of an unknown shape",
                                    r.id
                                ));
                            }
                        }
                        (path, name)
                    }
                    Action::RegRemove { path, name, .. } => (path, name),
                    Action::RegList {
                        path,
                        param,
                        kind,
                        prefix,
                        ..
                    } => {
                        let list = param.is_empty()
                            || matches!(r.params.get(param), Some(Param::List { .. }));
                        if !list
                            || !matches!(kind, RegKind::String | RegKind::ExpandString)
                            || prefix.contains('{')
                        {
                            return Err(format!(
                                "the rule {} has a list of an unknown shape",
                                r.id
                            ));
                        }
                        (path, prefix)
                    }
                    _ => {
                        return Err(format!(
                            "the rule {} has an action an import cannot have",
                            r.id
                        ));
                    }
                };
                let key = path
                    .strip_prefix("HKLM:\\")
                    .or_else(|| path.strip_prefix("DU:\\"));
                if !key.is_some_and(safe_key) || !safe_text(name, 16383) {
                    return Err(format!(
                        "the rule {} writes an unsafe registry key or value name",
                        r.id
                    ));
                }
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
                if texts
                    .iter()
                    .any(|t| !safe_text(t, 16383) || t.contains('{'))
                {
                    return Err(format!("the rule {} has an unsafe parameter text", r.id));
                }
            }
        }
        Ok(())
    }
}
