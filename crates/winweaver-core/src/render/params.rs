//! The placeholders of the texts of the actions and the parameter values that fill them.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::catalog::{Catalog, Rule, values};
use crate::profile::Profile;

use super::RenderError;

/// `{name}` placeholders replaced by the parameter values; other braces stay.
pub fn substitute(text: &str, params: &BTreeMap<String, Value>) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        match after.find('}') {
            Some(end) if params.contains_key(&after[..end]) => {
                out.push_str(&values::text(&params[&after[..end]]));
                rest = &after[end + 1..];
            }
            _ => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

/// The items of the list parameter of a `reg-list`; none for an empty parameter name.
pub(crate) fn list_values(params: &BTreeMap<String, Value>, param: &str) -> Vec<String> {
    if param.is_empty() {
        return Vec::new();
    }
    values::list_items(params.get(param))
}

/// A value of an action with its placeholders filled in: a value that is one list parameter
/// alone becomes the list of its items (the lines of a multi-line value).
pub(crate) fn fill_value(value: &Value, params: &BTreeMap<String, Value>) -> Value {
    match value {
        Value::String(text) => {
            if let Some(inner) = text.strip_prefix('{').and_then(|t| t.strip_suffix('}'))
                && let Some(list @ Value::Array(_)) = params.get(inner)
            {
                return Value::from(values::list_items(Some(list)));
            }
            Value::String(substitute(text, params))
        }
        Value::Array(items) => Value::Array(items.iter().map(|v| fill_value(v, params)).collect()),
        other => other.clone(),
    }
}

/// A template with its `{{name}}` slots filled, in one pass over the template: a value is put
/// in as it is and never read again, so a value that holds `{{name}}` (a profile named so, a
/// line of a script) is never filled as a slot. A slot of the template without a value is an
/// error.
pub(crate) fn fill(template: &str, slots: &[(&str, &str)]) -> Result<String, RenderError> {
    let template = template.replace("\r\n", "\n");
    let values: usize = slots.iter().map(|(_, value)| value.len()).sum();
    let mut out = String::with_capacity(template.len() + values);
    let mut rest = template.as_str();
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after.find("}}");
        let slot = end.and_then(|end| slots.iter().find(|(name, _)| *name == &after[..end]));
        match (end, slot) {
            (Some(end), Some((_, value))) => {
                out.push_str(value);
                rest = &after[end + 2..];
            }
            _ => {
                let end = end.map_or(rest.len(), |e| start + 2 + e + 2);
                return Err(RenderError::Template(format!(
                    "a slot stayed unfilled: {}",
                    &rest[start..end]
                )));
            }
        }
    }
    out.push_str(rest);
    Ok(out)
}

/// The parameter values of a rule in the profile (the defaults where the profile has none).
pub(crate) fn params_of(
    catalog: &Catalog,
    profile: &Profile,
    rule: &Rule,
) -> BTreeMap<String, Value> {
    rule.params
        .keys()
        .filter_map(|n| profile.param(catalog, &rule.id, n).map(|v| (n.clone(), v)))
        .collect()
}
