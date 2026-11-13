//! The files of an apply made from a profile, a selection of its rules or a diff, for the window
//! and the command line: the stand-alone scripts (apply, return to the defaults, audit, with the
//! undo script beside an apply or a return) and the registry files of an apply or a return. A
//! diff applies its other side: the rules it turns on are applied, the ones it turns off return to
//! the defaults of Windows. The command line writes each file where [`out_path`] says.

use std::path::{Path, PathBuf};

use crate::apply;
use crate::catalog::Catalog;
use crate::diff::Diff;
use crate::i18n::CatalogTexts;
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

/// Whether a path of the command line names a file with the extension `ext` (in any case) rather
/// than a folder; an existing folder never does.
pub fn names_file(path: &Path, ext: &str) -> bool {
    let fits = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case(ext));
    fits && !path.is_dir()
}

/// The file a command of the command line writes: `out` when it names a file with the extension
/// `ext` ([`names_file`]); any other `out` is the folder of the file, which gets `name` there;
/// without `out` the file `name` in the folder `dir`.
pub fn out_path(out: Option<&Path>, dir: &Path, name: &str, ext: &str) -> PathBuf {
    match out {
        Some(file) if names_file(file, ext) => file.to_path_buf(),
        Some(folder) => folder.join(name),
        None => dir.join(name),
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

/// The title and the description of a rule of the catalog in a language (`None` for English), for
/// the comments of a registry file.
pub fn describe(
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    id: &str,
) -> Option<(String, String)> {
    let rule = catalog.rule(id)?;
    Some(match texts {
        Some(t) => (
            t.rule_title(rule).to_owned(),
            t.rule_summary(rule).to_owned(),
        ),
        None => (rule.title.clone(), rule.summary.clone()),
    })
}

/// The registry file of the apply (or the return, `revert`) of the rules `ids` of a profile; the
/// steps a registry file cannot hold are comments in it, and every rule has its title and
/// description from the catalog in the language of `texts` (English without it).
pub fn reg(
    catalog: &Catalog,
    texts: Option<&CatalogTexts>,
    profile: &Profile,
    ids: &[String],
    revert: bool,
    stem: &str,
) -> Reg {
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
    let file = RegFile::from_plan_described(&plan, &title, &|id| describe(catalog, texts, id));
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
        let reverted = reg(&catalog, None, &profile, &ids, true, "Other");
        assert_eq!(reverted.name, "Revert-Other.reg");
        // Every rule has its title from the catalog under its id, in the language given.
        let written = catalog
            .rules()
            .iter()
            .find(|r| {
                r.params.is_empty()
                    && !r.actions.is_empty()
                    && r.actions.iter().all(|a| {
                        matches!(a, crate::catalog::Action::Reg { path, .. } if path.starts_with("HKLM:"))
                    })
            })
            .unwrap();
        let id = written.id.clone();
        let mut on = Profile::new("On", catalog.version());
        on.set_on(&id, true);
        let ids = vec![id.clone()];
        let comment = |file: &RegFile, text: &str| {
            file.lines
                .iter()
                .any(|l| matches!(l, Line::Comment(c) if c == &format!("; {text}")))
        };
        let english = reg(&catalog, None, &on, &ids, false, "On");
        assert!(comment(&english.file, &format!("Rule {id}")));
        assert!(comment(&english.file, &written.title), "the title of {id}");
        let texts = crate::i18n::CatalogTexts::embedded().unwrap();
        let uk = texts.iter().find(|t| t.code == "uk").unwrap();
        let ukrainian = reg(&catalog, Some(uk), &on, &ids, false, "On");
        let own = uk.rule_title(written);
        assert!(comment(&ukrainian.file, own), "the Ukrainian title of {id}");
    }

    #[test]
    fn a_file_stem_is_safe() {
        assert_eq!(file_stem("Office 2026"), "Office 2026");
        assert_eq!(file_stem("a/b:c"), "a_b_c");
        assert_eq!(file_stem("   "), "profile");
    }

    /// Where `--out` puts the registry file `Apply-Office.reg` of the output folder `output`.
    fn reg_out(out: &Path) -> PathBuf {
        out_path(Some(out), Path::new("output"), "Apply-Office.reg", "reg")
    }

    #[test]
    fn an_out_with_the_extension_names_the_file() {
        let named = Path::new("usb").join("Office.reg");
        assert_eq!(reg_out(&named), named);
        let upper = Path::new("usb").join("OFFICE.REG");
        assert_eq!(reg_out(&upper), upper);
        assert!(names_file(Path::new("answer.XML"), "xml"));
        assert!(!names_file(Path::new("answer.xml"), "wwprofile"));
        assert!(!names_file(Path::new("answer"), "xml"));
    }

    #[test]
    fn an_out_without_the_extension_is_a_folder() {
        let none = out_path(None, Path::new("output"), "Apply-Office.reg", "reg");
        assert_eq!(none, Path::new("output").join("Apply-Office.reg"));
        let folder = Path::new("usb").join("files");
        assert_eq!(reg_out(&folder), folder.join("Apply-Office.reg"));
        // Another extension is the name of a folder too.
        let other = Path::new("usb").join("Office.ps1");
        assert_eq!(reg_out(&other), other.join("Apply-Office.reg"));
    }

    #[test]
    fn an_existing_folder_is_a_folder_whatever_its_name() {
        let name = format!("winweaver-out-{}.reg", std::process::id());
        let folder = std::env::temp_dir().join(name);
        std::fs::create_dir_all(&folder).unwrap();
        let target = reg_out(&folder);
        let _ = std::fs::remove_dir(&folder);
        assert_eq!(target, folder.join("Apply-Office.reg"));
    }
}
