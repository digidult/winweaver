//! The files of an apply made from a profile, a selection of its rules or a diff, for the window
//! and the command line: the stand-alone scripts (apply, return to the defaults, audit, with the
//! undo script beside an apply or a return) and the registry files of an apply or a return. A
//! diff applies its other side: the rules it turns on are applied, the ones it turns off return to
//! the defaults of Windows.

use crate::apply;
use crate::catalog::Catalog;
use crate::diff::Diff;
use crate::profile::Profile;
use crate::regfile::{Line, RegFile};
use crate::scripts::{self, ScriptPlan};
use crate::thispc;

/// The stand-alone scripts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScriptKind {
    Apply,
    Revert,
    Audit,
}

impl ScriptKind {
    /// The kind of a word of the command line: apply, revert or audit.
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "apply" => Some(Self::Apply),
            "revert" => Some(Self::Revert),
            "audit" => Some(Self::Audit),
            _ => None,
        }
    }

    fn prefix(self) -> &'static str {
        match self {
            Self::Apply => "Apply",
            Self::Revert => "Revert",
            Self::Audit => "Audit",
        }
    }
}

/// A file to write: its name and its bytes.
#[derive(Debug, Clone)]
pub struct OutFile {
    pub name: String,
    pub bytes: Vec<u8>,
}

/// What a script does: the rules of an apply or a return, or the number of rules an audit reads.
#[derive(Debug, Clone)]
pub enum Done {
    Script(ScriptPlan),
    Audit(usize),
}

/// The files of a script: the script, and the undo script after an apply or a return.
#[derive(Debug, Clone)]
pub struct Scripts {
    pub files: Vec<OutFile>,
    pub done: Done,
}

/// A registry file of an apply or a return, its name and the number of its values.
#[derive(Debug, Clone)]
pub struct Reg {
    pub file: RegFile,
    pub name: String,
    pub values: usize,
}

/// A name of a file made of the name of a profile: letters, digits, spaces, hyphens and
/// underscores, every other character an underscore; `profile` when nothing is left.
pub fn file_stem(name: &str) -> String {
    let stem: String = name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let stem = stem.trim();
    if stem.is_empty() {
        "profile".to_owned()
    } else {
        stem.to_owned()
    }
}

/// Whether any of the rules can be applied to a running Windows: the passes of the installation,
/// the first sign-in and the sign-in screen cannot, so a selection of only those has no script
/// and no registry file.
pub fn applicable(catalog: &Catalog, ids: &[String]) -> bool {
    ids.iter()
        .filter_map(|id| catalog.rule(id))
        .any(|rule| thispc::exclusion(rule).is_none())
}

/// Every rule of the catalog: what a script of the whole profile covers (the rules off return to
/// the defaults of Windows).
pub fn all_rules(catalog: &Catalog) -> Vec<String> {
    catalog.rules().iter().map(|r| r.id.clone()).collect()
}

/// The rules of the ids of a command line: a rule by its id, a group by every rule under it.
pub fn rules_of(catalog: &Catalog, ids: &[String]) -> Result<Vec<String>, String> {
    let mut out: Vec<String> = Vec::new();
    for id in ids {
        let found: Vec<String> = if catalog.rule(id).is_some() {
            vec![id.clone()]
        } else {
            catalog
                .rules_under(id)
                .iter()
                .map(|r| r.id.clone())
                .collect()
        };
        if found.is_empty() {
            return Err(format!("{id} is neither a rule nor a group of the catalog"));
        }
        for rule in found {
            if !out.contains(&rule) {
                out.push(rule);
            }
        }
    }
    Ok(out)
}

/// The stand-alone script of `kind` for the rules `ids` of a profile, named after `stem`.
pub fn scripts(
    catalog: &Catalog,
    profile: &Profile,
    ids: &[String],
    kind: ScriptKind,
    stem: &str,
    version: &str,
) -> Result<Scripts, String> {
    let name = format!("{}-{stem}.ps1", kind.prefix());
    let (text, done) = match kind {
        ScriptKind::Apply => {
            scripts::apply_script(catalog, profile, ids, version).map(|(t, p)| (t, Done::Script(p)))
        }
        ScriptKind::Revert => scripts::revert_script(catalog, profile, ids, version)
            .map(|(t, p)| (t, Done::Script(p))),
        ScriptKind::Audit => scripts::audit_script(catalog, profile, ids, version)
            .map(|(t, p)| (t, Done::Audit(p.rules.len()))),
    }
    .map_err(|e| e.to_string())?;
    let mut files = vec![OutFile {
        name,
        bytes: scripts::file_bytes(&text),
    }];
    if kind != ScriptKind::Audit {
        let undo = scripts::undo_script(profile, version).map_err(|e| e.to_string())?;
        files.push(OutFile {
            name: scripts::UNDO_NAME.to_owned(),
            bytes: scripts::file_bytes(&undo),
        });
    }
    Ok(Scripts { files, done })
}

/// The registry file of the apply (or the return, `revert`) of the rules `ids` of a profile; the
/// steps a registry file cannot hold are comments in it.
pub fn reg(catalog: &Catalog, profile: &Profile, ids: &[String], revert: bool, stem: &str) -> Reg {
    let name = &profile.name;
    let (plan, title, file_name) = if revert {
        (
            apply::plan_revert(catalog, ids, "reg"),
            format!("Return the rules of {name} to the defaults of Windows"),
            format!("Revert-{stem}.reg"),
        )
    } else {
        (
            apply::plan_apply(catalog, profile, ids, "reg"),
            format!("Apply the profile {name}"),
            format!("Apply-{stem}.reg"),
        )
    };
    let file = RegFile::from_plan(&plan, &title);
    let values = file
        .lines
        .iter()
        .filter(|l| matches!(l, Line::Value { .. }))
        .count();
    Reg {
        file,
        name: file_name,
        values,
    }
}

/// The profile a diff applies and its rules: the other side of every rule it names (a rule the
/// other side leaves out is off, so it returns to the defaults). The fields of the forms belong to
/// the answer file and are not part of it.
pub fn diff_profile(diff: &Diff) -> (Profile, Vec<String>) {
    let name = if diff.other.name.is_empty() {
        "diff"
    } else {
        diff.other.name.as_str()
    };
    let mut profile = Profile::new(name, &diff.catalog);
    for (id, d) in &diff.rules {
        if let Some(choice) = &d.other {
            profile.rules.insert(id.clone(), choice.clone());
        }
    }
    (profile, diff.rules.keys().cloned().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{RuleDiff, Side};
    use crate::profile::Choice;

    fn catalog() -> Catalog {
        Catalog::embedded().unwrap()
    }

    #[test]
    fn the_passes_of_the_installation_have_no_script() {
        let catalog = catalog();
        let install = all_rules(&catalog)
            .into_iter()
            .filter(|id| {
                catalog
                    .rule(id)
                    .is_some_and(|r| thispc::exclusion(r).is_some())
            })
            .take(3)
            .collect::<Vec<_>>();
        assert!(!install.is_empty());
        assert!(!applicable(&catalog, &install));
        assert!(applicable(&catalog, &all_rules(&catalog)));
    }

    #[test]
    fn a_group_of_the_command_line_names_its_rules() {
        let catalog = catalog();
        let group = catalog.groups()[0].id.clone();
        let rules = rules_of(&catalog, std::slice::from_ref(&group)).unwrap();
        assert!(!rules.is_empty());
        assert!(rules_of(&catalog, &["no-such-thing".to_owned()]).is_err());
    }

    #[test]
    fn a_diff_applies_its_other_side() {
        let catalog = catalog();
        let rule = catalog
            .rules()
            .iter()
            .find(|r| {
                thispc::exclusion(r).is_none() && r.params.is_empty() && !r.actions.is_empty()
            })
            .unwrap()
            .id
            .clone();
        let mut diff = Diff::between(
            &Profile::new("Base", catalog.version()),
            &Profile::new("Other", catalog.version()),
            Side::default(),
            Side {
                name: "Other".to_owned(),
                ..Side::default()
            },
        );
        diff.rules.insert(
            rule.clone(),
            RuleDiff {
                base: None,
                other: Some(Choice {
                    on: true,
                    ..Choice::default()
                }),
            },
        );
        let (profile, ids) = diff_profile(&diff);
        assert_eq!(ids, vec![rule.clone()]);
        assert!(profile.rules[&rule].on);
        let made = scripts(&catalog, &profile, &ids, ScriptKind::Apply, "Other", "test").unwrap();
        assert_eq!(made.files[0].name, "Apply-Other.ps1");
        assert_eq!(made.files[1].name, scripts::UNDO_NAME);
        match made.done {
            Done::Script(plan) => assert!(plan.applied.contains(&rule)),
            Done::Audit(_) => panic!("an apply"),
        }
        let reg = reg(&catalog, &profile, &ids, true, "Other");
        assert_eq!(reg.name, "Revert-Other.reg");
    }

    #[test]
    fn a_file_stem_is_safe() {
        assert_eq!(file_stem("Office 2026"), "Office 2026");
        assert_eq!(file_stem("a/b:c"), "a_b_c");
        assert_eq!(file_stem("   "), "profile");
    }
}
