//! The view of an image: fitted to the tab or zoomed, moved with the mouse and the keys, drawn in
//! tiles of the level of its size the zoom needs, an animation played, the pages of a file of
//! several turned. The image is decoded in a thread of its own (`start`), and the view shows the
//! one it has until the next one is there.

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

use wwrt::egui::{self, Color32, Key, Pos2, Rect, TextureHandle, Vec2};
use wwrt::input::{self, Cursor};
use wwrt::plugin::{Host, Request, Step, Tool, View};
use wwrt::ww::i18n::{tr, trf};

use crate::picture::Picture;
use crate::wic::{Decoded, Kind, Options, Source};

/// The keys of the view while it has the focus.
const KEYS: [Key; 19] = [
    Key::ArrowLeft,
    Key::ArrowRight,
    Key::ArrowUp,
    Key::ArrowDown,
    Key::PageUp,
    Key::PageDown,
    Key::Home,
    Key::End,
    Key::Plus,
    Key::Equals,
    Key::Minus,
    Key::Num0,
    Key::Num1,
    Key::Num2,
    Key::Num3,
    Key::Num4,
    Key::Space,
    Key::Comma,
    Key::Period,
];
/// The zooms of + and -, of the size of the image; the wheel takes any between the first and the
/// last.
pub const LEVELS: [f32; 22] = [
    0.02,
    0.03,
    0.05,
    0.08,
    0.1,
    0.125,
    1.0 / 6.0,
    0.25,
    1.0 / 3.0,
    0.5,
    2.0 / 3.0,
    1.0,
    1.5,
    2.0,
    3.0,
    4.0,
    6.0,
    8.0,
    12.0,
    16.0,
    24.0,
    32.0,
];
/// The side of a tile, in the pixels of its level.
pub const TILE: u32 = 1024;
/// The tiles kept as textures; the ones drawn least lately go first.
const KEPT: usize = 64;
/// The tiles made in one frame; the others come in the next ones.
const MADE: usize = 6;

/// The next zoom of `LEVELS` above (`up`) or below a scale.
pub fn step(scale: f32, up: bool) -> f32 {
    if up {
        LEVELS
            .iter()
            .copied()
            .find(|&l| l > scale * 1.001)
            .unwrap_or(LEVELS[LEVELS.len() - 1])
    } else {
        LEVELS
            .iter()
            .rev()
            .copied()
            .find(|&l| l < scale / 1.001)
            .unwrap_or(LEVELS[0])
    }
}

/// The level a picture drawn `width` pixels of the screen wide takes, of the widths of its levels
/// (each smaller than the one before): the smallest one with a pixel for every pixel of the
/// screen, else the first.
pub fn level(widths: &[u32], width: f32) -> usize {
    widths.iter().rposition(|&w| w as f32 >= width).unwrap_or(0)
}

/// The tiles (column, row) of a level of `size` a picture drawn in `image` shows in `view`.
pub fn visible(size: [u32; 2], image: Rect, view: Rect) -> Vec<[u32; 2]> {
    let shown = image.intersect(view);
    if !shown.is_positive() || size[0] == 0 || size[1] == 0 {
        return Vec::new();
    }
    let span = |from: f32, to: f32, origin: f32, length: f32, side: u32| {
        let k = side as f32 / length;
        let first = (((from - origin) * k).floor().max(0.0) as u32).min(side - 1);
        let last = ((((to - origin) * k).ceil() as u32).clamp(1, side) - 1).max(first);
        (first / TILE, last / TILE)
    };
    let (c0, c1) = span(
        shown.min.x,
        shown.max.x,
        image.min.x,
        image.width(),
        size[0],
    );
    let (r0, r1) = span(
        shown.min.y,
        shown.max.y,
        image.min.y,
        image.height(),
        size[1],
    );
    let mut out = Vec::new();
    for r in r0..=r1 {
        for c in c0..=c1 {
            out.push([c, r]);
        }
    }
    out
}

/// The pixels of a tile (with one pixel of its neighbours around it, where it has them, so that
/// a smooth zoom shows no seam) and the part of them that is the tile itself.
pub fn tile(picture: &Picture, [c, r]: [u32; 2]) -> (egui::ColorImage, Rect) {
    let [w, h] = picture.size;
    let (x0, y0) = (c * TILE, r * TILE);
    let (x1, y1) = ((x0 + TILE).min(w), (y0 + TILE).min(h));
    let (bx0, by0) = (x0.saturating_sub(1), y0.saturating_sub(1));
    let (bx1, by1) = ((x1 + 1).min(w), (y1 + 1).min(h));
    let (tw, th) = (bx1 - bx0, by1 - by0);
    let mut pixels = Vec::with_capacity((tw * th * 4) as usize);
    for y in by0..by1 {
        let from = ((y * w + bx0) * 4) as usize;
        pixels.extend_from_slice(&picture.pixels[from..from + (tw * 4) as usize]);
    }
    let image = egui::ColorImage::from_rgba_unmultiplied([tw as usize, th as usize], &pixels);
    let uv = Rect::from_min_max(
        Pos2::new((x0 - bx0) as f32 / tw as f32, (y0 - by0) as f32 / th as f32),
        Pos2::new((x1 - bx0) as f32 / tw as f32, (y1 - by0) as f32 / th as f32),
    );
    (image, uv)
}

/// Where a tile of a level of `size` is drawn when the picture is drawn in `image`.
fn tile_rect(size: [u32; 2], [c, r]: [u32; 2], image: Rect) -> Rect {
    let kx = image.width() / size[0] as f32;
    let ky = image.height() / size[1] as f32;
    let (x0, y0) = (c * TILE, r * TILE);
    let (x1, y1) = ((x0 + TILE).min(size[0]), (y0 + TILE).min(size[1]));
    Rect::from_min_max(
        image.min + Vec2::new(x0 as f32 * kx, y0 as f32 * ky),
        image.min + Vec2::new(x1 as f32 * kx, y1 as f32 * ky),
    )
}

/// Decodes a page of an image (`None`: the first) in a thread of its own.
pub fn start(
    source: Source,
    page: Option<u32>,
    options: Options,
) -> Receiver<Result<Decoded, String>> {
    let (send, receive) = mpsc::channel();
    // A thread that does not start drops the sender, and the view says that the decoding stopped.
    let _ = std::thread::Builder::new()
        .name("wv-image".to_owned())
        .spawn(move || {
            let _ = send.send(crate::decode(&source, page, options));
        });
    receive
}

/// How large the image is drawn.
#[derive(Clone, Copy, PartialEq)]
enum Zoom {
    /// The whole image in the tab, never above its own size.
    Fit,
    /// So many points of the window a pixel of the image.
    Scale(f32),
}

pub struct ImageView {
    source: Source,
    options: Options,
    smooth: bool,
    /// The image decoded, once it is there.
    image: Option<Decoded>,
    /// The image (or the page) being decoded.
    job: Option<Receiver<Result<Decoded, String>>>,
    /// Why the last decoding failed.
    error: Option<String>,
    /// The textures of the tiles by their level and place, with their part of the texture and
    /// the frame that drew them last.
    tiles: HashMap<(usize, [u32; 2]), (TextureHandle, Rect, u64)>,
    /// The texture of an animation and the frame it shows.
    frame: Option<(TextureHandle, usize)>,
    /// The frames drawn, for the age of the tiles.
    tick: u64,
    zoom: Zoom,
    /// Where the centre of the image is from the centre of the view, in points.
    offset: Vec2,
    /// The scale of the last frame, for the status bar and a zoom from the fitted size.
    drawn: f32,
    /// The area of the last frame, for the commands of the bar.
    rect: Rect,
    /// The frame of an animation shown, whether it plays and when it gives way to the next one
    /// (the time of the input of egui).
    index: usize,
    playing: bool,
    next_at: Option<f64>,
}

impl ImageView {
    pub fn new(
        source: Source,
        options: Options,
        smooth: bool,
        playing: bool,
        first: Result<Decoded, Receiver<Result<Decoded, String>>>,
    ) -> ImageView {
        let (image, job) = match first {
            Ok(image) => (Some(image), None),
            Err(job) => (None, Some(job)),
        };
        ImageView {
            source,
            options,
            smooth,
            image,
            job,
            error: None,
            tiles: HashMap::new(),
            frame: None,
            tick: 0,
            zoom: Zoom::Fit,
            offset: Vec2::ZERO,
            drawn: 1.0,
            rect: Rect::NOTHING,
            index: 0,
            playing,
            next_at: None,
        }
    }

    /// The image of a finished decoding, in place of the one before.
    fn poll(&mut self) {
        let Some(job) = &self.job else {
            return;
        };
        let result = match job.try_recv() {
            Ok(result) => result,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err(tr("The decoding stopped").to_owned()),
        };
        self.job = None;
        match result {
            Ok(image) => {
                self.image = Some(image);
                self.error = None;
                self.tiles.clear();
                self.frame = None;
                self.index = 0;
                self.next_at = None;
                self.fit_now();
            }
            Err(e) => self.error = Some(e),
        }
    }

    fn size(&self) -> Option<[u32; 2]> {
        self.image.as_ref().map(|i| i.size)
    }

    /// The frames of an animation.
    fn frames(&self) -> Option<&[(Picture, u32)]> {
        match &self.image.as_ref()?.kind {
            Kind::Animation { frames } => Some(frames),
            Kind::Still { .. } => None,
        }
    }

    /// The page shown and the pages of a file of several.
    fn pages(&self) -> Option<(u32, u32)> {
        let image = self.image.as_ref()?;
        match &image.kind {
            Kind::Still { page, .. } if image.count > 1 => Some((*page, image.count)),
            _ => None,
        }
    }

    /// The scale of the fit in an area.
    fn fit(&self, rect: Rect) -> f32 {
        let Some([w, h]) = self.size() else {
            return 1.0;
        };
        (rect.width() / w as f32)
            .min(rect.height() / h as f32)
            .clamp(LEVELS[0], 1.0)
    }

    fn scale(&self, rect: Rect) -> f32 {
        match self.zoom {
            Zoom::Fit => self.fit(rect),
            Zoom::Scale(s) => s,
        }
    }

    /// A new scale with the point `at` of the window where it was on the image.
    fn zoom_at(&mut self, rect: Rect, at: Pos2, new: f32) {
        let old = self.scale(rect);
        let new = new.clamp(LEVELS[0], LEVELS[LEVELS.len() - 1]);
        let centre = rect.center() + self.offset;
        let point = (at - centre) / old;
        self.offset = at - rect.center() - point * new;
        self.zoom = Zoom::Scale(new);
    }

    fn fit_now(&mut self) {
        self.zoom = Zoom::Fit;
        self.offset = Vec2::ZERO;
    }

    /// The image stays at least partly in the view.
    fn keep_in(&mut self, rect: Rect, scale: f32) {
        let Some([w, h]) = self.size() else {
            return;
        };
        let size = Vec2::new(w as f32, h as f32) * scale;
        let room = ((size + rect.size()) / 2.0 - Vec2::splat(32.0)).max(Vec2::ZERO);
        self.offset = self.offset.clamp(-room, room);
    }

    /// The frame or the page before (`-1`) or after (`1`) the one shown; an animation stops.
    fn turn(&mut self, by: i64) {
        if let Some(n) = self.frames().map(<[_]>::len) {
            self.playing = false;
            self.next_at = None;
            self.index = (self.index as i64 + by).rem_euclid(n as i64) as usize;
        } else if let Some((page, count)) = self.pages()
            && self.job.is_none()
        {
            let next = (i64::from(page) + by).rem_euclid(i64::from(count)) as u32;
            self.job = Some(start(self.source.clone(), Some(next), self.options));
        }
    }

    fn play(&mut self) {
        if self.frames().is_some_and(|f| f.len() > 1) {
            self.playing = !self.playing;
            self.next_at = None;
        }
    }

    /// The next frame of an animation when its time came, and a frame of the window then.
    fn animate(&mut self, ctx: &egui::Context) {
        if !self.playing {
            return;
        }
        let n = match self.frames() {
            Some(frames) if frames.len() > 1 => frames.len(),
            _ => return,
        };
        let delay = |view: &Self, i: usize| {
            view.frames()
                .map_or(0.1, |f| f64::from(f[i % f.len()].1) / 1000.0)
        };
        let now = ctx.input(|i| i.time);
        let mut due = self
            .next_at
            .unwrap_or_else(|| now + delay(self, self.index));
        if now >= due {
            self.index = (self.index + 1) % n;
            // A view that was not drawn for a while goes on from now, not from the time it missed.
            let from = if now - due > 1.0 { now } else { due };
            due = from + delay(self, self.index);
        }
        self.next_at = Some(due);
        ctx.request_repaint_after(Duration::from_secs_f64((due - now).max(0.0)));
    }

    fn options(&self) -> egui::TextureOptions {
        if self.smooth {
            egui::TextureOptions::LINEAR
        } else {
            egui::TextureOptions {
                magnification: egui::TextureFilter::Nearest,
                ..egui::TextureOptions::LINEAR
            }
        }
    }

    /// Draws the frame of an animation shown in `image`.
    fn draw_frame(&mut self, ui: &egui::Ui, view: Rect, image: Rect) {
        let options = self.options();
        if self.frame.as_ref().map(|(_, shown)| *shown) != Some(self.index) {
            let Some(frames) = self.frames() else {
                return;
            };
            let (picture, _) = &frames[self.index.min(frames.len() - 1)];
            let pixels = egui::ColorImage::from_rgba_unmultiplied(
                [picture.size[0] as usize, picture.size[1] as usize],
                &picture.pixels,
            );
            match &mut self.frame {
                Some((texture, shown)) => {
                    texture.set(pixels, options);
                    *shown = self.index;
                }
                None => {
                    let texture = ui.ctx().load_texture("wv-image-frame", pixels, options);
                    self.frame = Some((texture, self.index));
                }
            }
        }
        if let Some((texture, _)) = &self.frame {
            let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
            ui.painter_at(view)
                .image(texture.id(), image, uv, Color32::WHITE);
        }
    }

    /// Draws a still image in `image`: the tiles of the level the zoom needs, and where one of
    /// them is not made yet, the last level, which is made first.
    fn draw_tiles(&mut self, ui: &egui::Ui, view: Rect, image: Rect) {
        let options = self.options();
        let Some(Decoded {
            kind: Kind::Still { levels, .. },
            ..
        }) = &self.image
        else {
            return;
        };
        self.tick += 1;
        let tick = self.tick;
        let ctx = ui.ctx();
        let painter = ui.painter_at(view);
        let widths: Vec<u32> = levels.iter().map(|l| l.size[0]).collect();
        let top = levels.len() - 1;
        let chosen = level(&widths, image.width() * ctx.pixels_per_point());
        let mut made = 0;
        let mut waiting = Vec::new();
        for lvl in [top, chosen] {
            for place in visible(levels[lvl].size, image, view) {
                match self.tiles.entry((lvl, place)) {
                    Entry::Occupied(mut kept) => kept.get_mut().2 = tick,
                    Entry::Vacant(_) if lvl != top && made >= MADE => waiting.push(place),
                    Entry::Vacant(new) => {
                        let (pixels, uv) = tile(&levels[lvl], place);
                        let name = format!("wv-image-{lvl}-{}-{}", place[0], place[1]);
                        new.insert((ctx.load_texture(name, pixels, options), uv, tick));
                        made += 1;
                    }
                }
            }
            if lvl == chosen {
                break;
            }
        }
        // The last level under a tile still to come, then the tiles of the level chosen.
        let draw = |lvl: usize, place: [u32; 2], clip: Rect| {
            if let Some((texture, uv, _)) = self.tiles.get(&(lvl, place)) {
                let at = tile_rect(levels[lvl].size, place, image);
                painter
                    .with_clip_rect(clip)
                    .image(texture.id(), at, *uv, Color32::WHITE);
            }
        };
        for place in &waiting {
            let clip = tile_rect(levels[chosen].size, *place, image).intersect(view);
            for under in visible(levels[top].size, image, clip) {
                draw(top, under, clip);
            }
        }
        for place in visible(levels[chosen].size, image, view) {
            draw(chosen, place, view);
        }
        if !waiting.is_empty() {
            ctx.request_repaint();
        }
        if self.tiles.len() > KEPT {
            let mut old: Vec<((usize, [u32; 2]), u64)> = self
                .tiles
                .iter()
                .filter(|((lvl, _), (_, _, t))| *t != tick && *lvl != top)
                .map(|(k, (_, _, t))| (*k, *t))
                .collect();
            old.sort_by_key(|(_, t)| *t);
            let extra = self.tiles.len() - KEPT;
            for (key, _) in old.into_iter().take(extra) {
                self.tiles.remove(&key);
            }
        }
    }
}

impl View for ImageView {
    fn ui(&mut self, ui: &mut egui::Ui, host: &mut Host) {
        self.poll();
        if self.job.is_some() {
            ui.ctx().request_repaint_after(Duration::from_millis(40));
        }
        egui::CentralPanel::default().show(ui, |ui| {
            let rect = ui.max_rect();
            self.rect = rect;
            let (_, input) = input::area(ui, ui.id().with("image"), rect, &KEYS);
            let at = input
                .pointer
                .map(|p| rect.min + p.to_vec2())
                .unwrap_or(rect.center());
            let size = self.size().map(|[w, h]| Vec2::new(w as f32, h as f32));
            for (key, _) in &input.keys {
                let scale = self.scale(rect);
                let drawn = size.map(|s| s * scale).unwrap_or(Vec2::ZERO);
                let (wide, tall) = (drawn.x > rect.width() + 0.5, drawn.y > rect.height() + 0.5);
                let pan = rect.size() / 8.0;
                match key {
                    Key::ArrowLeft if wide => self.offset.x += pan.x,
                    Key::ArrowRight if wide => self.offset.x -= pan.x,
                    Key::ArrowUp if tall => self.offset.y += pan.y,
                    Key::ArrowDown if tall => self.offset.y -= pan.y,
                    Key::ArrowLeft | Key::PageUp => {
                        host.requests.push(Request::Browse(Step::Previous))
                    }
                    Key::ArrowRight | Key::PageDown => {
                        host.requests.push(Request::Browse(Step::Next))
                    }
                    Key::Home => host.requests.push(Request::Browse(Step::First)),
                    Key::End => host.requests.push(Request::Browse(Step::Last)),
                    Key::Plus | Key::Equals => self.zoom_at(rect, rect.center(), step(scale, true)),
                    Key::Minus => self.zoom_at(rect, rect.center(), step(scale, false)),
                    Key::Num0 => self.fit_now(),
                    Key::Num1 => self.zoom_at(rect, rect.center(), 1.0),
                    Key::Num2 => self.zoom_at(rect, rect.center(), 2.0),
                    Key::Num3 => self.zoom_at(rect, rect.center(), 3.0),
                    Key::Num4 => self.zoom_at(rect, rect.center(), 4.0),
                    Key::Space => self.play(),
                    Key::Comma => self.turn(-1),
                    Key::Period => self.turn(1),
                    _ => {}
                }
            }
            let Some(size) = size else {
                let text = match &self.error {
                    Some(e) => e.clone(),
                    None => tr("Decoding...").to_owned(),
                };
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    text,
                    egui::TextStyle::Body.resolve(ui.style()),
                    ui.visuals().text_color(),
                );
                return;
            };
            let mut factor = input.zoom;
            if input.wheel.y != 0.0 {
                factor *= (-input.wheel.y * 0.002).exp();
            }
            if (factor - 1.0).abs() > 1e-4 {
                let scale = self.scale(rect);
                self.zoom_at(rect, at, scale * factor);
            }
            if input.double_clicked {
                if self.zoom == Zoom::Fit {
                    self.zoom_at(rect, at, 1.0);
                } else {
                    self.fit_now();
                }
            }
            if input.dragging && input.drag != Vec2::ZERO {
                if self.zoom == Zoom::Fit {
                    self.zoom = Zoom::Scale(self.fit(rect));
                }
                self.offset += input.drag;
            }
            let scale = self.scale(rect);
            self.keep_in(rect, scale);
            self.drawn = scale;
            let drawn = size * scale;
            if input.dragging {
                input::cursor(ui, Cursor::Grabbing);
            } else if input.hovered && (drawn.x > rect.width() || drawn.y > rect.height()) {
                input::cursor(ui, Cursor::Grab);
            }
            let image = Rect::from_center_size(rect.center() + self.offset, drawn);
            self.animate(ui.ctx());
            if self.frames().is_some() {
                self.draw_frame(ui, rect, image);
            } else {
                self.draw_tiles(ui, rect, image);
            }
        });
    }

    fn tools(&self) -> Vec<Tool> {
        let mut tools = vec![
            Tool {
                id: "fit",
                icon: "\u{1F5BC}",
                label: "Fit",
            },
            Tool {
                id: "actual",
                icon: "\u{1F4AF}",
                label: "100%",
            },
            Tool {
                id: "in",
                icon: "\u{2795}",
                label: "Zoom in",
            },
            Tool {
                id: "out",
                icon: "\u{2796}",
                label: "Zoom out",
            },
        ];
        if self.frames().is_some_and(|f| f.len() > 1) {
            tools.push(if self.playing {
                Tool {
                    id: "play",
                    icon: "\u{23F8}",
                    label: "Pause",
                }
            } else {
                Tool {
                    id: "play",
                    icon: "\u{25B6}",
                    label: "Play",
                }
            });
            tools.push(Tool {
                id: "previous",
                icon: "\u{23EE}",
                label: "Previous frame",
            });
            tools.push(Tool {
                id: "next",
                icon: "\u{23ED}",
                label: "Next frame",
            });
        } else if self.pages().is_some() {
            tools.push(Tool {
                id: "previous",
                icon: "\u{23EB}",
                label: "Previous page",
            });
            tools.push(Tool {
                id: "next",
                icon: "\u{23EC}",
                label: "Next page",
            });
        }
        tools
    }

    fn act(&mut self, id: &str, _host: &mut Host) {
        let rect = self.rect;
        let scale = self.scale(rect);
        match id {
            "fit" => self.fit_now(),
            "actual" => self.zoom_at(rect, rect.center(), 1.0),
            "in" => self.zoom_at(rect, rect.center(), step(scale, true)),
            "out" => self.zoom_at(rect, rect.center(), step(scale, false)),
            "play" => self.play(),
            "previous" => self.turn(-1),
            "next" => self.turn(1),
            _ => {}
        }
    }

    /// An image of a file steps through its folder; one of bytes has none.
    fn browses(&self) -> bool {
        matches!(self.source, Source::File(_))
    }

    fn status(&self) -> String {
        let Some(image) = &self.image else {
            return match &self.error {
                Some(e) => e.clone(),
                None => tr("Decoding...").to_owned(),
            };
        };
        let mut line = trf(
            "{w} x {h}, {format}, {zoom}%",
            &[
                ("w", &image.size[0]),
                ("h", &image.size[1]),
                ("format", &image.format),
                ("zoom", &format!("{:.0}", self.drawn * 100.0)),
            ],
        );
        let mut add = |text: String| {
            line.push_str(", ");
            line.push_str(&text);
        };
        if let Some(frames) = self.frames() {
            add(trf(
                "frame {n} of {count}",
                &[("n", &(self.index + 1)), ("count", &frames.len())],
            ));
            if frames.len() < image.count as usize {
                add(trf(
                    "the first {n} of {count} frames",
                    &[("n", &frames.len()), ("count", &image.count)],
                ));
            }
        } else if let Some((page, count)) = self.pages() {
            add(trf(
                "page {n} of {count}",
                &[("n", &(page + 1)), ("count", &count)],
            ));
        }
        let shown = image.shown();
        if shown != image.size {
            add(trf(
                "decoded at {w} x {h}",
                &[("w", &shown[0]), ("h", &shown[1])],
            ));
        }
        if image.turned {
            add(tr("turned by its EXIF orientation").to_owned());
        }
        if self.job.is_some() {
            add(tr("Decoding...").to_owned());
        } else if let Some(e) = &self.error {
            add(e.clone());
        }
        line
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plus_and_minus_go_by_the_levels() {
        assert_eq!(step(1.0, true), 1.5);
        assert_eq!(step(1.0, false), 2.0 / 3.0);
        assert_eq!(
            step(1.2, true),
            1.5,
            "a zoom of the wheel goes to the next level"
        );
        assert_eq!(step(1.2, false), 1.0);
        assert_eq!(step(32.0, true), 32.0);
        assert_eq!(step(0.02, false), 0.02);
    }

    #[test]
    fn the_level_has_a_pixel_for_every_pixel_of_the_screen() {
        let widths = [8000, 4000, 2000];
        assert_eq!(level(&widths, 9000.0), 0);
        assert_eq!(level(&widths, 4000.0), 1);
        assert_eq!(level(&widths, 3000.0), 1);
        assert_eq!(level(&widths, 500.0), 2);
    }

    #[test]
    fn the_tiles_a_view_shows() {
        let size = [3000, 1500];
        // At 100%, the image from 0 to 3000, the view over its middle.
        let image = Rect::from_min_size(Pos2::ZERO, Vec2::new(3000.0, 1500.0));
        let view = Rect::from_min_max(Pos2::new(1000.0, 100.0), Pos2::new(2100.0, 900.0));
        assert_eq!(visible(size, image, view), vec![[0, 0], [1, 0], [2, 0]]);
        let corner = Rect::from_min_max(Pos2::new(2900.0, 1400.0), Pos2::new(5000.0, 5000.0));
        assert_eq!(visible(size, image, corner), vec![[2, 1]]);
        let away = Rect::from_min_max(Pos2::new(4000.0, 0.0), Pos2::new(5000.0, 10.0));
        assert!(visible(size, image, away).is_empty());
    }

    #[test]
    fn a_tile_carries_a_pixel_of_its_neighbours() {
        let picture = Picture {
            pixels: vec![255; 1500 * 2 * 4],
            size: [1500, 2],
        };
        let (first, uv) = tile(&picture, [0, 0]);
        assert_eq!(first.size, [TILE as usize + 1, 2]);
        assert_eq!(uv.min, Pos2::ZERO);
        assert!((uv.max.x - TILE as f32 / (TILE + 1) as f32).abs() < 1e-6);
        let (second, uv) = tile(&picture, [1, 0]);
        assert_eq!(second.size, [1500 - TILE as usize + 1, 2]);
        assert!((uv.min.x - 1.0 / second.size[0] as f32).abs() < 1e-6);
        assert_eq!(uv.max.x, 1.0);
    }
}
