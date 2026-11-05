//! The view of a binary file: the tree of its description on the left and its bytes on the right
//! (a node selected marks its bytes, a byte clicked selects the smallest node that holds it), or
//! the tree as a Markdown page drawn by W#, with the search and the settings of any page. A node of
//! a lazy field is read when it opens; Save the structure writes the tree as a `.wwbin`.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use wwrt::blocks::BlockFile;
use wwrt::egui;
use wwrt::plugin::{Host, Tool, View};
use wwrt::ww::Search;
use wwrt::ww::i18n::{tr, trf};
use wwrt::ww::mdview::{self, MdDoc};
use wwrt::ww::tree::{self, Check, Event, Row, Scroll};
use wwrt::ww::widgets;

use crate::engine::{self, Data, Limits, Tree};
use crate::format::Format;
use crate::{hex, wwbin};

/// The items of a node the tree lists; the text view lists them all.
const LISTED: usize = 10_000;
/// The nodes the text view writes.
const WRITTEN: usize = 20_000;

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Bytes,
    Text,
}

pub struct BinaryView {
    name: String,
    path: PathBuf,
    format: &'static Format,
    /// The SHA-256 of the text of the description.
    description: String,
    limits: Limits,
    tree: Tree,
    source: BlockFile,
    per_row: u64,
    open: HashSet<usize>,
    selected: usize,
    /// The selection moved with the keys (follow) or by a click on a byte (reveal).
    follow: bool,
    reveal: bool,
    /// The offset the bytes scroll to at the next frame.
    to_offset: Option<u64>,
    mode: Mode,
    page: Option<MdDoc>,
    /// What the last save did.
    saved: Option<String>,
}

/// The file a structure is saved to by default: the name of the file with `.wwbin` added.
pub fn wwbin_path(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".wwbin");
    path.with_file_name(name)
}

/// A text of a page with the signs of Markdown escaped.
fn escaped(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if "\\`*_[]<>#|".contains(c) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

/// The tree as a Markdown page: a list of every node with its value, offset and size.
pub fn markdown(tree: &Tree, title: &str, format: &str) -> String {
    let mut out = format!(
        "# {}\n\n{}\n\n",
        escaped(title),
        escaped(&trf(
            "{format}, {nodes} nodes",
            &[("format", &format), ("nodes", &tree.nodes.len())]
        ))
    );
    let mut stack = vec![(0usize, 0usize)];
    let mut written = 0;
    while let Some((n, depth)) = stack.pop() {
        if written == WRITTEN {
            out.push_str(&format!(
                "\n{}\n",
                escaped(&trf(
                    "The page shows the first {n} nodes.",
                    &[("n", &WRITTEN)]
                ))
            ));
            break;
        }
        written += 1;
        let node = &tree.nodes[n];
        let value = tree.value(n);
        let place = format!("0x{:X}, {}", node.offset, node.size);
        let label = escaped(&tree.label(n));
        let line = match node.data {
            Data::Struct(_) | Data::Array(_) => format!("**{label}** {}", escaped(&value)),
            _ if value.is_empty() => label,
            _ => format!("{label}: `{}`", value.replace('`', "'")),
        };
        out.push_str(&format!("{}- {line} ({place})\n", "  ".repeat(depth)));
        for c in node.children.iter().rev() {
            stack.push((*c, depth + 1));
        }
    }
    out
}

impl BinaryView {
    pub fn new(
        path: &Path,
        format: &'static Format,
        description: String,
        mut tree: Tree,
        source: BlockFile,
        per_row: u64,
    ) -> Self {
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        // The root is the file: its name, its format and all its bytes.
        let root = &mut tree.nodes[0];
        root.name = name.clone();
        root.data = Data::Struct(format.name.clone());
        root.size = source.len();
        BinaryView {
            name,
            path: path.to_path_buf(),
            format,
            description,
            limits: Limits::default(),
            tree,
            source,
            per_row,
            open: HashSet::from([0]),
            selected: 0,
            follow: false,
            reveal: false,
            to_offset: None,
            mode: Mode::Bytes,
            page: None,
            saved: None,
        }
    }

    /// Reads a node not read yet; the page is written again with it.
    fn expand(&mut self, node: usize) {
        if self.tree.nodes[node].pending {
            engine::expand(
                &mut self.tree,
                self.format,
                &mut self.source,
                node,
                self.limits,
            );
            self.tree.nodes[0].size = self.source.len();
            self.page = None;
        }
    }

    /// Writes the structure to a file, every node not read yet read first.
    pub fn save_to(&mut self, path: &Path) -> Result<(), String> {
        engine::expand_all(&mut self.tree, self.format, &mut self.source, self.limits);
        self.tree.nodes[0].size = self.source.len();
        self.page = None;
        let doc = wwbin::document(
            &self.tree,
            &mut self.source,
            &self.name,
            &self.format.name,
            &self.description,
        );
        let text = wwrt::serde_json::to_string_pretty(&doc).map_err(|e| e.to_string())? + "\n";
        wwrt::ww::dialogs::write_file(path, text.as_bytes())
    }

    /// Asks where to save the structure, next to the file by default, and saves it there.
    fn save(&mut self) {
        let default = wwbin_path(&self.path);
        let mut dialog = wwrt::ww::rfd::FileDialog::new()
            .add_filter(tr("Structure of a file"), &["wwbin"])
            .set_file_name(default.file_name().unwrap_or_default().to_string_lossy());
        if let Some(dir) = default.parent() {
            dialog = dialog.set_directory(dir);
        }
        let Some(path) = dialog.save_file() else {
            return;
        };
        self.saved = Some(match self.save_to(&path) {
            Ok(()) => trf("Saved {path}", &[("path", &path.display())]),
            Err(why) => trf(
                "{path} was not saved: {why}",
                &[("path", &path.display()), ("why", &why)],
            ),
        });
    }

    /// The rows of the open nodes, in the order of the tree, with a row for the items of an
    /// array the tree leaves out after the ones it lists.
    fn rows(&self) -> Vec<Row<usize>> {
        enum Next {
            Node(usize, usize),
            More(usize, usize, usize),
        }
        let mut out = Vec::new();
        let mut stack = vec![Next::Node(0, 0)];
        while let Some(next) = stack.pop() {
            let (n, depth) = match next {
                Next::Node(n, depth) => (n, depth),
                Next::More(n, depth, left) => {
                    out.push(Row {
                        key: n,
                        depth,
                        title: trf("{n} more items in the text view", &[("n", &left)]),
                        right: String::new(),
                        check: Check::None,
                        open: None,
                        muted: true,
                        icon: None,
                        switch: None,
                    });
                    continue;
                }
            };
            let node = &self.tree.nodes[n];
            let opens = !node.children.is_empty() || node.pending;
            let open = opens.then(|| self.open.contains(&n));
            let value = self.tree.value(n);
            let label = self.tree.label(n);
            out.push(Row {
                key: n,
                depth,
                title: if value.is_empty() {
                    label
                } else {
                    format!("{label}: {value}")
                },
                right: format!("0x{:X}", node.offset),
                check: Check::None,
                open,
                muted: false,
                icon: node.error.then_some("\u{26A0}"),
                switch: None,
            });
            if open == Some(true) {
                let listed = node.children.len().min(LISTED);
                if node.children.len() > listed {
                    stack.push(Next::More(n, depth + 1, node.children.len() - listed));
                }
                for c in node.children[..listed].iter().rev() {
                    stack.push(Next::Node(*c, depth + 1));
                }
            }
        }
        out
    }

    /// Selects a node and opens the ones above it.
    fn select(&mut self, node: usize) {
        self.selected = node;
        for n in self.tree.path(node) {
            if n != node {
                self.open.insert(n);
            }
        }
    }

    fn page(&mut self) -> &mut MdDoc {
        let (tree, name, format) = (&self.tree, &self.name, &self.format.name);
        self.page
            .get_or_insert_with(|| MdDoc::new(name.clone(), None, markdown(tree, name, format)))
    }

    fn bytes(&mut self, ui: &mut egui::Ui) {
        let rows = self.rows();
        let selected = rows.iter().position(|r| r.key == self.selected && !r.muted);
        let scroll = Scroll::wanted(std::mem::take(&mut self.reveal), self.follow);
        let id = egui::Id::new("wv-binary-tree");
        let output = widgets::side_panel(ui, "wv-binary-tree", 460.0, |ui| {
            tree::show(ui, id, &rows, selected, scroll)
        });
        self.follow = output.moved;
        for event in output.events {
            match event {
                Event::Select(n) => {
                    self.selected = n;
                    self.to_offset = Some(self.tree.nodes[n].offset);
                }
                Event::Expand(n, open) => {
                    if open {
                        self.expand(n);
                        self.open.insert(n);
                    } else {
                        self.open.remove(&n);
                    }
                }
                Event::Toggle(_) | Event::Switch(..) => {}
            }
        }
        let node = &self.tree.nodes[self.selected];
        let marked = (node.size > 0).then(|| node.offset..node.offset + node.size);
        let per_row = self.per_row;
        let to = self.to_offset.take();
        let clicked = egui::CentralPanel::default()
            .show(ui, |ui| {
                hex::show(ui, &mut self.source, per_row, marked, to).clicked
            })
            .inner;
        if let Some(at) = clicked
            && let Some(n) = self.tree.at(at)
        {
            self.select(n);
            self.reveal = true;
        }
    }
}

impl View for BinaryView {
    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        match self.mode {
            Mode::Bytes => self.bytes(ui),
            Mode::Text => {
                let (wrap, width) = (host.wrap_lines, host.page_width);
                let doc = self.page();
                egui::CentralPanel::default().show(ui, |ui| {
                    mdview::md_view(ui, doc, wrap, width);
                });
            }
        }
    }

    fn tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                id: "bytes",
                icon: "\u{1F522}",
                label: "Tree and bytes",
            },
            Tool {
                id: "text",
                icon: "\u{1F4C4}",
                label: "Text",
            },
            Tool {
                id: "save",
                icon: "\u{1F4BE}",
                label: "Save the structure",
            },
        ]
    }

    fn act(&mut self, id: &str) {
        match id {
            "bytes" => self.mode = Mode::Bytes,
            "text" => {
                self.page();
                self.mode = Mode::Text;
            }
            "save" => self.save(),
            _ => {}
        }
    }

    fn search(&mut self) -> Option<&mut Search> {
        match self.mode {
            Mode::Text => self.page.as_mut().map(|p| &mut p.search),
            Mode::Bytes => None,
        }
    }

    fn status(&self) -> String {
        let mut line = trf(
            "{format}, {nodes} nodes, {size} bytes",
            &[
                ("format", &self.format.name),
                ("nodes", &self.tree.nodes.len()),
                ("size", &self.source.len()),
            ],
        );
        if self.tree.errors > 0 {
            line.push_str(", ");
            line.push_str(&trf(
                "{n} nodes not read whole",
                &[("n", &self.tree.errors)],
            ));
        }
        if let Some(saved) = &self.saved {
            line.push_str("; ");
            line.push_str(saved);
        }
        let node = &self.tree.nodes[self.selected];
        line.push_str("; ");
        line.push_str(&trf(
            "{name} at {offset}, {size} bytes",
            &[
                ("name", &self.tree.label(self.selected)),
                ("offset", &format!("0x{:X}", node.offset)),
                ("size", &node.size),
            ],
        ));
        line
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Limits, read};
    use crate::format::Format;

    #[test]
    fn the_text_view_lists_every_node() {
        let description = r#"{ "format": "wwformat", "version": 1, "name": "Pair",
            "extensions": [], "root": "file", "types": { "file": { "fields": [
                { "name": "a_b", "type": "u8" },
                { "name": "items", "type": "u8", "count": "2" } ] } } }"#;
        let format = Format::parse(description).unwrap();
        let t = read(&format, &mut vec![5u8, 1, 2], Limits::default());
        let page = markdown(&t, "pair.bin", "Pair");
        assert!(page.starts_with("# pair.bin\n"), "{page}");
        assert!(page.contains("- **file** file (0x0, 3)\n"), "{page}");
        assert!(
            page.contains("  - a\\_b: `5` (0x0, 1)\n"),
            "the names are escaped: {page}"
        );
        assert!(page.contains("    - \\[1\\]: `2` (0x2, 1)\n"), "{page}");
    }
}
