//! The pieces of the passes of the answer file: nested settings, synchronous commands and
//! components.

use super::text::xml_escape;
use super::{MAX_COMMAND, RenderError};

/// A nested XML setting of a component: the element names from the component down.
#[derive(Debug, Clone)]
pub(super) struct Setting {
    pub(super) path: Vec<String>,
    pub(super) value: String,
}

/// Nested elements of settings that share a component, in the order given.
pub(super) fn setting_lines(
    settings: &[Setting],
    indent: usize,
) -> Result<Vec<String>, RenderError> {
    #[derive(Default)]
    struct Node {
        value: Option<String>,
        children: Vec<(String, Node)>,
    }
    fn insert(node: &mut Node, path: &[String], value: &str) -> Result<(), RenderError> {
        let Some((first, rest)) = path.split_first() else {
            return Ok(());
        };
        let child = match node.children.iter_mut().position(|(n, _)| n == first) {
            Some(i) => &mut node.children[i].1,
            None => {
                node.children.push((first.clone(), Node::default()));
                &mut node.children.last_mut().expect("just pushed").1
            }
        };
        if rest.is_empty() {
            if !child.children.is_empty() || child.value.is_some() {
                return Err(RenderError::Template(format!(
                    "the setting {} is both a value and a parent, or set twice",
                    path.join("/")
                )));
            }
            child.value = Some(value.to_owned());
            Ok(())
        } else {
            if child.value.is_some() {
                return Err(RenderError::Template(format!(
                    "the setting {} is both a value and a parent",
                    path.join("/")
                )));
            }
            insert(child, rest, value)
        }
    }
    fn write(node: &Node, indent: usize, out: &mut Vec<String>) {
        let pad = " ".repeat(indent);
        for (name, child) in &node.children {
            match &child.value {
                Some(v) => out.push(format!("{pad}<{name}>{}</{name}>", xml_escape(v))),
                None => {
                    out.push(format!("{pad}<{name}>"));
                    write(child, indent + 2, out);
                    out.push(format!("{pad}</{name}>"));
                }
            }
        }
    }
    let mut root = Node::default();
    for s in settings {
        insert(&mut root, &s.path, &s.value)?;
    }
    let mut out = Vec::new();
    write(&root, indent, &mut out);
    Ok(out)
}

/// A command of a pass.
#[derive(Debug, Clone)]
pub(super) struct Command {
    pub(super) description: String,
    pub(super) path: String,
}

pub(super) fn command_lines(
    commands: &[Command],
    indent: usize,
) -> Result<Vec<String>, RenderError> {
    let pad = " ".repeat(indent);
    let mut out = Vec::new();
    if commands.is_empty() {
        return Ok(out);
    }
    out.push(format!("{pad}<RunSynchronous>"));
    for (i, c) in commands.iter().enumerate() {
        let len = c.path.chars().count();
        if len > MAX_COMMAND {
            return Err(RenderError::CommandTooLong {
                what: c.description.clone(),
                len,
                command: c.path.clone(),
            });
        }
        out.push(format!("{pad}  <RunSynchronousCommand wcm:action=\"add\">"));
        out.push(format!("{pad}    <Order>{}</Order>", i + 1));
        out.push(format!(
            "{pad}    <Description>{}</Description>",
            xml_escape(&c.description)
        ));
        out.push(format!("{pad}    <Path>{}</Path>", xml_escape(&c.path)));
        out.push(format!("{pad}  </RunSynchronousCommand>"));
    }
    out.push(format!("{pad}</RunSynchronous>"));
    Ok(out)
}

pub(super) fn component(name: &str, arch: &str, body: &[String]) -> Vec<String> {
    let mut out = vec![format!(
        "    <component name=\"{name}\" processorArchitecture=\"{arch}\" publicKeyToken=\"31bf3856ad364e35\" language=\"neutral\" versionScope=\"nonSxS\">"
    )];
    out.extend(body.iter().cloned());
    out.push("    </component>".to_owned());
    out
}
