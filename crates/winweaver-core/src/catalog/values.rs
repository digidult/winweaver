//! The values of the actions and parameters of a rule read one way: as a text, a whole number,
//! the items of a list or the lines of a multi-line value. The scripts, the registry files, the
//! audit and the native apply all read them here.

use serde_json::Value;

use super::ListNames;

/// The text of a value: a string as it is, null as nothing, anything else as its JSON.
pub fn text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// The whole number of a value: a JSON number, a text of digits, a boolean as 1 or 0; `None`
/// for anything else.
pub fn number(value: &Value) -> Option<i64> {
    match value {
        Value::Number(n) => n.as_i64().or_else(|| n.as_u64().map(|u| u as i64)),
        Value::String(s) => s.trim().parse().ok(),
        Value::Bool(b) => Some(i64::from(*b)),
        _ => None,
    }
}

/// The items of a list value: the texts of an array, trimmed, without the empty ones.
pub fn list_items(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// The lines of a multi-line value as a `REG_MULTI_SZ` holds them: the texts of an array, an
/// empty text as no line, any other value as one line.
pub fn multi_items(value: &Value) -> Vec<String> {
    match value {
        Value::Array(items) => items.iter().map(text).collect(),
        Value::String(s) if s.is_empty() => Vec::new(),
        other => vec![text(other)],
    }
}

/// The value names and the data a `reg-list` writes for its items; an error for a pair without
/// `=` or a name that comes twice (Windows compares value names without regard to case).
pub fn list_entries(
    names: ListNames,
    prefix: &str,
    items: &[String],
) -> Result<(Vec<String>, Vec<String>), String> {
    let mut out_names: Vec<String> = Vec::new();
    let mut out_values = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let (name, value) = match names {
            ListNames::Items => (item.clone(), item.clone()),
            ListNames::Numbered => (format!("{prefix}{}", i + 1), item.clone()),
            ListNames::Pairs => match item.split_once('=') {
                Some((n, v)) if !n.trim().is_empty() => (n.trim().to_owned(), v.trim().to_owned()),
                _ => return Err(format!("the item {item} is not name=value")),
            },
        };
        if out_names.iter().any(|n| n.eq_ignore_ascii_case(&name)) {
            return Err(format!("the value name {name} comes twice"));
        }
        out_names.push(name);
        out_values.push(value);
    }
    Ok((out_names, out_values))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn texts_numbers_and_lists() {
        assert_eq!(text(&Value::from("a")), "a");
        assert_eq!(text(&Value::Null), "");
        assert_eq!(text(&Value::from(5)), "5");
        assert_eq!(number(&Value::from(5)), Some(5));
        assert_eq!(number(&Value::from(" 7 ")), Some(7));
        assert_eq!(number(&Value::from(true)), Some(1));
        assert_eq!(number(&Value::from("x")), None);
        assert_eq!(number(&Value::from(1.5)), None);
        let list = Value::from(vec![" a ", "", "b"]);
        assert_eq!(list_items(Some(&list)), vec!["a", "b"]);
        assert!(list_items(None).is_empty());
        assert_eq!(multi_items(&list), vec![" a ", "", "b"]);
        assert!(multi_items(&Value::from("")).is_empty());
        assert_eq!(multi_items(&Value::from("one")), vec!["one"]);
        let items = vec!["a=1".to_owned(), "B=2".to_owned()];
        let (n, v) = list_entries(ListNames::Pairs, "", &items).unwrap();
        assert_eq!(n, vec!["a", "B"]);
        assert_eq!(v, vec!["1", "2"]);
        assert!(list_entries(ListNames::Pairs, "", &["x".to_owned()]).is_err());
        let twice = vec!["a=1".to_owned(), "A=2".to_owned()];
        assert!(list_entries(ListNames::Pairs, "", &twice).is_err());
        let (n, _) = list_entries(ListNames::Numbered, "p", &items).unwrap();
        assert_eq!(n, vec!["p1", "p2"]);
    }
}
