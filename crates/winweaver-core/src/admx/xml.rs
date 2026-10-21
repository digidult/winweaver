//! A small tree of XML elements: what the ADMX and ADML parsers walk.

use quick_xml::events::Event;
use quick_xml::reader::Reader;

use super::DEPTH_MAX;

#[derive(Debug, Default)]
pub(super) struct Node {
    pub(super) name: String,
    pub(super) attrs: Vec<(String, String)>,
    pub(super) children: Vec<Node>,
    pub(super) text: String,
}

impl Node {
    pub(super) fn attr(&self, key: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    pub(super) fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|c| c.name == name)
    }

    pub(super) fn all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> + 'a {
        self.children.iter().filter(move |c| c.name == name)
    }
}

/// Parses XML text into a tree; a DTD, an unknown entity or nesting deeper than the limit is an
/// error.
pub(super) fn parse_xml(text: &str) -> Result<Node, String> {
    let mut reader = Reader::from_str(text);
    let mut stack: Vec<Node> = vec![Node::default()];
    loop {
        let event = reader
            .read_event()
            .map_err(|e| format!("not well formed: {e}"))?;
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
    root.children
        .pop()
        .ok_or_else(|| "an empty document".to_owned())
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
        let value = a
            .unescape_value()
            .map_err(|e| format!("an attribute: {e}"))?
            .into_owned();
        node.attrs.push((key, value));
    }
    Ok(node)
}
