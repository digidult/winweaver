//! The rule catalog: the group tree and the rules, loaded from the embedded data or a folder.

pub mod check;
pub mod embedded;
pub mod model;
pub mod values;

use std::collections::HashMap;
use std::path::Path;

pub use model::{
    Action, EnumValue, Group, GroupsFile, Level, Lines, ListNames, Param, Phase, Pick, RegKind,
    Rule, RulesFile,
};

use crate::jsonfile::{self, JsonError};

/// The root of the installation: the answer file settings and the scripts of the installation.
pub const ROOT_UNATTEND: &str = "unattend";
/// The root of the policies: every rule that only writes the registry, and every imported
/// policy template.
pub const ROOT_POLICIES: &str = "policies";
/// The root of everything else.
pub const ROOT_OTHER: &str = "other";
/// The three roots of the tree, in order.
pub const TREE_ROOTS: [&str; 3] = [ROOT_UNATTEND, ROOT_POLICIES, ROOT_OTHER];

/// The mechanism that carries a rule; it decides the roots the rule may live under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Mechanism {
    /// Only settings and commands of the answer file.
    Unattend,
    /// Only registry values, their removals and lists, whatever the key.
    Registry,
    /// Only steps of a script: of the installation, or of a running Windows.
    Script,
    /// Anything else, a mix included.
    Other,
}

impl Mechanism {
    /// The roots a rule of this mechanism may live under; a script goes under the installation
    /// or under the rest, as the catalog decides.
    pub fn roots(self) -> &'static [&'static str] {
        match self {
            Mechanism::Unattend => &[ROOT_UNATTEND],
            Mechanism::Registry => &[ROOT_POLICIES],
            Mechanism::Script => &[ROOT_UNATTEND, ROOT_OTHER],
            Mechanism::Other => &[ROOT_OTHER],
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CatalogError {
    #[error(transparent)]
    Json(#[from] JsonError),
    #[error("{0}")]
    Io(String),
    #[error("duplicate {kind} id {id}")]
    Duplicate { kind: &'static str, id: String },
    #[error("{what} of {owner} refers to the unknown {kind} {id}")]
    Unknown {
        what: &'static str,
        owner: String,
        kind: &'static str,
        id: String,
    },
    #[error("{0}")]
    Invalid(String),
}

/// The catalog: groups sorted by order, rules in catalog order, indexes by id.
#[derive(Debug, Clone)]
pub struct Catalog {
    version: String,
    groups: Vec<Group>,
    rules: Vec<Rule>,
    group_index: HashMap<String, usize>,
    rule_index: HashMap<String, usize>,
    /// Rules shown in a group of another tree too: (the group, the rule).
    aliases: Vec<(String, String)>,
}

impl Catalog {
    /// Builds a catalog and checks that every reference (parent, group, requires, conflicts)
    /// names something that exists and that no id repeats.
    pub fn from_parts(
        version: &str,
        mut groups: Vec<Group>,
        rules: Vec<Rule>,
    ) -> Result<Self, CatalogError> {
        groups.sort_by_key(|g| g.order);
        let mut group_index = HashMap::new();
        for (i, group) in groups.iter().enumerate() {
            if group_index.insert(group.id.clone(), i).is_some() {
                return Err(CatalogError::Duplicate {
                    kind: "group",
                    id: group.id.clone(),
                });
            }
        }
        let mut rule_index = HashMap::new();
        for (i, rule) in rules.iter().enumerate() {
            if rule_index.insert(rule.id.clone(), i).is_some() {
                return Err(CatalogError::Duplicate {
                    kind: "rule",
                    id: rule.id.clone(),
                });
            }
        }
        for group in &groups {
            if let Some(parent) = &group.parent
                && !group_index.contains_key(parent)
            {
                return Err(CatalogError::Unknown {
                    what: "parent",
                    owner: group.id.clone(),
                    kind: "group",
                    id: parent.clone(),
                });
            }
        }
        for rule in &rules {
            if !group_index.contains_key(&rule.group) {
                return Err(CatalogError::Unknown {
                    what: "group",
                    owner: rule.id.clone(),
                    kind: "group",
                    id: rule.group.clone(),
                });
            }
            for (what, list) in [("requires", &rule.requires), ("conflicts", &rule.conflicts)] {
                for other in list {
                    if !rule_index.contains_key(other) {
                        return Err(CatalogError::Unknown {
                            what,
                            owner: rule.id.clone(),
                            kind: "rule",
                            id: other.clone(),
                        });
                    }
                }
            }
        }
        let catalog = Self {
            version: version.to_owned(),
            groups,
            rules,
            group_index,
            rule_index,
            aliases: Vec::new(),
        };
        for root in TREE_ROOTS {
            if catalog.group(root).is_none() {
                return Err(CatalogError::Invalid(format!(
                    "the root group {root} is missing"
                )));
            }
        }
        for group in &catalog.groups {
            if catalog.root_of(&group.id).is_none() {
                return Err(CatalogError::Invalid(format!(
                    "group {}: a cycle of parents or no root",
                    group.id
                )));
            }
        }
        Ok(catalog)
    }

    /// The catalog embedded in the executable.
    pub fn embedded() -> Result<Self, CatalogError> {
        embedded::load()
    }

    /// A catalog read from a folder with `groups.json` and `NN-*.json` files.
    pub fn load_dir(dir: &Path) -> Result<Self, CatalogError> {
        let groups: GroupsFile =
            jsonfile::read(&dir.join(embedded::GROUPS_FILE), jsonfile::MAX_BYTES)?;
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .map_err(|e| CatalogError::Io(format!("{}: {e}", dir.display())))?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| {
                name.ends_with(".json") && name.as_bytes().first().is_some_and(u8::is_ascii_digit)
            })
            .collect();
        names.sort();
        let mut rules = Vec::new();
        for name in names {
            let file: RulesFile = jsonfile::read(&dir.join(&name), jsonfile::MAX_BYTES)?;
            rules.extend(file.rules);
        }
        Self::from_parts(embedded::CATALOG_VERSION, groups.groups, rules)
    }

    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn groups(&self) -> &[Group] {
        &self.groups
    }

    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }

    pub fn group(&self, id: &str) -> Option<&Group> {
        self.group_index.get(id).map(|&i| &self.groups[i])
    }

    pub fn rule(&self, id: &str) -> Option<&Rule> {
        self.rule_index.get(id).map(|&i| &self.rules[i])
    }

    /// The groups directly under `parent` (`None`: the roots), in order.
    pub fn child_groups(&self, parent: Option<&str>) -> Vec<&Group> {
        self.groups
            .iter()
            .filter(|g| g.parent.as_deref() == parent)
            .collect()
    }

    /// The rules whose group is `group`, in catalog order.
    pub fn rules_in(&self, group: &str) -> Vec<&Rule> {
        let mut out: Vec<&Rule> = self.rules.iter().filter(|r| r.group == group).collect();
        out.extend(
            self.aliases
                .iter()
                .filter(|(g, _)| g == group)
                .filter_map(|(_, r)| self.rule(r)),
        );
        out
    }

    /// The groups of other trees a rule shows in as an alias.
    pub fn alias_groups<'a>(&'a self, rule: &'a str) -> impl Iterator<Item = &'a str> + 'a {
        self.aliases
            .iter()
            .filter(move |(_, r)| r == rule)
            .map(|(g, _)| g.as_str())
    }

    /// The catalog with rules shown in other groups too: an imported policy two imports hold is
    /// one rule, owned by the first import, and an alias in the trees of the others. An alias in
    /// the group of the rule, or of something unknown, is left out.
    pub fn with_aliases(mut self, aliases: Vec<(String, String)>) -> Self {
        let mut seen = std::collections::HashSet::new();
        let kept: Vec<(String, String)> = aliases
            .into_iter()
            .filter(|(g, r)| {
                self.group_index.contains_key(g)
                    && self.rule(r).is_some_and(|rule| &rule.group != g)
                    && seen.insert((g.clone(), r.clone()))
            })
            .collect();
        self.aliases = kept;
        self
    }

    /// The rules of a group and of every group under it.
    pub fn rules_under(&self, group: &str) -> Vec<&Rule> {
        let mut out = self.rules_in(group);
        for child in self.child_groups(Some(group)) {
            out.extend(self.rules_under(&child.id));
        }
        out
    }

    /// The chain of groups from the root down to `group`, as ids.
    pub fn path_of(&self, group: &str) -> Vec<&str> {
        let mut chain = Vec::new();
        let mut current = self.group(group);
        let mut guard = 0;
        while let Some(g) = current {
            chain.push(g.id.as_str());
            guard += 1;
            if guard > self.groups.len() {
                return Vec::new();
            }
            current = g.parent.as_deref().and_then(|p| self.group(p));
        }
        chain.reverse();
        chain
    }

    /// The root group of `group`, or `None` for an unknown group or a cycle of parents.
    pub fn root_of(&self, group: &str) -> Option<&str> {
        self.path_of(group).first().copied()
    }

    /// The mechanism of a rule, read from its actions: only answer file settings and commands
    /// is `Unattend`; only registry values, removals and lists is `Registry`; only script steps
    /// is `Script`; anything else or a mix is `Other`.
    pub fn mechanism(rule: &Rule) -> Mechanism {
        let actions = &rule.actions;
        if actions.iter().all(Action::is_xml) {
            Mechanism::Unattend
        } else if actions.iter().all(|a| a.registry_path().is_some()) {
            Mechanism::Registry
        } else if actions.iter().all(|a| matches!(a, Action::Ps { .. })) {
            Mechanism::Script
        } else {
            Mechanism::Other
        }
    }
}
