//! The model of a set of policy templates and its reading: the ADMX files with the ADML texts
//! of the cultures the program keeps.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use crate::jsonfile;

use super::xml::{Node, parse_xml};
use super::{CULTURES, FILE_MAX};

/// Texts by culture.
pub type Texts = BTreeMap<String, String>;

/// A value of a template.
#[derive(Debug, Clone, PartialEq)]
pub enum Val {
    Decimal(u64),
    LongDecimal(u64),
    Text(String),
    Delete,
}

#[derive(Debug, Clone)]
pub(super) struct Item {
    pub(super) key: Option<String>,
    pub(super) value_name: String,
    pub(super) value: Val,
}

#[derive(Debug, Clone)]
pub(super) enum Element {
    Enum {
        id: String,
        key: Option<String>,
        value_name: String,
        items: Vec<(Texts, Val)>,
    },
    Decimal {
        id: String,
        key: Option<String>,
        value_name: String,
        min: u64,
        max: u64,
        text: bool,
        long: bool,
    },
    Boolean {
        id: String,
        key: Option<String>,
        value_name: String,
        on: Val,
        off: Val,
    },
    Text {
        id: String,
        key: Option<String>,
        value_name: String,
        expandable: bool,
        required: bool,
        /// The `maxLength` of the element, none when absent or not a number above zero.
        max_length: Option<usize>,
    },
    /// The values of a key; `prefix` absent: the items name the values, empty or set: the prefix
    /// and a number; `explicit`: `name=value` items.
    List {
        id: String,
        key: Option<String>,
        prefix: Option<String>,
        additive: bool,
        expandable: bool,
        explicit: bool,
    },
    /// The lines of one multi-line value.
    MultiText {
        id: String,
        key: Option<String>,
        value_name: String,
    },
}

#[derive(Debug, Clone)]
pub(super) struct Category {
    pub(super) ns: String,
    pub(super) name: String,
    pub(super) title: Texts,
    pub(super) parent: Option<(String, String)>,
}

#[derive(Debug, Clone)]
pub(super) struct Policy {
    pub(super) prefix: String,
    /// The target namespace of its ADMX, which tells two policies of one prefix apart.
    pub(super) ns: String,
    pub(super) name: String,
    pub(super) user: bool,
    pub(super) title: Texts,
    pub(super) explain: Texts,
    pub(super) key: String,
    pub(super) value_name: Option<String>,
    pub(super) enabled: Option<Val>,
    pub(super) disabled: Option<Val>,
    pub(super) enabled_list: Vec<Item>,
    pub(super) disabled_list: Vec<Item>,
    pub(super) elements: Vec<Element>,
    pub(super) labels: HashMap<String, Texts>,
    pub(super) defaults: HashMap<String, String>,
    pub(super) category: Option<(String, String)>,
}

/// The policies and categories of a set of templates, and what was skipped.
#[derive(Debug, Default)]
pub struct Templates {
    pub(super) categories: Vec<Category>,
    pub(super) policies: Vec<Policy>,
    pub skipped: Vec<(String, String)>,
}

impl Templates {
    pub fn policies(&self) -> usize {
        self.policies.len()
    }
}

/// The strings and presentations of one ADML.
#[derive(Default)]
pub(super) struct Resources {
    pub(super) strings: HashMap<String, String>,
    pub(super) presentations: HashMap<String, HashMap<String, (String, Option<String>)>>,
}

pub(super) fn resources(text: &str) -> Result<Resources, String> {
    let root = parse_xml(text)?;
    let mut out = Resources::default();
    let Some(res) = root.child("resources") else {
        return Ok(out);
    };
    if let Some(table) = res.child("stringTable") {
        for s in table.all("string") {
            if let Some(id) = s.attr("id") {
                out.strings.insert(id.to_owned(), s.text.clone());
            }
        }
    }
    if let Some(table) = res.child("presentationTable") {
        for p in table.all("presentation") {
            let Some(id) = p.attr("id") else {
                continue;
            };
            let mut labels = HashMap::new();
            for c in &p.children {
                if let Some(r) = c.attr("refId") {
                    let label = c
                        .child("label")
                        .map_or_else(|| c.text.clone(), |l| l.text.clone());
                    let default = c
                        .attr("defaultValue")
                        .map(str::to_owned)
                        .or_else(|| c.child("defaultValue").map(|d| d.text.clone()))
                        .or_else(|| c.attr("defaultItem").map(str::to_owned))
                        .or_else(|| c.attr("defaultChecked").map(str::to_owned));
                    labels.insert(r.to_owned(), (label, default));
                }
            }
            out.presentations.insert(id.to_owned(), labels);
        }
    }
    Ok(out)
}

/// The text of a `$(string.X)` reference in every culture.
fn texts_of(reference: Option<&str>, res: &[(String, Resources)], fallback: &str) -> Texts {
    let mut out = Texts::new();
    let id = reference
        .and_then(|r| r.strip_prefix("$(string."))
        .and_then(|r| r.strip_suffix(')'));
    for (culture, r) in res {
        if let Some(t) = id.and_then(|i| r.strings.get(i)) {
            out.insert(culture.clone(), t.trim().to_owned());
        }
    }
    if out.is_empty() {
        out.insert("en-US".to_owned(), fallback.to_owned());
    }
    out
}

fn value_of(node: &Node) -> Option<Val> {
    if let Some(d) = node.child("decimal") {
        return d.attr("value")?.parse().ok().map(Val::Decimal);
    }
    if let Some(d) = node.child("longDecimal") {
        return d.attr("value")?.parse().ok().map(Val::LongDecimal);
    }
    if let Some(s) = node.child("string") {
        return Some(Val::Text(s.text.clone()));
    }
    if node.child("delete").is_some() {
        return Some(Val::Delete);
    }
    None
}

fn items_of(list: Option<&Node>) -> Result<Vec<Item>, String> {
    let Some(list) = list else {
        return Ok(Vec::new());
    };
    let default_key = list.attr("defaultKey").map(str::to_owned);
    list.all("item")
        .map(|i| {
            let value = i
                .child("value")
                .and_then(value_of)
                .ok_or("an item has no value")?;
            Ok(Item {
                key: i
                    .attr("key")
                    .map(str::to_owned)
                    .or_else(|| default_key.clone()),
                value_name: i
                    .attr("valueName")
                    .ok_or("an item has no value name")?
                    .to_owned(),
                value,
            })
        })
        .collect()
}

fn reference(r: &str, own: &str, using: &HashMap<String, String>) -> (String, String) {
    match r.split_once(':') {
        Some((prefix, name)) => (
            using
                .get(prefix)
                .cloned()
                .unwrap_or_else(|| prefix.to_owned()),
            name.to_owned(),
        ),
        None => (own.to_owned(), r.to_owned()),
    }
}

fn elements_of(node: Option<&Node>, res: &[(String, Resources)]) -> Result<Vec<Element>, String> {
    let Some(node) = node else {
        return Ok(Vec::new());
    };
    let mut out = Vec::new();
    for e in &node.children {
        let id = e.attr("id").ok_or("an element has no id")?.to_owned();
        let key = e.attr("key").map(str::to_owned);
        let value_name = {
            let missing = format!("the element {id} has no value name");
            move || {
                e.attr("valueName")
                    .map(str::to_owned)
                    .ok_or(missing.clone())
            }
        };
        out.push(match e.name.as_str() {
            "enum" => {
                let mut items = Vec::new();
                for item in e.all("item") {
                    if item.child("valueList").is_some() {
                        return Err(format!("the element {id} has value lists"));
                    }
                    let value = item
                        .child("value")
                        .and_then(value_of)
                        .ok_or_else(|| format!("an item of {id} has no value"))?;
                    items.push((texts_of(item.attr("displayName"), res, "?"), value));
                }
                Element::Enum {
                    id,
                    key,
                    value_name: value_name()?,
                    items,
                }
            }
            "decimal" | "longDecimal" => Element::Decimal {
                key,
                value_name: value_name()?,
                min: e.attr("minValue").and_then(|v| v.parse().ok()).unwrap_or(0),
                max: e
                    .attr("maxValue")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(9999),
                text: e.attr("storeAsText") == Some("true"),
                long: e.name == "longDecimal",
                id,
            },
            "boolean" => {
                if e.child("trueList").is_some() || e.child("falseList").is_some() {
                    return Err(format!("the element {id} has value lists"));
                }
                let on = e
                    .child("trueValue")
                    .and_then(value_of)
                    .unwrap_or(Val::Decimal(1));
                let off = e
                    .child("falseValue")
                    .and_then(value_of)
                    .unwrap_or(Val::Decimal(0));
                Element::Boolean {
                    id,
                    key,
                    value_name: value_name()?,
                    on,
                    off,
                }
            }
            "text" => Element::Text {
                key,
                value_name: value_name()?,
                expandable: e.attr("expandable") == Some("true"),
                required: e.attr("required") == Some("true"),
                max_length: e
                    .attr("maxLength")
                    .and_then(|v| v.trim().parse::<usize>().ok())
                    .filter(|&n| n > 0),
                id,
            },
            "list" => Element::List {
                key,
                prefix: e.attr("valuePrefix").map(str::to_owned),
                additive: e.attr("additive") == Some("true"),
                expandable: e.attr("expandable") == Some("true"),
                explicit: e.attr("explicitValue") == Some("true"),
                id,
            },
            "multiText" => Element::MultiText {
                key,
                value_name: value_name()?,
                id,
            },
            other => {
                return Err(format!(
                    "the element {id} is a {other}, which the program does not import yet"
                ));
            }
        });
    }
    Ok(out)
}

/// Reads one ADMX with the ADML texts of its cultures.
pub(super) fn read_pair(
    admx: &str,
    adml: &[(String, String)],
    file: &str,
    out: &mut Templates,
) -> Result<(), String> {
    let root = parse_xml(admx)?;
    if root.name != "policyDefinitions" {
        return Err("not an ADMX file".to_owned());
    }
    let ns_node = root
        .child("policyNamespaces")
        .ok_or("no policyNamespaces")?;
    let target = ns_node.child("target").ok_or("no target namespace")?;
    let ns = target
        .attr("namespace")
        .ok_or("no target namespace")?
        .to_owned();
    let prefix = target.attr("prefix").unwrap_or("x").to_owned();
    let mut using = HashMap::new();
    for u in ns_node.all("using") {
        if let (Some(p), Some(n)) = (u.attr("prefix"), u.attr("namespace")) {
            using.insert(p.to_owned(), n.to_owned());
        }
    }
    let mut res = Vec::new();
    for (culture, text) in adml {
        match resources(text) {
            Ok(r) => res.push((culture.clone(), r)),
            Err(e) => out.skipped.push((
                format!("{file} {culture}"),
                format!("the texts were not read: {e}"),
            )),
        }
    }
    if let Some(cats) = root.child("categories") {
        for c in cats.all("category") {
            let Some(name) = c.attr("name") else {
                continue;
            };
            out.categories.push(Category {
                ns: ns.clone(),
                name: name.to_owned(),
                title: texts_of(c.attr("displayName"), &res, name),
                parent: c
                    .child("parentCategory")
                    .and_then(|p| p.attr("ref"))
                    .map(|r| reference(r, &ns, &using)),
            });
        }
    }
    let Some(policies) = root.child("policies") else {
        return Ok(());
    };
    for p in policies.all("policy") {
        let name = p.attr("name").unwrap_or("?").to_owned();
        let built = (|| -> Result<Policy, String> {
            let class = p.attr("class").unwrap_or("Machine");
            let presentation = p
                .attr("presentation")
                .and_then(|r| r.strip_prefix("$(presentation."))
                .and_then(|r| r.strip_suffix(')'));
            let mut labels: HashMap<String, Texts> = HashMap::new();
            let mut defaults: HashMap<String, String> = HashMap::new();
            for (culture, r) in &res {
                if let Some(table) = presentation.and_then(|id| r.presentations.get(id)) {
                    for (refid, (label, default)) in table {
                        labels
                            .entry(refid.clone())
                            .or_default()
                            .insert(culture.clone(), label.trim().to_owned());
                        if let Some(d) = default {
                            defaults.entry(refid.clone()).or_insert_with(|| d.clone());
                        }
                    }
                }
            }
            Ok(Policy {
                prefix: prefix.clone(),
                ns: ns.clone(),
                name: name.clone(),
                user: class == "User",
                title: texts_of(p.attr("displayName"), &res, &name),
                explain: texts_of(p.attr("explainText"), &res, ""),
                key: p.attr("key").ok_or("no key")?.to_owned(),
                value_name: p.attr("valueName").map(str::to_owned),
                enabled: p.child("enabledValue").and_then(value_of),
                disabled: p.child("disabledValue").and_then(value_of),
                enabled_list: items_of(p.child("enabledList"))?,
                disabled_list: items_of(p.child("disabledList"))?,
                elements: elements_of(p.child("elements"), &res)?,
                labels,
                defaults,
                category: p
                    .child("parentCategory")
                    .and_then(|c| c.attr("ref"))
                    .map(|r| reference(r, &ns, &using)),
            })
        })();
        match built {
            Ok(policy) => out.policies.push(policy),
            Err(why) => out.skipped.push((format!("{prefix}:{name}"), why)),
        }
    }
    Ok(())
}

/// Reads every ADMX of a folder (the templates of Windows are `C:\Windows\PolicyDefinitions`) with
/// the ADML of the cultures the program keeps. A file that cannot be read is listed, not fatal.
pub fn read_folder(dir: &Path) -> Templates {
    let mut out = Templates::default();
    let Ok(entries) = std::fs::read_dir(dir) else {
        out.skipped.push((
            dir.display().to_string(),
            "the folder cannot be read".to_owned(),
        ));
        return out;
    };
    let mut files: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("admx"))
        })
        .collect();
    files.sort();
    for path in files {
        read_one(&path, &mut out);
    }
    out
}

/// Reads one ADMX file with the ADML of the cultures the program keeps from the folders of the
/// cultures next to it (`en-US/<name>.adml`), as an import of it alone shows it.
pub fn read_file(path: &Path) -> Templates {
    let mut out = Templates::default();
    read_one(path, &mut out);
    out
}

fn read_one(path: &Path, out: &mut Templates) {
    let file = jsonfile::display_name(path);
    let admx = match jsonfile::read_text_bom(path, FILE_MAX) {
        Ok(t) => t,
        Err(e) => {
            out.skipped.push((file, e.to_string()));
            return;
        }
    };
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let dir = path.parent().unwrap_or(Path::new("."));
    let mut adml = Vec::new();
    for (culture, _) in CULTURES {
        let p = dir.join(culture).join(format!("{stem}.adml"));
        if let Ok(t) = jsonfile::read_text_bom(&p, FILE_MAX) {
            adml.push((culture.to_owned(), t));
        }
    }
    if let Err(e) = read_pair(&admx, &adml, &file, out) {
        out.skipped.push((file, e));
    }
}
