//! Policy templates (`docs/spec/03-features.md` section 6): ADMX files with their ADML texts read
//! into rules, kept as catalog files (`.wwcatalog`) and merged into the catalog.
//!
//! Every template is untrusted input: no DTD, size and depth limits, safe characters in keys,
//! value names and values (they reach the registry, the scripts and a CDATA section of the answer
//! file). A policy the program cannot represent is skipped and listed, never fatal. A catalog
//! file is checked again when it is loaded, so that a crafted file cannot bring anything but
//! registry values.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

use indexmap::IndexMap;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::catalog::{Action, Catalog, CatalogError, EnumValue, Group, Level, ListNames, Param, Phase, RegKind, Rule, ROOT_POLICIES, check};
use crate::i18n::{CatalogTexts, GroupText, RuleText};
use crate::jsonfile;

pub const CATALOG_FORMAT: &str = "winweaver-catalog";
pub const CATALOG_VERSION: u32 = 1;
pub const CATALOG_EXTENSION: &str = "wwcatalog";
/// The prefix of the ids of the rules and groups an import brings.
pub const RULE_PREFIX: &str = "admx.";
const FILE_MAX: u64 = 8 * 1024 * 1024;
const CATALOG_MAX: u64 = 64 * 1024 * 1024;
const DEPTH_MAX: usize = 64;
const SUMMARY_MAX: usize = 4000;
const NAME_MAX: usize = 255;
/// The cultures whose texts an import keeps, with the language code of the catalog texts.
pub const CULTURES: [(&str, &str); 3] = [("en-US", "en"), ("uk-UA", "uk"), ("ru-RU", "ru")];

// ------------------------------------------------------------------------------------------------
// A small tree of XML elements

#[derive(Debug, Default)]
struct Node {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<Node>,
    text: String,
}

impl Node {
    fn attr(&self, key: &str) -> Option<&str> {
        self.attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
    }

    fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name == name)
    }

    fn all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> + 'a {
        self.children.iter().filter(move |c| c.name == name)
    }
}

/// Parses XML text into a tree; a DTD, an unknown entity or nesting deeper than the limit is an
/// error.
fn parse_xml(text: &str) -> Result<Node, String> {
    let mut reader = Reader::from_str(text);
    let mut stack: Vec<Node> = vec![Node::default()];
    loop {
        let event = reader.read_event().map_err(|e| format!("not well formed: {e}"))?;
        match event {
            Event::Start(e) => {
                if stack.len() > DEPTH_MAX {
                    return Err("nested too deep".to_owned());
                }
                stack.push(element(&e)?);
            }
            Event::Empty(e) => {
                let node = element(&e)?;
                if let Some(parent) = stack.last_mut() {
                    parent.children.push(node);
                }
            }
            Event::End(_) => {
                let mut node = stack.pop().ok_or("an end tag without a start")?;
                node.text = node.text.trim().to_owned();
                match stack.last_mut() {
                    Some(parent) => parent.children.push(node),
                    None => return Err("an end tag without a start".to_owned()),
                }
            }
            Event::Text(t) => {
                if let Some(n) = stack.last_mut() {
                    n.text.push_str(&t.xml10_content());
                }
            }
            Event::CData(c) => {
                if let Some(n) = stack.last_mut() {
                    n.text.push_str(&c.into_inner());
                }
            }
            Event::GeneralRef(r) => {
                let c = match r.resolve_char_ref().map_err(|e| e.to_string())? {
                    Some(c) => c,
                    None => match r.xml10_content().as_ref() {
                        "lt" => '<',
                        "gt" => '>',
                        "amp" => '&',
                        "quot" => '"',
                        "apos" => '\'',
                        other => return Err(format!("the entity {other} is not allowed")),
                    },
                };
                if let Some(n) = stack.last_mut() {
                    n.text.push(c);
                }
            }
            Event::DocType(_) => return Err("a DTD is not allowed".to_owned()),
            Event::Eof => break,
            _ => {}
        }
    }
    let mut root = stack.pop().ok_or("an empty document")?;
    if !stack.is_empty() {
        return Err("an element is not closed".to_owned());
    }
    root.children.pop().ok_or_else(|| "an empty document".to_owned())
}

fn element(e: &quick_xml::events::BytesStart<'_>) -> Result<Node, String> {
    let mut node = Node {
        name: e.local_name().as_ref().to_owned(),
        ..Node::default()
    };
    for a in e.attributes() {
        let a = a.map_err(|e| format!("an attribute: {e}"))?;
        let key = a.key.local_name().as_ref().to_owned();
        #[allow(deprecated)]
        let value = a.unescape_value().map_err(|e| format!("an attribute: {e}"))?.into_owned();
        node.attrs.push((key, value));
    }
    Ok(node)
}

// ------------------------------------------------------------------------------------------------
// Templates

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
struct Item {
    key: Option<String>,
    value_name: String,
    value: Val,
}

#[derive(Debug, Clone)]
enum Element {
    Enum { id: String, key: Option<String>, value_name: String, items: Vec<(Texts, Val)> },
    Decimal { id: String, key: Option<String>, value_name: String, min: u64, max: u64, text: bool, long: bool },
    Boolean { id: String, key: Option<String>, value_name: String, on: Val, off: Val },
    Text { id: String, key: Option<String>, value_name: String, expandable: bool, required: bool },
    /// The values of a key; `prefix` absent: the items name the values, empty or set: the prefix
    /// and a number; `explicit`: `name=value` items.
    List { id: String, key: Option<String>, prefix: Option<String>, additive: bool, expandable: bool, explicit: bool },
    /// The lines of one multi-line value.
    MultiText { id: String, key: Option<String>, value_name: String },
}

#[derive(Debug, Clone)]
struct Category {
    ns: String,
    name: String,
    title: Texts,
    parent: Option<(String, String)>,
}

#[derive(Debug, Clone)]
struct Policy {
    prefix: String,
    name: String,
    user: bool,
    title: Texts,
    explain: Texts,
    key: String,
    value_name: Option<String>,
    enabled: Option<Val>,
    disabled: Option<Val>,
    enabled_list: Vec<Item>,
    disabled_list: Vec<Item>,
    elements: Vec<Element>,
    labels: HashMap<String, Texts>,
    defaults: HashMap<String, String>,
    category: Option<(String, String)>,
}

/// The policies and categories of a set of templates, and what was skipped.
#[derive(Debug, Default)]
pub struct Templates {
    categories: Vec<Category>,
    policies: Vec<Policy>,
    pub skipped: Vec<(String, String)>,
}

impl Templates {
    pub fn policies(&self) -> usize {
        self.policies.len()
    }
}

/// The strings and presentations of one ADML.
#[derive(Default)]
struct Resources {
    strings: HashMap<String, String>,
    presentations: HashMap<String, HashMap<String, (String, Option<String>)>>,
}

fn resources(text: &str) -> Result<Resources, String> {
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
                    let label = c.child("label").map_or_else(|| c.text.clone(), |l| l.text.clone());
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
            let value = i.child("value").and_then(value_of).ok_or("an item has no value")?;
            Ok(Item {
                key: i.attr("key").map(str::to_owned).or_else(|| default_key.clone()),
                value_name: i.attr("valueName").ok_or("an item has no value name")?.to_owned(),
                value,
            })
        })
        .collect()
}

fn reference(r: &str, own: &str, using: &HashMap<String, String>) -> (String, String) {
    match r.split_once(':') {
        Some((prefix, name)) => (using.get(prefix).cloned().unwrap_or_else(|| prefix.to_owned()), name.to_owned()),
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
            move || e.attr("valueName").map(str::to_owned).ok_or(missing.clone())
        };
        out.push(match e.name.as_str() {
            "enum" => {
                let mut items = Vec::new();
                for item in e.all("item") {
                    if item.child("valueList").is_some() {
                        return Err(format!("the element {id} has value lists"));
                    }
                    let value = item.child("value").and_then(value_of).ok_or_else(|| format!("an item of {id} has no value"))?;
                    items.push((texts_of(item.attr("displayName"), res, "?"), value));
                }
                Element::Enum { id, key, value_name: value_name()?, items }
            }
            "decimal" | "longDecimal" => Element::Decimal {
                key,
                value_name: value_name()?,
                min: e.attr("minValue").and_then(|v| v.parse().ok()).unwrap_or(0),
                max: e.attr("maxValue").and_then(|v| v.parse().ok()).unwrap_or(9999),
                text: e.attr("storeAsText") == Some("true"),
                long: e.name == "longDecimal",
                id,
            },
            "boolean" => {
                if e.child("trueList").is_some() || e.child("falseList").is_some() {
                    return Err(format!("the element {id} has value lists"));
                }
                let on = e.child("trueValue").and_then(value_of).unwrap_or(Val::Decimal(1));
                let off = e.child("falseValue").and_then(value_of).unwrap_or(Val::Decimal(0));
                Element::Boolean { id, key, value_name: value_name()?, on, off }
            }
            "text" => Element::Text {
                key,
                value_name: value_name()?,
                expandable: e.attr("expandable") == Some("true"),
                required: e.attr("required") == Some("true"),
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
            other => return Err(format!("the element {id} is a {other}, which the program does not import yet")),
        });
    }
    Ok(out)
}

/// Reads one ADMX with the ADML texts of its cultures.
fn read_pair(admx: &str, adml: &[(String, String)], file: &str, out: &mut Templates) -> Result<(), String> {
    let root = parse_xml(admx)?;
    if root.name != "policyDefinitions" {
        return Err("not an ADMX file".to_owned());
    }
    let ns_node = root.child("policyNamespaces").ok_or("no policyNamespaces")?;
    let target = ns_node.child("target").ok_or("no target namespace")?;
    let ns = target.attr("namespace").ok_or("no target namespace")?.to_owned();
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
            Err(e) => out.skipped.push((format!("{file} {culture}"), format!("the texts were not read: {e}"))),
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
                parent: c.child("parentCategory").and_then(|p| p.attr("ref")).map(|r| reference(r, &ns, &using)),
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
                        labels.entry(refid.clone()).or_default().insert(culture.clone(), label.trim().to_owned());
                        if let Some(d) = default {
                            defaults.entry(refid.clone()).or_insert_with(|| d.clone());
                        }
                    }
                }
            }
            Ok(Policy {
                prefix: prefix.clone(),
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
                category: p.child("parentCategory").and_then(|c| c.attr("ref")).map(|r| reference(r, &ns, &using)),
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
        out.skipped.push((dir.display().to_string(), "the folder cannot be read".to_owned()));
        return out;
    };
    let mut files: Vec<_> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e.eq_ignore_ascii_case("admx")))
        .collect();
    files.sort();
    for path in files {
        let file = jsonfile::display_name(&path);
        let admx = match jsonfile::read_text_bom(&path, FILE_MAX) {
            Ok(t) => t,
            Err(e) => {
                out.skipped.push((file, e.to_string()));
                continue;
            }
        };
        let stem = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
        let mut adml = Vec::new();
        for (culture, _) in CULTURES {
            let p = dir.join(culture).join(format!("{stem}.adml"));
            if let Ok(t) = jsonfile::read_text_bom(&p, FILE_MAX) {
                adml.push((culture.to_owned(), t));
            }
        }
        if let Err(e) = read_pair(&admx, &adml, &file, &mut out) {
            out.skipped.push((file, e));
        }
    }
    out
}

// ------------------------------------------------------------------------------------------------
// Rules

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

fn slug(text: &str) -> String {
    let s: String = text
        .to_ascii_lowercase()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .collect();
    let s = s.trim_matches('-').to_owned();
    if s.is_empty() { "x".to_owned() } else { s.chars().take(80).collect() }
}

fn en(texts: &Texts) -> String {
    texts
        .get("en-US")
        .or_else(|| texts.values().next())
        .cloned()
        .unwrap_or_default()
}

/// Whether a text can go into a key, a value name or a value: no control characters, nothing that
/// closes a CDATA section, no non-characters, within the length limit.
pub fn safe_text(text: &str, limit: usize) -> bool {
    text.chars().count() <= limit
        && !text.contains("]]>")
        && !text.chars().any(|c| c.is_control() || c == '\u{FFFE}' || c == '\u{FFFF}' || ('\u{80}'..='\u{9F}').contains(&c))
}

/// A registry key of a template: segments that are not empty, `.` or `..`.
pub fn safe_key(key: &str) -> bool {
    safe_text(key, 1024)
        && !key.is_empty()
        && key.split(['\\', '/']).all(|s| !s.is_empty() && s != "." && s != ".." && s.len() <= NAME_MAX)
}

fn cut(text: &str, limit: usize) -> String {
    let clean: String = text.chars().map(|c| if c.is_control() && c != '\n' { ' ' } else { c }).collect();
    if clean.chars().count() <= limit {
        clean
    } else {
        clean.chars().take(limit).collect::<String>() + "..."
    }
}

fn reg_of(path: &str, name: &str, val: &Val) -> Result<Action, String> {
    if !safe_text(name, 16383) {
        return Err("a value name with unsafe characters".to_owned());
    }
    Ok(match val {
        Val::Delete => Action::RegRemove { path: path.to_owned(), name: name.to_owned(), default: None },
        Val::Decimal(n) => Action::Reg { path: path.to_owned(), name: name.to_owned(), kind: RegKind::DWord, value: Value::from(*n), default: None, why: String::new() },
        Val::LongDecimal(n) => Action::Reg { path: path.to_owned(), name: name.to_owned(), kind: RegKind::QWord, value: Value::from(*n), default: None, why: String::new() },
        Val::Text(t) => {
            if !safe_text(t, 16383) || t.contains('{') {
                return Err("a text value with unsafe characters or braces".to_owned());
            }
            Action::Reg { path: path.to_owned(), name: name.to_owned(), kind: RegKind::String, value: Value::from(t.as_str()), default: None, why: String::new() }
        }
    })
}

/// The rules of one policy: the rule that enables it and the rule `<id>.off` that disables it.
fn policy_rules(p: &Policy, id: &str, group: &str) -> Result<(Rule, Option<Rule>), String> {
    if !safe_key(&p.key) {
        return Err("an unsafe registry key".to_owned());
    }
    let root = if p.user { "DU:\\" } else { "HKLM:\\" };
    let path_of = |key: &Option<String>| -> Result<String, String> {
        let k = key.as_deref().unwrap_or(&p.key);
        if !safe_key(k) {
            return Err("an unsafe registry key".to_owned());
        }
        Ok(format!("{root}{k}"))
    };
    let main_path = format!("{root}{}", p.key);
    let mut on_actions = Vec::new();
    let mut off_actions = Vec::new();
    if let Some(name) = &p.value_name {
        on_actions.push(reg_of(&main_path, name, p.enabled.as_ref().unwrap_or(&Val::Decimal(1)))?);
        off_actions.push(reg_of(&main_path, name, p.disabled.as_ref().unwrap_or(&Val::Decimal(0)))?);
    }
    for item in &p.enabled_list {
        on_actions.push(reg_of(&path_of(&item.key)?, &item.value_name, &item.value)?);
    }
    for item in &p.disabled_list {
        off_actions.push(reg_of(&path_of(&item.key)?, &item.value_name, &item.value)?);
    }
    let mut params = IndexMap::new();
    // A list that is not additive deletes the other values of its key, so the lists come first.
    let mut list_on = Vec::new();
    let mut list_off = Vec::new();
    for (n, e) in p.elements.iter().enumerate() {
        let pname = format!("e{}", n + 1);
        let placeholder = Value::from(format!("{{{pname}}}"));
        let (key, eid) = match e {
            Element::Enum { key, id, .. }
            | Element::Decimal { key, id, .. }
            | Element::Boolean { key, id, .. }
            | Element::Text { key, id, .. }
            | Element::List { key, id, .. }
            | Element::MultiText { key, id, .. } => (key, id),
        };
        let path = path_of(key)?;
        let label = p.labels.get(eid).map(en).filter(|l| !l.is_empty()).unwrap_or_else(|| eid.clone());
        let value_name = match e {
            Element::List { prefix, additive, expandable, explicit, .. } => {
                if prefix.as_deref().is_some_and(|x| !safe_text(x, NAME_MAX) || x.contains('{')) {
                    return Err("a list prefix with unsafe characters".to_owned());
                }
                let names = if *explicit {
                    ListNames::Pairs
                } else if prefix.is_some() {
                    ListNames::Numbered
                } else {
                    ListNames::Items
                };
                let kind = if *expandable { RegKind::ExpandString } else { RegKind::String };
                let prefix = prefix.clone().unwrap_or_default();
                if !*additive {
                    list_off.push(Action::RegList { path: path.clone(), param: String::new(), names, prefix: prefix.clone(), additive: false, kind });
                }
                list_on.push(Action::RegList { path, param: pname.clone(), names, prefix, additive: *additive, kind });
                params.insert(pname, Param::List { title: cut(&label, 200), default: Vec::new() });
                continue;
            }
            Element::Enum { value_name, .. }
            | Element::Decimal { value_name, .. }
            | Element::Boolean { value_name, .. }
            | Element::Text { value_name, .. }
            | Element::MultiText { value_name, .. } => value_name,
        };
        if !safe_text(value_name, 16383) {
            return Err("a value name with unsafe characters".to_owned());
        }
        let (kind, param) = match e {
            Element::Enum { items, .. } => {
                let decimal = items.iter().all(|(_, v)| matches!(v, Val::Decimal(_)));
                let text = items.iter().all(|(_, v)| matches!(v, Val::Text(_)));
                if items.is_empty() || !(decimal || text) {
                    return Err("an enum of mixed or deleted values".to_owned());
                }
                let values: Vec<EnumValue> = items
                    .iter()
                    .map(|(t, v)| EnumValue {
                        value: match v {
                            Val::Decimal(n) => Value::from(*n),
                            Val::Text(s) => Value::from(s.as_str()),
                            _ => Value::Null,
                        },
                        title: cut(&en(t), 200),
                    })
                    .collect();
                if values.iter().any(|v| v.value.as_str().is_some_and(|s| !safe_text(s, 1000) || s.contains('{'))) {
                    return Err("an enum value with unsafe characters".to_owned());
                }
                let default = p
                    .defaults
                    .get(eid)
                    .and_then(|d| d.parse::<usize>().ok())
                    .and_then(|i| values.get(i))
                    .map_or_else(|| values[0].value.clone(), |v| v.value.clone());
                (
                    if decimal { RegKind::DWord } else { RegKind::String },
                    Param::Enum { title: cut(&label, 200), default, values, differs_from: None, same_allowed: Vec::new() },
                )
            }
            Element::Boolean { on, off, .. } => {
                let pair = [on, off];
                let decimal = pair.iter().all(|v| matches!(v, Val::Decimal(_)));
                if !decimal {
                    return Err("a check box that writes texts or deletes".to_owned());
                }
                let num = |v: &Val| if let Val::Decimal(n) = v { Value::from(*n) } else { Value::Null };
                let checked = p.defaults.get(eid).is_some_and(|d| d == "true");
                (
                    RegKind::DWord,
                    Param::Enum {
                        title: cut(&label, 200),
                        default: if checked { num(on) } else { num(off) },
                        values: vec![
                            EnumValue { value: num(on), title: "On".to_owned() },
                            EnumValue { value: num(off), title: "Off".to_owned() },
                        ],
                        differs_from: None,
                        same_allowed: Vec::new(),
                    },
                )
            }
            Element::Decimal { min, max, text, long, .. } => {
                if min > max || *max > i64::MAX as u64 {
                    return Err("a number with a wrong range".to_owned());
                }
                let default = p.defaults.get(eid).and_then(|d| d.parse::<i64>().ok()).filter(|d| (*min as i64..=*max as i64).contains(d)).unwrap_or(*min as i64);
                (
                    if *text { RegKind::String } else if *long { RegKind::QWord } else { RegKind::DWord },
                    Param::Int { title: cut(&label, 200), default, min: *min as i64, max: *max as i64 },
                )
            }
            Element::Text { expandable, required, .. } => {
                let default = p.defaults.get(eid).cloned().unwrap_or_default();
                if !safe_text(&default, 1000) || default.contains('{') {
                    return Err("a default text with unsafe characters".to_owned());
                }
                (
                    if *expandable { RegKind::ExpandString } else { RegKind::String },
                    Param::Text { title: cut(&label, 200), default: default.clone(), required: *required && !default.is_empty() },
                )
            }
            Element::MultiText { .. } => (RegKind::MultiString, Param::List { title: cut(&label, 200), default: Vec::new() }),
            Element::List { .. } => continue,
        };
        on_actions.push(Action::Reg { path: path.clone(), name: value_name.clone(), kind, value: placeholder, default: None, why: String::new() });
        off_actions.push(Action::RegRemove { path, name: value_name.clone(), default: None });
        params.insert(pname, param);
    }
    list_on.append(&mut on_actions);
    on_actions = list_on;
    list_off.append(&mut off_actions);
    off_actions = list_off;
    if on_actions.is_empty() {
        return Err("the policy writes nothing".to_owned());
    }
    let title = cut(&en(&p.title), 300);
    let summary = cut(&en(&p.explain), SUMMARY_MAX);
    let phase = if p.user { Phase::DefaultUser } else { Phase::Specialize };
    let off_id = format!("{id}.off");
    let has_off = !off_actions.is_empty();
    let on = Rule {
        id: id.to_owned(),
        group: group.to_owned(),
        phase,
        title: title.clone(),
        level: Level::Optional,
        default: false,
        tags: vec![p.prefix.clone(), p.name.clone()],
        doc: String::new(),
        summary: summary.clone(),
        effect: String::new(),
        risk: String::new(),
        versions: String::new(),
        rollback: String::new(),
        verify: String::new(),
        note: String::new(),
        params,
        requires: Vec::new(),
        conflicts: if has_off { vec![off_id.clone()] } else { Vec::new() },
        actions: on_actions,
    };
    let off = has_off.then(|| Rule {
        id: off_id,
        title: format!("{title}: Disabled"),
        params: IndexMap::new(),
        conflicts: vec![id.to_owned()],
        actions: off_actions,
        ..on.clone()
    });
    Ok((on, off))
}

/// The catalog file of a set of templates.
pub fn to_catalog_file(t: &Templates, id: &str, name: &str, windows: &str, created: &str) -> CatalogFile {
    let root_id = format!("admx.{}", slug(id));
    let mut groups = vec![Group {
        id: root_id.clone(),
        title: name.to_owned(),
        order: 900,
        summary: String::new(),
        parent: Some(ROOT_POLICIES.to_owned()),
    }];
    let mut texts: IndexMap<String, LangTexts> = IndexMap::new();
    let mut group_of: HashMap<(String, String), String> = HashMap::new();
    let mut used_groups: HashSet<String> = HashSet::new();
    used_groups.insert(root_id.clone());
    for c in &t.categories {
        if group_of.contains_key(&(c.ns.clone(), c.name.clone())) {
            continue;
        }
        let mut gid = format!("{root_id}.{}.{}", slug(c.ns.rsplit('.').next().unwrap_or("x")), slug(&c.name));
        let mut n = 2;
        while used_groups.contains(&gid) {
            gid = format!("{root_id}.{}-{n}", slug(&c.name));
            n += 1;
        }
        used_groups.insert(gid.clone());
        group_of.insert((c.ns.clone(), c.name.clone()), gid);
    }
    let mut placed: HashSet<(String, String)> = HashSet::new();
    for (order, c) in t.categories.iter().enumerate() {
        if !placed.insert((c.ns.clone(), c.name.clone())) {
            continue;
        }
        let gid = group_of[&(c.ns.clone(), c.name.clone())].clone();
        let parent = c.parent.as_ref().and_then(|p| group_of.get(p)).cloned().unwrap_or_else(|| root_id.clone());
        groups.push(Group {
            id: gid.clone(),
            title: cut(&en(&c.title), 200),
            order: 1000 + order as u32,
            summary: String::new(),
            parent: Some(parent),
        });
        for (culture, code) in CULTURES.iter().skip(1) {
            if let Some(text) = c.title.get(*culture) {
                texts.entry((*code).to_owned()).or_default().groups.insert(gid.clone(), cut(text, 200));
            }
        }
    }
    // A cycle of parents would hang the tree: a group whose chain does not reach the root goes
    // under the root.
    let parents: HashMap<String, String> = groups.iter().filter_map(|g| g.parent.clone().map(|p| (g.id.clone(), p))).collect();
    for g in groups.iter_mut().skip(1) {
        let mut seen = HashSet::new();
        let mut cur = g.id.clone();
        let mut ok = false;
        while let Some(p) = parents.get(&cur) {
            if p == ROOT_POLICIES {
                ok = true;
                break;
            }
            if !seen.insert(p.clone()) {
                break;
            }
            cur = p.clone();
        }
        if !ok {
            g.parent = Some(root_id.clone());
        }
    }
    let mut rules = Vec::new();
    let mut skipped = t.skipped.clone();
    let mut used: HashSet<String> = HashSet::new();
    for p in &t.policies {
        let base = format!("admx.{}.{}", slug(&p.prefix), slug(&p.name));
        let mut rid = base.clone();
        let mut n = 2;
        while used.contains(&rid) || used.contains(&format!("{rid}.off")) {
            rid = format!("{base}-{n}");
            n += 1;
        }
        let group = p.category.as_ref().and_then(|c| group_of.get(c)).cloned().unwrap_or_else(|| root_id.clone());
        match policy_rules(p, &rid, &group) {
            Ok((on, off)) => {
                used.insert(rid.clone());
                used.insert(format!("{rid}.off"));
                for (culture, code) in CULTURES.iter().skip(1) {
                    if let Some(title) = p.title.get(*culture) {
                        let summary = p.explain.get(*culture).map(|s| cut(s, SUMMARY_MAX)).unwrap_or_default();
                        let lang = texts.entry((*code).to_owned()).or_default();
                        lang.rules.insert(rid.clone(), (cut(title, 300), summary));
                    }
                }
                rules.push(on);
                if let Some(off) = off {
                    rules.push(off);
                }
            }
            Err(why) => skipped.push((format!("{}:{}", p.prefix, p.name), why)),
        }
    }
    CatalogFile {
        format: CATALOG_FORMAT.to_owned(),
        version: CATALOG_VERSION,
        kind: "admx".to_owned(),
        id: slug(id),
        name: name.to_owned(),
        windows: windows.to_owned(),
        created: created.to_owned(),
        groups,
        rules,
        texts,
        skipped,
    }
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
        let tmp = path.with_extension("tmp");
        std::fs::write(&tmp, text).map_err(|e| format!("{}: {e}", tmp.display()))?;
        std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
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
                return Err(format!("the rule {} refers to rules outside the file", r.id));
            }
            if !matches!(r.phase, Phase::Specialize | Phase::DefaultUser) {
                return Err(format!("the rule {} has a phase an import cannot have", r.id));
            }
            for a in &r.actions {
                let (path, name) = match a {
                    Action::Reg { path, name, value, .. } => {
                        let placeholders: Vec<&str> = value.as_str().map(check::placeholders).unwrap_or_default();
                        match value {
                            Value::String(s) if !placeholders.is_empty() => {
                                if placeholders.len() != 1 || s != &format!("{{{}}}", placeholders[0]) || !r.params.contains_key(placeholders[0]) {
                                    return Err(format!("the rule {} has a value with a placeholder of its own", r.id));
                                }
                            }
                            Value::String(s) if !safe_text(s, 16383) || s.contains('{') => {
                                return Err(format!("the rule {} writes an unsafe text", r.id));
                            }
                            Value::String(_) | Value::Number(_) => {}
                            _ => return Err(format!("the rule {} writes a value of an unknown shape", r.id)),
                        }
                        (path, name)
                    }
                    Action::RegRemove { path, name, .. } => (path, name),
                    Action::RegList { path, param, kind, prefix, .. } => {
                        let list = param.is_empty() || matches!(r.params.get(param), Some(Param::List { .. }));
                        if !list || !matches!(kind, RegKind::String | RegKind::ExpandString) || prefix.contains('{') {
                            return Err(format!("the rule {} has a list of an unknown shape", r.id));
                        }
                        (path, prefix)
                    }
                    _ => return Err(format!("the rule {} has an action an import cannot have", r.id)),
                };
                let key = path.strip_prefix("HKLM:\\").or_else(|| path.strip_prefix("DU:\\"));
                if !key.is_some_and(safe_key) || !safe_text(name, 16383) {
                    return Err(format!("the rule {} writes an unsafe registry key or value name", r.id));
                }
            }
            for p in r.params.values() {
                let texts: Vec<&str> = match p {
                    Param::Enum { values, .. } => values.iter().filter_map(|v| v.value.as_str()).collect(),
                    Param::Text { default, .. } => vec![default.as_str()],
                    Param::List { default, .. } => default.iter().map(String::as_str).collect(),
                    Param::Int { .. } => Vec::new(),
                };
                if texts.iter().any(|t| !safe_text(t, 16383) || t.contains('{')) {
                    return Err(format!("the rule {} has an unsafe parameter text", r.id));
                }
            }
        }
        Ok(())
    }
}

/// The catalog with the rules and groups of the catalog files added; a rule or group whose id is
/// already there is left out (the first file wins), and the notes say so.
pub fn merge(base: &Catalog, files: &[CatalogFile]) -> Result<(Catalog, Vec<String>), CatalogError> {
    let mut groups: Vec<Group> = base.groups().to_vec();
    let mut rules: Vec<Rule> = base.rules().to_vec();
    let mut group_ids: HashSet<String> = groups.iter().map(|g| g.id.clone()).collect();
    let mut rule_ids: HashSet<String> = rules.iter().map(|r| r.id.clone()).collect();
    let mut notes = Vec::new();
    for f in files {
        let mut skipped = 0;
        for g in &f.groups {
            if group_ids.insert(g.id.clone()) {
                groups.push(g.clone());
            }
        }
        let own_rules: HashSet<&str> = f.rules.iter().map(|r| r.id.as_str()).filter(|id| !rule_ids.contains(*id)).collect();
        for r in &f.rules {
            if !own_rules.contains(r.id.as_str()) || !group_ids.contains(&r.group) {
                skipped += 1;
                continue;
            }
            let mut r = r.clone();
            r.conflicts.retain(|c| own_rules.contains(c.as_str()));
            rule_ids.insert(r.id.clone());
            rules.push(r);
        }
        if skipped > 0 {
            notes.push(format!("{}: {skipped} rules are already in the catalog and were left out", f.name));
        }
    }
    let catalog = Catalog::from_parts(base.version(), groups, rules)?;
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

#[cfg(test)]
mod tests {
    use super::*;

    const ADMX: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<policyDefinitions xmlns="http://schemas.microsoft.com/GroupPolicy/2006/07/PolicyDefinitions" revision="1.0" schemaVersion="1.0">
  <policyNamespaces>
    <target prefix="sample" namespace="Test.Policies.Sample" />
    <using prefix="windows" namespace="Microsoft.Policies.Windows" />
  </policyNamespaces>
  <resources minRequiredRevision="1.0" />
  <categories>
    <category name="Sample" displayName="$(string.Sample)">
      <parentCategory ref="windows:WindowsComponents" />
    </category>
    <category name="Inner" displayName="$(string.Inner)">
      <parentCategory ref="Sample" />
    </category>
  </categories>
  <policies>
    <policy name="TurnOff" class="Machine" displayName="$(string.TurnOff)" explainText="$(string.TurnOff_Help)" key="Software\Policies\Test" valueName="Off">
      <parentCategory ref="Inner" />
      <enabledValue><decimal value="1" /></enabledValue>
      <disabledValue><decimal value="0" /></disabledValue>
    </policy>
    <policy name="Level" class="User" displayName="$(string.Level)" explainText="$(string.Level_Help)" key="Software\Policies\Test" presentation="$(presentation.Level)">
      <parentCategory ref="Sample" />
      <elements>
        <enum id="Mode" valueName="Mode">
          <item displayName="$(string.Low)"><value><decimal value="1" /></value></item>
          <item displayName="$(string.High)"><value><decimal value="2" /></value></item>
        </enum>
        <decimal id="Size" valueName="Size" minValue="1" maxValue="50" />
        <text id="Name" valueName="Name" />
      </elements>
    </policy>
    <policy name="Listed" class="Machine" displayName="$(string.Listed)" key="Software\Policies\Test">
      <elements><list id="L" key="Software\Policies\Test\L" /></elements>
    </policy>
    <policy name="Lines" class="Machine" displayName="$(string.Listed)" key="Software\Policies\Test">
      <elements>
        <multiText id="M" valueName="Lines" />
        <list id="N" key="Software\Policies\Test\N" valuePrefix="" />
      </elements>
    </policy>
    <policy name="Unsafe" class="Machine" displayName="$(string.Listed)" key="Software\Policies\..\Test" valueName="X" />
  </policies>
</policyDefinitions>"#;

    const ADML: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<policyDefinitionResources revision="1.0" schemaVersion="1.0">
  <resources>
    <stringTable>
      <string id="Sample">Sample &amp; tests</string>
      <string id="Inner">Inner</string>
      <string id="TurnOff">Turn the sample off</string>
      <string id="TurnOff_Help">Explains &#x41; lot.</string>
      <string id="Level">Level</string>
      <string id="Level_Help">Sets the level.</string>
      <string id="Low">Low</string>
      <string id="High">High</string>
      <string id="Listed">Listed</string>
    </stringTable>
    <presentationTable>
      <presentation id="Level">
        <dropdownList refId="Mode" defaultItem="1">Mode of work</dropdownList>
        <decimalTextBox refId="Size" defaultValue="10">Size</decimalTextBox>
        <textBox refId="Name"><label>Name</label></textBox>
      </presentation>
    </presentationTable>
  </resources>
</policyDefinitionResources>"#;

    const ADML_UK: &str = r#"<policyDefinitionResources><resources><stringTable>
      <string id="TurnOff">Вимкнути зразок</string><string id="Inner">Усередині</string>
    </stringTable></resources></policyDefinitionResources>"#;

    fn sample() -> CatalogFile {
        let mut t = Templates::default();
        read_pair(ADMX, &[("en-US".into(), ADML.into()), ("uk-UA".into(), ADML_UK.into())], "sample.admx", &mut t).unwrap();
        to_catalog_file(&t, "test", "Sample templates", "26300", "now")
    }

    #[test]
    fn templates_become_rules_and_groups() {
        let f = sample();
        assert_eq!(f.groups.len(), 3, "{:?}", f.groups);
        assert_eq!(f.groups[0].parent.as_deref(), Some(ROOT_POLICIES));
        assert_eq!(f.groups[1].title, "Sample & tests");
        let ids: Vec<&str> = f.rules.iter().map(|r| r.id.as_str()).collect();
        assert_eq!(
            ids,
            vec![
                "admx.sample.turnoff",
                "admx.sample.turnoff.off",
                "admx.sample.level",
                "admx.sample.level.off",
                "admx.sample.listed",
                "admx.sample.listed.off",
                "admx.sample.lines",
                "admx.sample.lines.off",
            ]
        );
        let off = &f.rules[0];
        assert_eq!(off.summary, "Explains A lot.");
        assert!(matches!(&off.actions[0], Action::Reg { path, name, value, .. } if path == "HKLM:\\Software\\Policies\\Test" && name == "Off" && value == &Value::from(1)));
        assert_eq!(off.conflicts, vec!["admx.sample.turnoff.off".to_owned()]);
        let level = &f.rules[2];
        assert_eq!(level.phase, Phase::DefaultUser);
        assert!(matches!(&level.actions[0], Action::Reg { path, value, .. } if path.starts_with("DU:\\") && value == &Value::from("{e1}")));
        assert!(matches!(&level.params["e1"], Param::Enum { default, values, .. } if default == &Value::from(2) && values.len() == 2));
        assert!(matches!(&level.params["e2"], Param::Int { default: 10, min: 1, max: 50, .. }));
        let listed = f.rules.iter().find(|r| r.id == "admx.sample.listed").unwrap();
        assert!(matches!(&listed.actions[0],
            Action::RegList { param, names: ListNames::Items, additive: false, kind: RegKind::String, .. } if param == "e1"));
        assert!(matches!(&listed.params["e1"], Param::List { .. }));
        let off = f.rules.iter().find(|r| r.id == "admx.sample.listed.off").unwrap();
        assert!(matches!(&off.actions[0], Action::RegList { param, .. } if param.is_empty()), "Disabled clears the list");
        assert!(f.skipped.iter().any(|(p, why)| p == "sample:Unsafe" && why.contains("unsafe")));
        assert_eq!(f.texts["uk"].rules["admx.sample.turnoff"].0, "Вимкнути зразок");
        f.check().unwrap();
    }

    #[test]
    fn a_catalog_file_merges_and_crafted_files_are_refused() {
        let f = sample();
        let base = Catalog::embedded().unwrap();
        let (merged, notes) = merge(&base, &[f.clone(), f.clone()]).unwrap();
        assert_eq!(merged.rules().len(), base.rules().len() + 8);
        assert_eq!(notes.len(), 1, "the second copy is left out");
        assert_eq!(merged.root_of("admx.test"), Some(ROOT_POLICIES));
        let mut texts = CatalogTexts::embedded().unwrap();
        merge_texts(&mut texts, std::slice::from_ref(&f));
        let uk = texts.iter().find(|t| t.code == "uk").unwrap();
        assert!(uk.rules.contains_key("admx.sample.turnoff"));
        let off = uk.rules["admx.sample.turnoff.off"].title.as_deref().unwrap_or_default();
        assert!(off.ends_with(&format!(": {}", uk.disabled)), "{off}");
        let mut bad = f.clone();
        bad.rules[0].actions.push(Action::Ps { script: crate::catalog::Lines::One("x".into()) });
        assert!(bad.check().is_err(), "an import never brings a script");
        let mut bad = f.clone();
        if let Action::Reg { path, .. } = &mut bad.rules[0].actions[0] {
            *path = "HKCU:\\X".into();
        }
        assert!(bad.check().is_err(), "only the machine and the default user branches");
        let mut bad = f;
        if let Action::Reg { value, .. } = &mut bad.rules[0].actions[0] {
            *value = Value::from("x]]>y");
        }
        assert!(bad.check().is_err(), "nothing that closes a CDATA section");
    }

    #[test]
    fn a_policy_a_built_in_rule_writes_is_linked_to_it() {
        let f = sample();
        let base = Catalog::embedded().unwrap();
        let mut builtin = base.rules()[0].clone();
        builtin.id = "test.turn-off".to_owned();
        builtin.requires.clear();
        builtin.conflicts.clear();
        builtin.params.clear();
        builtin.actions = vec![Action::Reg {
            path: "HKLM:\\SOFTWARE\\Policies\\Test".to_owned(),
            name: "off".to_owned(),
            kind: RegKind::DWord,
            value: Value::from(1),
            default: None,
            why: String::new(),
        }];
        let mut rules = base.rules().to_vec();
        rules.push(builtin.clone());
        let with_rule = Catalog::from_parts(base.version(), base.groups().to_vec(), rules).unwrap();
        let (catalog, _) = merge(&with_rule, &[f]).unwrap();
        let mut profile = crate::profile::Profile::from_defaults("test", &catalog);
        profile.set_on("test.turn-off", true);
        let r = crate::resolver::resolve(&catalog, &profile);
        assert_eq!(r.linked_by("admx.sample.turnoff"), Some("test.turn-off"));
        assert!(r.shown_on("admx.sample.turnoff") && !r.in_effect("admx.sample.turnoff"));
        assert_eq!(r.linked_by("admx.sample.turnoff.off"), None, "it writes 0");
        assert_eq!(r.linked_by("admx.sample.level"), None, "a policy with parameters");
        profile.set_on("test.turn-off", false);
        let r = crate::resolver::resolve(&catalog, &profile);
        assert_eq!(r.linked_by("admx.sample.turnoff"), None, "the built-in rule is off");
    }

    #[test]
    fn lists_of_templates_are_built_applied_and_checked() {
        let base = Catalog::embedded().unwrap();
        let (catalog, _) = merge(&base, &[sample()]).unwrap();
        let mut profile = crate::profile::Profile::from_defaults("test", &catalog);
        let set = |profile: &mut crate::profile::Profile, id: &str, name: &str, items: &[&str]| {
            profile.set_on(id, true);
            let choice = profile.rules.entry(id.to_owned()).or_default();
            choice.params.insert(name.to_owned(), Value::from(items.to_vec()));
        };
        set(&mut profile, "admx.sample.listed", "e1", &["a.example", " ", "b.example"]);
        set(&mut profile, "admx.sample.lines", "e1", &["one", "two"]);
        set(&mut profile, "admx.sample.lines", "e2", &["x", "y"]);
        let listed = catalog.rule("admx.sample.listed").unwrap();
        let params = crate::render::params_of(&catalog, &profile, listed);
        let line = crate::render::script_lines(listed, &listed.actions[0], &params, listed.phase).unwrap();
        assert_eq!(
            line,
            vec![
                "Set-RegList -Path 'HKLM:\\Software\\Policies\\Test\\L' -Type String -Names @('a.example','b.example') -Values @('a.example','b.example')"
                    .to_owned()
            ]
        );
        let lines = catalog.rule("admx.sample.lines").unwrap();
        let params = crate::render::params_of(&catalog, &profile, lines);
        let list = crate::render::script_lines(lines, &lines.actions[0], &params, lines.phase).unwrap();
        assert_eq!(
            list[0],
            "Set-RegList -Path 'HKLM:\\Software\\Policies\\Test\\N' -Type String -Names @('1','2') -Values @('x','y')"
        );
        let multi = crate::render::script_lines(lines, &lines.actions[1], &params, lines.phase).unwrap();
        assert!(multi[0].contains("-Name 'Lines' -Type MultiString") && multi[0].contains("'one'"), "{multi:?}");
        let ids = vec!["admx.sample.lines".to_owned()];
        let plan = crate::apply::plan_apply(&catalog, &profile, &ids, "n");
        assert!(plan.steps.iter().any(|s| matches!(&s.op,
            crate::apply::Op::SetList { names, values, additive: false, .. } if names == &["1", "2"] && values == &["x", "y"])));
        let checks = crate::thispc::plan(&catalog, &profile, &ids);
        let key = "HKLM:\\Software\\Policies\\Test\\N";
        let mut m = crate::thispc::MemoryMachine::default();
        m.set(key, "1", crate::thispc::RegData::Str("x".into()));
        m.set(key, "2", crate::thispc::RegData::Str("y".into()));
        let status = |m: &crate::thispc::MemoryMachine| {
            let report = crate::thispc::run(m, &checks, false, "now");
            report.results.iter().find(|r| r.check.ends_with("(list)")).map(|r| r.status)
        };
        assert_eq!(status(&m), Some(crate::thispc::Status::Ok));
        m.set(key, "3", crate::thispc::RegData::Str("z".into()));
        assert_eq!(status(&m), Some(crate::thispc::Status::Differs), "a value outside the list");
        let word = format!("caf{}", char::from_u32(0xe9).unwrap());
        set(&mut profile, "admx.sample.lines", "e2", &[word.as_str()]);
        let resolution = crate::resolver::resolve(&catalog, &profile);
        let issues = crate::validate::check(&catalog, &profile, &resolution);
        assert!(issues.iter().any(|i| i.text.contains("ASCII")), "{issues:?}");
        let items = |list: &[&str]| list.iter().map(|t| (*t).to_owned()).collect::<Vec<_>>();
        assert!(crate::render::list_entries(ListNames::Pairs, "", &items(&["a=1", "b"])).is_err());
        assert!(crate::render::list_entries(ListNames::Items, "", &items(&["a", "A"])).is_err());
        assert_eq!(
            crate::render::list_entries(ListNames::Numbered, "item", &items(&["x"])).unwrap(),
            (vec!["item1".to_owned()], vec!["x".to_owned()])
        );
    }

    #[test]
    fn xml_without_dtd_or_unknown_entities() {
        assert!(parse_xml("<!DOCTYPE x [<!ENTITY a \"b\">]><x>&a;</x>").is_err());
        assert!(parse_xml("<x>&unknown;</x>").is_err());
        assert_eq!(parse_xml("<x>a &lt; b</x>").unwrap().text, "a < b");
        let deep = format!("{}{}", "<a>".repeat(80), "</a>".repeat(80));
        assert!(parse_xml(&deep).is_err());
        assert!(safe_key("Software\\Policies\\X") && !safe_key("Software\\..\\X") && !safe_key("A\\\\B"));
    }
}

#[cfg(all(test, windows))]
mod windows_tests {
    use super::*;

    #[test]
    fn the_templates_of_this_windows_import_check_and_merge() {
        let dir = std::path::Path::new(r"C:\Windows\PolicyDefinitions");
        if !dir.is_dir() {
            return;
        }
        let t = read_folder(dir);
        assert!(t.policies() > 1000, "{} policies", t.policies());
        let f = to_catalog_file(&t, "system", "Policy templates of this Windows", "", "now");
        f.check().unwrap();
        assert!(f.rules.len() > 1000);
        let base = Catalog::embedded().unwrap();
        let (merged, _) = merge(&base, &[f]).unwrap();
        assert!(merged.rules().len() > base.rules().len() + 1000);
    }
}
