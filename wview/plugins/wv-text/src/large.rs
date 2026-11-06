//! A text larger than the plugin reads whole (the setting `max_mb`), shown in parts. A thread
//! counts its lines and keeps the start of a line every `STEP` lines and of the first line after
//! every `SPAN` bytes (`count`); the view reads the lines it shows through `BlockFile` from the
//! nearest of those starts (`read_lines`) and scrolls by its own numbers of lines, so a file of any
//! size scrolls to its end. The search of the tab lists the lines that hold every word of it,
//! found by a thread of its own (`search`), and a click on one goes to it in the text. A text is
//! read as UTF-8; a line shows its first `LINE` bytes.

use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};

use wwrt::blocks::BlockFile;
use wwrt::egui::{self, Align2, Key, Sense, pos2, vec2};
use wwrt::input;
use wwrt::plugin::{Host, View};
use wwrt::ww::i18n::{tr, trf};
use wwrt::ww::{Search, matches_words, style};

/// A start of a line kept every so many lines, and after so many bytes.
pub const STEP: u64 = 1024;
pub const SPAN: u64 = 1 << 20;
/// The bytes of a line shown, and looked at by the search, at most.
pub const LINE: usize = 4096;
/// The lines a search lists at most.
pub const FOUND: usize = 10_000;
/// The bytes the threads read at a time.
const CHUNK: usize = 1 << 20;
/// The keys of the view while it has the focus.
const KEYS: [Key; 8] = [
    Key::ArrowUp,
    Key::ArrowDown,
    Key::ArrowLeft,
    Key::ArrowRight,
    Key::PageUp,
    Key::PageDown,
    Key::Home,
    Key::End,
];

/// The lines of a file counted so far.
#[derive(Debug, Default)]
pub struct Index {
    /// The number and the offset of the lines whose start is kept, the first one `(0, 0)`.
    pub points: Vec<(u64, u64)>,
    pub lines: u64,
    pub done: bool,
    pub error: Option<String>,
}

/// The lines that hold a search.
#[derive(Debug, Default)]
pub struct Found {
    /// The number of each line and its first bytes as a text.
    pub lines: Vec<(u64, String)>,
    pub done: bool,
    /// Whether the search stopped at `FOUND` lines.
    pub full: bool,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

/// A line as it shows: its first bytes as UTF-8, without the return of its end, a tab as four
/// spaces.
fn shown(bytes: &[u8]) -> String {
    let bytes = bytes.strip_suffix(b"\r").unwrap_or(bytes);
    String::from_utf8_lossy(bytes).replace('\t', "    ")
}

/// Reads the lines of a reader and gives each to `each` with its number, the offset of its start
/// and its first `LINE` bytes, until it answers false, the reader ends or `stop` is set; an error
/// of the reader is its reason.
fn lines_of(
    reader: &mut dyn Read,
    stop: &AtomicBool,
    each: &mut dyn FnMut(u64, u64, &[u8]) -> bool,
) -> Result<(), String> {
    let mut buf = vec![0u8; CHUNK];
    let mut head = Vec::with_capacity(LINE);
    let (mut line, mut at, mut start) = (0u64, 0u64, 0u64);
    loop {
        if stop.load(Ordering::Relaxed) {
            return Ok(());
        }
        let n = match reader.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => return Err(e.to_string()),
        };
        for (i, &b) in buf[..n].iter().enumerate() {
            if b == b'\n' {
                if !each(line, start, &head) {
                    return Ok(());
                }
                head.clear();
                line += 1;
                start = at + i as u64 + 1;
            } else if head.len() < LINE {
                head.push(b);
            }
        }
        at += n as u64;
    }
    if at > start {
        each(line, start, &head);
    }
    Ok(())
}

/// Counts the lines of a reader into the index: the start of line 0, then of a line every `step`
/// lines and of the first line that starts `span` bytes or more after the last start kept.
pub fn count(reader: &mut dyn Read, index: &Mutex<Index>, stop: &AtomicBool, step: u64, span: u64) {
    lock(index).points = vec![(0, 0)];
    let mut last = (0u64, 0u64);
    let mut pending = Vec::new();
    let mut lines = 0u64;
    let mut seen = 0usize;
    let result = lines_of(reader, stop, &mut |line, start, _| {
        if line > 0 && (line - last.0 >= step || start - last.1 >= span) {
            last = (line, start);
            pending.push(last);
        }
        lines = line + 1;
        seen += 1;
        // The lines counted so far reach the view now and then.
        if seen.is_multiple_of(65_536) {
            let mut i = lock(index);
            i.points.append(&mut pending);
            i.lines = lines;
        }
        true
    });
    let mut i = lock(index);
    i.points.append(&mut pending);
    i.lines = lines;
    i.error = result.err();
    i.done = true;
}

/// The lines that hold every word of a search, at most `FOUND` of them.
pub fn search(reader: &mut dyn Read, words: &[String], found: &Mutex<Found>, stop: &AtomicBool) {
    let mut seen = 0usize;
    let _ = lines_of(reader, stop, &mut |line, _, head| {
        let text = shown(head);
        seen += 1;
        if matches_words(&text, words) {
            let mut f = lock(found);
            f.lines.push((line, text));
            if f.lines.len() >= FOUND {
                f.full = true;
                return false;
            }
        }
        true
    });
    lock(found).done = true;
}

/// Up to `n` lines from line `first`, read from the nearest start kept before it.
pub fn read_lines(
    file: &mut BlockFile,
    points: &[(u64, u64)],
    first: u64,
    n: usize,
) -> Vec<String> {
    let mut out = Vec::new();
    let p = points.partition_point(|(l, _)| *l <= first);
    let Some(&(mut line, mut at)) = points.get(p.saturating_sub(1)) else {
        return out;
    };
    let mut head = Vec::new();
    let mut block = vec![0u8; 64 << 10];
    while out.len() < n {
        let got = match file.read_at(at, &mut block) {
            Ok(0) | Err(_) => break,
            Ok(got) => got,
        };
        for &b in &block[..got] {
            if b == b'\n' {
                if line >= first {
                    out.push(shown(&head));
                    if out.len() == n {
                        return out;
                    }
                }
                head.clear();
                line += 1;
            } else if line >= first && head.len() < LINE {
                head.push(b);
            }
        }
        at += got as u64;
    }
    if line >= first && !head.is_empty() && out.len() < n {
        out.push(shown(&head));
    }
    out
}

/// A thread that ends when its flag is set (the view closed, or a new search).
struct Worker {
    stop: Arc<AtomicBool>,
}

impl Worker {
    fn start(path: &Path, work: impl FnOnce(&mut dyn Read, &AtomicBool) + Send + 'static) -> Self {
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        let path = path.to_path_buf();
        let _ = std::thread::Builder::new()
            .name("wv-text".to_owned())
            .spawn(move || {
                if let Ok(file) = File::open(&path) {
                    work(&mut std::io::BufReader::with_capacity(CHUNK, file), &flag);
                }
            });
        Worker { stop }
    }
}

impl Drop for Worker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

pub struct LargeText {
    path: PathBuf,
    file: BlockFile,
    index: Arc<Mutex<Index>>,
    _counter: Worker,
    /// The first line shown, the first character shown of each line, and the wheel not yet a
    /// whole line.
    top: u64,
    left: usize,
    wheel: f32,
    rows: u64,
    /// The lines shown: the first, the count asked, the lines counted then, and their texts.
    lines: Option<(u64, usize, u64, Vec<String>)>,
    /// The line a click on a line found went to.
    marked: Option<u64>,
    search: Search,
    searched: Vec<String>,
    found: Option<(Arc<Mutex<Found>>, Worker)>,
}

impl LargeText {
    /// Opens a file and starts to count its lines.
    pub fn open(path: &Path) -> Result<Self, String> {
        let file = BlockFile::open(path, 64 << 10, 8 << 20).map_err(|e| e.to_string())?;
        let index = Arc::new(Mutex::new(Index::default()));
        let counted = index.clone();
        let counter = Worker::start(path, move |reader, stop| {
            count(reader, &counted, stop, STEP, SPAN);
        });
        Ok(LargeText {
            path: path.to_path_buf(),
            file,
            index,
            _counter: counter,
            top: 0,
            left: 0,
            wheel: 0.0,
            rows: 1,
            lines: None,
            marked: None,
            search: Search::default(),
            searched: Vec::new(),
            found: None,
        })
    }

    /// The lines counted so far and whether the count is done.
    fn counted(&self) -> (u64, bool) {
        let i = lock(&self.index);
        (i.lines, i.done)
    }

    /// The lines from `top` the view has room for, read again when they change.
    fn visible(&mut self) -> &[String] {
        let (counted, _) = self.counted();
        let n = self.rows as usize + 1;
        let fresh = matches!(&self.lines, Some((t, c, k, _)) if *t == self.top && *c == n
            && (*k == counted || self.top + n as u64 <= *k));
        if !fresh {
            let index = lock(&self.index);
            let read = read_lines(&mut self.file, &index.points, self.top, n);
            drop(index);
            self.lines = Some((self.top, n, counted, read));
        }
        self.lines.as_ref().map_or(&[], |(_, _, _, l)| l.as_slice())
    }

    /// Goes to a line, a third of the view under the top.
    fn go_to(&mut self, line: u64) {
        self.top = line.saturating_sub(self.rows / 3);
        self.marked = Some(line);
    }

    /// A new search when its words changed.
    fn follow_search(&mut self) {
        let words = self.search.words();
        if words == self.searched {
            return;
        }
        self.found = (!words.is_empty()).then(|| {
            let found = Arc::new(Mutex::new(Found::default()));
            let into = found.clone();
            let wanted = words.clone();
            let worker = Worker::start(&self.path, move |reader, stop| {
                search(reader, &wanted, &into, stop);
            });
            (found, worker)
        });
        self.searched = words;
    }

    /// The text: the numbers of the lines, the lines, and a bar of the place in the file.
    fn text_ui(&mut self, ui: &mut egui::Ui) {
        let rect = ui.available_rect_before_wrap();
        let font = style::mono(ui, 13.0);
        let row = ui.fonts_mut(|f| f.row_height(&font)).max(1.0);
        let glyph = ui.fonts_mut(|f| f.glyph_width(&font, '0')).max(1.0);
        let (counted, _) = self.counted();
        let digits = counted.max(1).ilog10() as f32 + 1.0;
        let gutter = digits * glyph + 16.0;
        let bar = 12.0;
        let text = egui::Rect::from_min_max(rect.min, pos2(rect.right() - bar, rect.bottom()));
        let track = egui::Rect::from_min_max(pos2(rect.right() - bar, rect.top()), rect.max);
        self.rows = ((text.height() / row).floor() as u64).max(1);
        let (_, input) = input::area(ui, ui.id().with("wv-large-text"), text, &KEYS);
        let last = counted.saturating_sub(self.rows);
        let mut top = self.top as i128;
        let page = self.rows.saturating_sub(1).max(1) as i128;
        for (key, mods) in &input.keys {
            match key {
                Key::ArrowDown => top += 1,
                Key::ArrowUp => top -= 1,
                Key::PageDown => top += page,
                Key::PageUp => top -= page,
                Key::Home if mods.is_none() || mods.ctrl => top = 0,
                Key::End if mods.is_none() || mods.ctrl => top = i128::from(last),
                Key::ArrowRight => self.left += 8,
                Key::ArrowLeft => self.left = self.left.saturating_sub(8),
                _ => {}
            }
        }
        self.wheel += input.wheel.y / row;
        let whole = self.wheel.trunc();
        self.wheel -= whole;
        top += whole as i128;
        if input.wheel.x != 0.0 {
            let by = (input.wheel.x / glyph) as i64;
            self.left = (self.left as i64 + by).max(0) as usize;
        }
        let bar_response = ui.interact(
            track,
            ui.id().with("wv-large-text-bar"),
            Sense::click_and_drag(),
        );
        if (bar_response.dragged() || bar_response.clicked())
            && let Some(p) = bar_response.interact_pointer_pos()
        {
            let at = ((p.y - track.top()) / track.height()).clamp(0.0, 1.0);
            top = (f64::from(at) * last as f64).round() as i128;
        }
        self.top = top.clamp(0, i128::from(last)) as u64;
        let (top, left, marked) = (self.top, self.left, self.marked);
        let palette = style::palette(ui);
        let painter = ui.painter_at(rect);
        painter.rect_filled(
            egui::Rect::from_min_max(text.min, pos2(text.left() + gutter - 6.0, text.bottom())),
            0.0,
            palette.card,
        );
        let room = ((text.width() - gutter) / glyph) as usize + 2;
        let colour = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();
        let lines = self.visible().to_vec();
        for (i, line) in lines.iter().enumerate() {
            let number = top + i as u64;
            let y = text.top() + i as f32 * row;
            if marked == Some(number) {
                let band = egui::Rect::from_min_size(pos2(text.left(), y), vec2(text.width(), row));
                painter.rect_filled(
                    band,
                    0.0,
                    ui.visuals().selection.bg_fill.gamma_multiply(0.35),
                );
            }
            painter.text(
                pos2(text.left() + gutter - 12.0, y),
                Align2::RIGHT_TOP,
                (number + 1).to_string(),
                font.clone(),
                weak,
            );
            let part: String = line.chars().skip(left).take(room).collect();
            painter.text(
                pos2(text.left() + gutter, y),
                Align2::LEFT_TOP,
                part,
                font.clone(),
                colour,
            );
        }
        painter.rect_filled(track, 0.0, palette.card);
        if counted > self.rows {
            let h = (track.height() * self.rows as f32 / counted as f32).max(20.0);
            let at = if last == 0 {
                0.0
            } else {
                top as f32 / last as f32
            };
            let thumb = egui::Rect::from_min_size(
                pos2(track.left() + 2.0, track.top() + at * (track.height() - h)),
                vec2(bar - 4.0, h),
            );
            painter.rect_filled(thumb, 4.0, ui.visuals().widgets.inactive.bg_fill);
        }
        ui.advance_cursor_after_rect(rect);
    }

    /// The lines that hold the search; a click on one goes to it in the text.
    fn found_ui(&mut self, ui: &mut egui::Ui) {
        let Some((found, _)) = &self.found else {
            return;
        };
        let found = found.clone();
        let (count, done, full) = {
            let f = lock(&found);
            (f.lines.len(), f.done, f.full)
        };
        ui.horizontal(|ui| {
            ui.weak(trf("{n} lines hold the search", &[("n", &count)]));
            if !done {
                ui.spinner();
            }
        });
        if full {
            ui.weak(trf(
                "The search shows the first {n} lines that hold it.",
                &[("n", &FOUND)],
            ));
        }
        ui.weak(tr("A click on a line goes to it in the text."));
        ui.separator();
        let font = style::mono(ui, 13.0);
        let row = ui.fonts_mut(|f| f.row_height(&font)) + 4.0;
        let mut go = None;
        egui::ScrollArea::vertical()
            .auto_shrink(false)
            .show_rows(ui, row, count, |ui, range| {
                let f = lock(&found);
                for (number, text) in &f.lines[range] {
                    let shown = format!("{:>8}  {text}", number + 1);
                    let label = egui::Button::new(egui::RichText::new(shown).font(font.clone()))
                        .frame(false)
                        .truncate();
                    if ui.add(label).clicked() {
                        go = Some(*number);
                    }
                }
            });
        if !done {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }
        if let Some(line) = go {
            self.search.text.clear();
            self.go_to(line);
        }
    }
}

impl View for LargeText {
    fn ui(&mut self, ui: &mut egui::Ui, _host: &mut Host) {
        self.follow_search();
        let (_, done) = self.counted();
        egui::CentralPanel::default().show(ui, |ui| {
            if self.found.is_some() {
                self.found_ui(ui);
            } else {
                self.text_ui(ui);
            }
        });
        if !done {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    fn search(&mut self) -> Option<&mut Search> {
        Some(&mut self.search)
    }

    fn status(&self) -> String {
        let i = lock(&self.index);
        if let Some(e) = &i.error {
            return e.clone();
        }
        let mut line = trf(
            "line {line} of {count}",
            &[("line", &(self.top + 1)), ("count", &i.lines)],
        );
        if !i.done {
            line.push_str(", ");
            line.push_str(tr("counting the lines"));
        }
        if let Some((found, _)) = &self.found {
            line.push_str(", ");
            line.push_str(&trf(
                "{n} lines hold the search",
                &[("n", &lock(found).lines.len())],
            ));
        }
        line
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn counted(text: &str, step: u64, span: u64) -> Index {
        let index = Mutex::new(Index::default());
        count(
            &mut text.as_bytes(),
            &index,
            &AtomicBool::new(false),
            step,
            span,
        );
        index.into_inner().unwrap()
    }

    #[test]
    fn the_lines_are_counted_with_a_start_every_step_and_span() {
        let i = counted("a\nbb\n\nccc", 2, 1 << 20);
        assert_eq!((i.lines, i.done), (4, true));
        assert_eq!(i.points, [(0, 0), (2, 5)]);
        let i = counted("a\nbb\n\nccc\n", 1 << 20, 3);
        assert_eq!(i.lines, 4, "the newline at the end opens no line");
        assert_eq!(i.points, [(0, 0), (2, 5)]);
        assert_eq!(counted("", 2, 2).lines, 0);
    }

    #[test]
    fn the_lines_shown_are_read_from_the_nearest_start() {
        let text: String = (0..5000).map(|i| format!("line {i}\r\n")).collect();
        let i = counted(&text, 1024, 1 << 20);
        assert_eq!((i.lines, i.points.len()), (5000, 5));
        let mut file = BlockFile::from_bytes(text.into_bytes());
        for first in [0u64, 1023, 1024, 4990] {
            let lines = read_lines(&mut file, &i.points, first, 3);
            let want: Vec<String> = (first..(first + 3).min(5000))
                .map(|n| format!("line {n}"))
                .collect();
            assert_eq!(lines, want, "from {first}");
        }
        assert!(read_lines(&mut file, &i.points, 6000, 3).is_empty());
        let long = format!("{}\nend", "x".repeat(LINE * 2));
        let i = counted(&long, 1024, 1 << 20);
        let mut file = BlockFile::from_bytes(long.into_bytes());
        let lines = read_lines(&mut file, &i.points, 0, 2);
        assert_eq!((lines[0].len(), lines[1].as_str()), (LINE, "end"));
    }

    #[test]
    fn the_search_lists_the_lines_that_hold_every_word() {
        let text = "alpha beta\ngamma\nBeta Alpha\n\talpha only\n";
        let found = Mutex::new(Found::default());
        let words = vec!["alpha".to_owned(), "beta".to_owned()];
        search(
            &mut text.as_bytes(),
            &words,
            &found,
            &AtomicBool::new(false),
        );
        let f = found.into_inner().unwrap();
        let numbers: Vec<u64> = f.lines.iter().map(|(n, _)| *n).collect();
        assert_eq!((numbers, f.done, f.full), (vec![0, 2], true, false));
    }

    #[test]
    fn a_file_opens_and_counts_its_lines_in_a_thread() {
        let path = std::env::temp_dir().join(format!("wv-text-large-{}.log", std::process::id()));
        let text: String = (0..3000).map(|i| format!("entry {i}\n")).collect();
        std::fs::write(&path, text).unwrap();
        let view = LargeText::open(&path).unwrap();
        for _ in 0..500 {
            if view.counted().1 {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(view.status(), "line 1 of 3000");
        drop(view);
        let _ = std::fs::remove_file(&path);
    }
}
