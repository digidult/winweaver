//! The view of a binary file: the tree of its description on the left and its bytes on the right
//! (a node selected marks its bytes, a byte clicked selects the smallest node that holds it), or
//! the tree as a Markdown page drawn by W#, with the search and the settings of any page, or the
//! content of the node selected shown by the plugin of its type in place of its bytes (a manifest
//! as a text, an icon as an image: `crate::content`). A node of a lazy field is read when it
//! opens; Save the structure writes the tree as a `.wwbin`.

use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};

use wwrt::blocks::BlockFile;
use wwrt::egui;
use wwrt::inner::Inner;
use wwrt::mime;
use wwrt::plugin::{Host, Request, Tool, View};
use wwrt::ww::i18n::{tr, trf};
use wwrt::ww::mdview::{self, MdDoc};
use wwrt::ww::tree::{self, Check, Event, Row, Scroll};
use wwrt::ww::widgets;
use wwrt::ww::{Search, icons};

use crate::compareview::Comparison;
use crate::diff::{self, DiffFile, Note};
use crate::engine::{self, Data, Limits, Tree};
use crate::format::Format;
use crate::{content, hex, wwbin};

/// The items of a node the tree lists; the text view lists them all.
const LISTED: usize = 10_000;
/// The nodes the text view writes.
const WRITTEN: usize = 20_000;

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Bytes,
    Text,
    Compare,
    /// The tree and the content of the node selected, by the plugin of its type.
    Content,
}

/// What a node holds for the plugin of its type: its node of bytes, the type it shows as, and the
/// type its description names.
#[derive(Clone)]
struct Found {
    raw: usize,
    mime: String,
    declared: Option<String>,
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
    /// The comparison with a structure saved, once there is one.
    compare: Option<Comparison>,
    /// The content of the node selected, looked for when the selection changes.
    found: Option<(usize, Option<Found>)>,
    /// The content shown: its node of bytes and its view, or why it does not show.
    shown: Option<(usize, Result<Inner, String>)>,
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

/// The rows of the open nodes, in the order of the tree, with a row for the items of an
/// array the tree leaves out after the ones it lists.
pub fn tree_rows(tree: &Tree, open: &HashSet<usize>) -> Vec<Row<usize>> {
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
        let node = &tree.nodes[n];
        let opens = !node.children.is_empty() || node.pending;
        let open = opens.then(|| open.contains(&n));
        let value = tree.value(n);
        let label = tree.label(n);
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
            compare: None,
            found: None,
            shown: None,
        }
    }

    /// The same view under a name of its own: bytes inside another file have no path.
    pub fn named(mut self, name: &str) -> Self {
        name.clone_into(&mut self.name);
        name.clone_into(&mut self.tree.nodes[0].name);
        self
    }

    /// Selects the node a path of names or keys leads to (`*` for the first child), the nodes
    /// not read yet read on the way, and shows its content (the tests of a program).
    #[cfg(any(test, feature = "library"))]
    pub fn show_content(&mut self, path: &[&str]) -> Result<(), String> {
        let mut at = 0;
        for step in path {
            self.expand(at);
            let tree = &self.tree;
            at = tree.nodes[at]
                .children
                .iter()
                .copied()
                .find(|c| {
                    let n = &tree.nodes[*c];
                    *step == "*" || n.name == *step || n.key.as_deref() == Some(*step)
                })
                .ok_or_else(|| format!("no {step} in {}", tree.label(at)))?;
        }
        self.select(at);
        self.mode = Mode::Content;
        Ok(())
    }

    /// The content of the node selected made a file of its type, whatever the plugins: its
    /// name, its type and its bytes.
    #[cfg(test)]
    pub fn selected_content(&mut self) -> Result<(String, String, Vec<u8>), String> {
        let raw = self.raw_of(self.selected).ok_or("no bytes")?;
        let declared = self.declared(raw);
        self.content_of(raw, declared)
    }

    /// Compares the file with a structure saved of it: every node of the file read first.
    pub fn compare_with(&mut self, structure: &Path) -> Result<(), String> {
        let saved = wwbin::read(structure)?;
        engine::expand_all(&mut self.tree, self.format, &mut self.source, self.limits);
        self.tree.nodes[0].size = self.source.len();
        self.page = None;
        let (tree, source) = (&self.tree, &mut self.source);
        let mut sha =
            |n: usize| wwbin::sha256(&mut *source, tree.nodes[n].offset, tree.nodes[n].size);
        let lines = diff::compare(&saved.tree, tree, &mut sha);
        self.compare = Some(Comparison::new(structure.to_path_buf(), saved, lines));
        self.mode = Mode::Compare;
        Ok(())
    }

    /// The notes of a comparison read from its `.wwbindiff`.
    pub fn set_notes(&mut self, notes: Vec<Note>, file: PathBuf) {
        if let Some(c) = &mut self.compare {
            c.notes = notes.into_iter().map(|n| (n.key.clone(), n)).collect();
            c.file = Some(file);
            c.dirty = false;
        }
    }

    /// Writes the comparison to a `.wwbindiff`: the file, the structure and the notes.
    pub fn save_comparison_to(&mut self, path: &Path) -> Result<(), String> {
        let Some(c) = &mut self.compare else {
            return Err(tr("no comparison").to_owned());
        };
        DiffFile::new(&self.path, &c.structure, path, &c.notes).write(path)?;
        c.file = Some(path.to_path_buf());
        c.dirty = false;
        Ok(())
    }

    /// Asks for the structure to compare the file with.
    fn ask_compare(&mut self) {
        let mut dialog =
            wwrt::ww::rfd::FileDialog::new().add_filter(tr("Structure of a file"), &["wwbin"]);
        if let Some(dir) = self.path.parent() {
            dialog = dialog.set_directory(dir);
        }
        let Some(path) = dialog.pick_file() else {
            return;
        };
        if let Err(why) = self.compare_with(&path) {
            self.saved = Some(trf(
                "{path} was not compared: {why}",
                &[("path", &path.display()), ("why", &why)],
            ));
        }
    }

    /// Asks where to save the comparison, its own file or next to the file by default.
    fn save_comparison(&mut self) {
        let default = self
            .compare
            .as_ref()
            .and_then(|c| c.file.clone())
            .unwrap_or_else(|| {
                let mut name = self.path.file_name().unwrap_or_default().to_os_string();
                name.push(".wwbindiff");
                self.path.with_file_name(name)
            });
        let mut dialog = wwrt::ww::rfd::FileDialog::new()
            .add_filter(tr("Comparison of structures"), &["wwbindiff"])
            .set_file_name(default.file_name().unwrap_or_default().to_string_lossy());
        if let Some(dir) = default.parent() {
            dialog = dialog.set_directory(dir);
        }
        let Some(path) = dialog.save_file() else {
            return;
        };
        self.saved = Some(match self.save_comparison_to(&path) {
            Ok(()) => trf("Saved {path}", &[("path", &path.display())]),
            Err(why) => trf(
                "{path} was not saved: {why}",
                &[("path", &path.display()), ("why", &why)],
            ),
        });
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
            // A structure read now may hold the bytes of the node selected.
            self.found = None;
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

    fn rows(&self) -> Vec<Row<usize>> {
        tree_rows(&self.tree, &self.open)
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

    /// The node of bytes of a node: itself, or for a structure its first field of bytes.
    fn raw_of(&self, node: usize) -> Option<usize> {
        let is_raw = |n: usize| {
            let n = &self.tree.nodes[n];
            n.data == Data::Raw && n.size > 0
        };
        if is_raw(node) {
            return Some(node);
        }
        self.tree.nodes[node]
            .children
            .iter()
            .copied()
            .find(|c| is_raw(*c))
    }

    /// The type the description names for a node: its own, or the one of the nearest node above.
    fn declared(&self, node: usize) -> Option<String> {
        let mut at = Some(node);
        while let Some(n) = at {
            if let Some(c) = &self.tree.nodes[n].content {
                return Some(c.clone());
            }
            at = self.tree.nodes[n].parent;
        }
        None
    }

    /// The content of the node selected that a plugin of the window opens, looked for once a
    /// selection.
    fn find(&mut self, host: &Host) {
        if self
            .found
            .as_ref()
            .is_some_and(|(n, _)| *n == self.selected)
        {
            return;
        }
        let found = self.raw_of(self.selected).and_then(|raw| {
            let declared = self.declared(raw);
            let node = &self.tree.nodes[raw];
            let mut first = vec![0u8; node.size.min(mime::SNIFFED as u64) as usize];
            let n = self.source.read_at(node.offset, &mut first).ok()?;
            first.truncate(n);
            let mime = content::shown_type(declared.as_deref(), &first)?;
            host.opens(&mime).then_some(Found {
                raw,
                mime,
                declared,
            })
        });
        self.found = Some((self.selected, found));
    }

    /// The content of the node selected, when there is one.
    fn found(&self) -> Option<&Found> {
        match &self.found {
            Some((n, Some(f))) if *n == self.selected => Some(f),
            _ => None,
        }
    }

    /// The name of the content of a node: the file and the keys of the nodes above it (the type,
    /// the name and the language of a resource), or the names of the node and its parent.
    fn content_name(&self, raw: usize) -> String {
        let path = self.tree.path(raw);
        let keys: Vec<&str> = path
            .iter()
            .filter_map(|n| self.tree.nodes[*n].key.as_deref())
            .collect();
        let inside = if keys.is_empty() {
            let parent = self.tree.nodes[raw].parent.unwrap_or(0);
            format!("{}.{}", self.tree.label(parent), self.tree.nodes[raw].name)
        } else {
            keys.join("/")
        };
        format!("{}, {inside}", self.name)
    }

    /// The bytes of a node, at most `content::LARGEST_MB`.
    fn node_bytes(&mut self, raw: usize) -> Result<Vec<u8>, String> {
        let node = &self.tree.nodes[raw];
        if node.size > content::LARGEST_MB << 20 {
            return Err(trf("larger than {mb} MB", &[("mb", &content::LARGEST_MB)]));
        }
        let mut bytes = vec![0u8; node.size as usize];
        let n = self
            .source
            .read_at(node.offset, &mut bytes)
            .map_err(|e| e.to_string())?;
        bytes.truncate(n);
        Ok(bytes)
    }

    /// The content made a file of its type: its name, its type and its bytes.
    fn content_file(&mut self, found: &Found) -> Result<(String, String, Vec<u8>), String> {
        self.content_of(found.raw, found.declared.clone())
    }

    /// The bytes of a node made a file of the type its description names or its first bytes
    /// show, with the images of a group of icons found by `member`.
    fn content_of(
        &mut self,
        raw: usize,
        declared: Option<String>,
    ) -> Result<(String, String, Vec<u8>), String> {
        let bytes = self.node_bytes(raw)?;
        let members = declared
            .as_deref()
            .and_then(|d| mime::parameter(d, "members"));
        let mut member = |id: u16| self.member(raw, members.as_deref()?, id);
        let (mime, bytes) = content::prepared(declared.as_deref(), bytes, &mut member)?;
        Ok((self.content_name(raw), mime, bytes))
    }

    /// The bytes of an image of a group of icons: the node of the same path as the directory, its
    /// first key made the type of the members (`ICON`), its second the id of the image, and the
    /// ones after kept where the image has them (the language), else its first; the nodes not
    /// read yet are read on the way.
    fn member(&mut self, raw: usize, members: &str, id: u16) -> Option<Vec<u8>> {
        let path = self.tree.path(raw);
        let id = id.to_string();
        let mut at = 0;
        let mut keyed = 0;
        for &n in path.iter().skip(1) {
            let (name, key) = {
                let node = &self.tree.nodes[n];
                (node.name.clone(), node.key.clone())
            };
            self.expand(at);
            let tree = &self.tree;
            let children = &tree.nodes[at].children;
            let with_key = |k: &str| {
                children.iter().copied().find(|c| {
                    tree.nodes[*c].key.as_deref().is_some_and(|key| {
                        // A number of 10 and more shows with its hexadecimal: `13 (0xD)`.
                        key == k || key.strip_prefix(k).is_some_and(|r| r.starts_with(" ("))
                    })
                })
            };
            at = match (key, keyed) {
                (Some(_), 0) => with_key(members)?,
                (Some(_), 1) => with_key(&id)?,
                (Some(k), _) => with_key(&k).or_else(|| children.first().copied())?,
                (None, _) => children
                    .iter()
                    .copied()
                    .find(|c| tree.nodes[*c].name == name)?,
            };
            if tree.nodes[n].key.is_some() {
                keyed += 1;
            }
        }
        (self.tree.nodes[at].data == Data::Raw)
            .then(|| self.node_bytes(at).ok())
            .flatten()
    }

    /// Asks where to save the node as a file, its content made a file of its type when it has one
    /// (an icon as an ICO file), else its bytes, and saves it.
    fn save_node(&mut self) {
        let raw = self.raw_of(self.selected).unwrap_or(self.selected);
        let found = self.found().filter(|f| f.raw == raw).cloned();
        let extension = match found.as_ref().map(|f| mime::canonical(&f.mime)).as_deref() {
            Some("image/png") => "png",
            Some("image/bmp") => "bmp",
            Some("image/x-icon") => "ico",
            Some("image/jpeg") => "jpg",
            Some("image/gif") => "gif",
            Some("application/xml") => "xml",
            Some("text/html") => "html",
            Some("text/plain") => "txt",
            _ => "bin",
        };
        let label: String = self
            .tree
            .label(raw)
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '_' })
            .collect();
        let name = format!("{}.{label}.{extension}", self.name);
        let mut dialog = wwrt::ww::rfd::FileDialog::new().set_file_name(name);
        if let Some(dir) = self.path.parent().filter(|d| !d.as_os_str().is_empty()) {
            dialog = dialog.set_directory(dir);
        }
        let Some(path) = dialog.save_file() else {
            return;
        };
        let written = match &found {
            Some(f) => self
                .content_file(f)
                .and_then(|(_, _, bytes)| wwrt::ww::dialogs::write_file(&path, &bytes)),
            None => self.write_range(raw, &path),
        };
        self.saved = Some(match written {
            Ok(()) => trf("Saved {path}", &[("path", &path.display())]),
            Err(why) => trf(
                "{path} was not saved: {why}",
                &[("path", &path.display()), ("why", &why)],
            ),
        });
    }

    /// Writes the bytes of a node to a file a block at a time.
    fn write_range(&mut self, node: usize, path: &Path) -> Result<(), String> {
        let (mut at, end) = {
            let n = &self.tree.nodes[node];
            (n.offset, n.offset + n.size)
        };
        let mut file = std::fs::File::create(path).map_err(|e| e.to_string())?;
        let mut block = vec![0u8; 1 << 20];
        while at < end {
            let want = ((end - at) as usize).min(block.len());
            let n = self
                .source
                .read_at(at, &mut block[..want])
                .map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            file.write_all(&block[..n]).map_err(|e| e.to_string())?;
            at += n as u64;
        }
        Ok(())
    }

    /// The content of the node selected by the plugin of its type, opened when the selection
    /// changes; a node without one shows a line and its bytes.
    fn content(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        let Some(found) = self.found().cloned() else {
            self.shown = None;
            ui.weak(tr("No plugin shows the bytes of this node."));
            self.hex(ui);
            return;
        };
        if self.shown.as_ref().map(|(n, _)| *n) != Some(found.raw) {
            let opened = self
                .content_file(&found)
                .and_then(|(name, mime, bytes)| host.open_inside(&name, &mime, bytes));
            self.shown = Some((found.raw, opened));
        }
        match &mut self.shown {
            Some((_, Ok(inner))) => inner.ui(ui, host),
            Some((_, Err(why))) => {
                let why = why.clone();
                ui.colored_label(ui.visuals().warn_fg_color, why);
                self.hex(ui);
            }
            None => {}
        }
    }

    /// The bytes of the file with the ones of the node selected marked.
    fn hex(&mut self, ui: &mut egui::Ui) {
        let node = &self.tree.nodes[self.selected];
        let marked = (node.size > 0).then(|| node.offset..node.offset + node.size);
        let to = self.to_offset.take();
        let clicked = hex::show(ui, &mut self.source, self.per_row, marked, to).clicked;
        if let Some(at) = clicked
            && let Some(n) = self.tree.at(at)
        {
            self.select(n);
            self.reveal = true;
        }
    }

    fn bytes(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        let rows = self.rows();
        let selected = rows.iter().position(|r| r.key == self.selected && !r.muted);
        let scroll = Scroll::wanted(std::mem::take(&mut self.reveal), self.follow);
        // An id of the view's own place: a binary file shown inside another keeps apart.
        let id = ui.make_persistent_id("wv-binary-tree");
        let output = widgets::side_panel(ui, id, 460.0, |ui| {
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
        self.find(host);
        if self.mode == Mode::Content {
            egui::CentralPanel::default()
                .frame(egui::Frame::new())
                .show(ui, |ui| self.content(ui, host));
        } else {
            egui::CentralPanel::default().show(ui, |ui| self.hex(ui));
        }
    }
}

impl View for BinaryView {
    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        match self.mode {
            Mode::Bytes | Mode::Content => self.bytes(ui, host),
            Mode::Text => {
                let (wrap, width) = (host.wrap_lines, host.page_width);
                let doc = self.page();
                egui::CentralPanel::default().show(ui, |ui| {
                    mdview::md_view(ui, doc, wrap, width);
                });
            }
            Mode::Compare => {
                let Some(c) = &mut self.compare else {
                    self.mode = Mode::Bytes;
                    return;
                };
                if let Some(node) = c.ui(ui, &self.tree) {
                    self.select(node);
                    self.reveal = true;
                    self.to_offset = Some(self.tree.nodes[node].offset);
                    self.mode = Mode::Bytes;
                }
            }
        }
    }

    fn tools(&self) -> Vec<Tool> {
        let mut tools = vec![
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
        ];
        if self.mode == Mode::Content || self.found().is_some() {
            tools.push(Tool {
                id: "content",
                icon: icons::PACKAGE,
                label: "Content",
            });
        }
        tools.extend([
            Tool {
                id: "save",
                icon: "\u{1F4BE}",
                label: "Save the structure",
            },
            Tool {
                id: "compare",
                icon: "\u{2696}",
                label: "Compare with a structure",
            },
        ]);
        if let Some(c) = &self.compare {
            tools.push(Tool {
                id: "comparison",
                icon: "\u{1F50D}",
                label: "Comparison",
            });
            tools.push(if c.changed_only() {
                Tool {
                    id: "changed",
                    icon: "\u{1F441}",
                    label: "All nodes",
                }
            } else {
                Tool {
                    id: "changed",
                    icon: "\u{1F441}",
                    label: "Changed only",
                }
            });
            tools.push(Tool {
                id: "save_comparison",
                icon: "\u{1F4DD}",
                label: "Save the comparison",
            });
        }
        tools
    }

    fn menu(&self) -> Vec<Tool> {
        let mut items = Vec::new();
        if self.found().is_some() {
            items.push(Tool {
                id: "content_tab",
                icon: icons::GO_TO,
                label: "Open the content in a tab",
            });
        }
        items.extend([
            Tool {
                id: "save_node",
                icon: icons::EXPORT,
                label: "Save the node as a file",
            },
            Tool {
                id: "read_all",
                icon: icons::BOOKS,
                label: "Read every node",
            },
        ]);
        items
    }

    fn act(&mut self, id: &str, host: &mut Host) {
        match id {
            "bytes" => self.mode = Mode::Bytes,
            "content" => self.mode = Mode::Content,
            "content_tab" => {
                if let Some(found) = self.found().cloned() {
                    match self.content_file(&found) {
                        Ok((name, mime, bytes)) => {
                            host.requests.push(Request::OpenBytes { name, mime, bytes });
                        }
                        Err(why) => host.requests.push(Request::Status(why)),
                    }
                }
            }
            "save_node" => self.save_node(),
            "read_all" => {
                engine::expand_all(&mut self.tree, self.format, &mut self.source, self.limits);
                self.tree.nodes[0].size = self.source.len();
                self.page = None;
                self.found = None;
            }
            "text" => {
                self.page();
                self.mode = Mode::Text;
            }
            "save" => self.save(),
            "compare" => self.ask_compare(),
            "comparison" if self.compare.is_some() => self.mode = Mode::Compare,
            "changed" => {
                if let Some(c) = &mut self.compare {
                    c.toggle_changed_only();
                }
            }
            "save_comparison" => self.save_comparison(),
            _ => {}
        }
    }

    fn search(&mut self) -> Option<&mut Search> {
        match self.mode {
            Mode::Text => self.page.as_mut().map(|p| &mut p.search),
            Mode::Content => match &mut self.shown {
                Some((_, Ok(inner))) => inner.search(),
                _ => None,
            },
            Mode::Bytes | Mode::Compare => None,
        }
    }

    fn status(&self) -> String {
        if self.mode == Mode::Compare
            && let Some(c) = &self.compare
        {
            let mut line = c.status();
            if let Some(saved) = &self.saved {
                line.push_str("; ");
                line.push_str(saved);
            }
            return line;
        }
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
        if self.mode == Mode::Content
            && let Some((_, Ok(inner))) = &self.shown
        {
            line.push_str("; ");
            line.push_str(&inner.status());
        }
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
