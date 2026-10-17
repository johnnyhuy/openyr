//! OpenYR window: a live skirmish session over a rendered retail map.
//!
//! Usage: `openyr <gamedir> <map> [scale]`
//! Left-click / box-drag select own units, right-click move or attack
//! (enemy under cursor), `A` then right-click = attack-move, `S` stop,
//! `H` harvest, right-drag / arrows pan, wheel or +/- zoom,
//! Esc quit. Right-hand sidebar: left-click a cameo queues it at the
//! war factory/barracks, right-click cancels (refund). Orders go through `yr_sim::Command` only — the UI never
//! mutates sim state directly (issue #15).

use pixels::{Pixels, SurfaceTexture};
use std::collections::BTreeSet;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};
use yr_app::session::{Outcome, Session};
use yr_sim::{Command, Fix};

const WIN_W: u32 = 1280;
const WIN_H: u32 = 800;
const TICK: Duration = Duration::from_millis(1000 / 15);

const PLAYER_COL: [u8; 4] = [64, 255, 128, 255];
const ENEMY_COL: [u8; 4] = [255, 64, 64, 255];
const NEUTRAL_COL: [u8; 4] = [180, 180, 180, 255];
const SELECT_COL: [u8; 4] = [255, 255, 255, 255];

/// Production sidebar: two columns of 60x48 cameos on the right edge.
const SIDEBAR_W: u32 = 136;
const CAMEO_W: i32 = 60;
const CAMEO_H: i32 = 48;
const CAMEO_GAP: i32 = 4;
const SIDEBAR_TOP: i32 = 34;

struct View {
    cam_x: f32,
    cam_y: f32,
    zoom: f32,
    dirty: bool,
}

struct App {
    gamedir: String,
    map_name: String,
    scale: u32,
    window: Option<std::sync::Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    map: Option<yr_render::RenderedMap>,
    session: Option<Session>,
    view: View,
    selected: BTreeSet<u32>,
    pending: Vec<Command>,
    attack_move: bool,
    shift: bool,
    cursor: (f64, f64),
    left_down: Option<(f64, f64)>,
    right_down: Option<(f64, f64)>,
    right_panning: bool,
    last_cursor: (f64, f64),
    keys: std::collections::HashSet<KeyCode>,
    last_tick: Instant,
    accum: Duration,
    title: String,
    /// The match result has been printed once.
    announced: bool,
}

impl App {
    fn window_to_map(&self, pos: (f64, f64)) -> (f32, f32) {
        (
            self.view.cam_x + pos.0 as f32 / self.view.zoom,
            self.view.cam_y + pos.1 as f32 / self.view.zoom,
        )
    }

    fn window_to_cell(&self, pos: (f64, f64)) -> Option<(f32, f32)> {
        let map = self.map.as_ref()?;
        let (mx, my) = self.window_to_map(pos);
        Some(map.px_to_cell(mx, my))
    }

    fn cell_leptons(cell: (f32, f32)) -> (Fix, Fix) {
        let c = |v: f32| Fix::from_int(v.floor() as i32) + Fix::from_raw(128);
        (c(cell.0), c(cell.1))
    }

    /// Enemy (or neutral) entity under a cell: buildings by footprint first.
    fn target_at(&self, cell: (f32, f32)) -> Option<u32> {
        let sim = &self.session.as_ref()?.sim;
        let (cx, cy) = (cell.0.floor() as i32, cell.1.floor() as i32);
        for b in sim.buildings() {
            if b.owner != yr_sim::OWNER_PLAYER
                && cx >= i32::from(b.cell.0)
                && cx < i32::from(b.cell.0) + i32::from(b.size.0)
                && cy >= i32::from(b.cell.1)
                && cy < i32::from(b.cell.1) + i32::from(b.size.1)
            {
                return Some(b.id);
            }
        }
        sim.units()
            .iter()
            .filter(|u| u.owner != yr_sim::OWNER_PLAYER)
            .find(|u| u.x.to_int_floor() == cx && u.y.to_int_floor() == cy)
            .map(|u| u.id)
    }

    fn select_at(&mut self, pos: (f64, f64), additive: bool) {
        let Some(cell) = self.window_to_cell(pos) else {
            return;
        };
        if !additive {
            self.selected.clear();
        }
        if let Some(session) = &self.session {
            let (cx, cy) = (cell.0.floor() as i32, cell.1.floor() as i32);
            if let Some(u) = session.sim.units().iter().find(|u| {
                u.owner == yr_sim::OWNER_PLAYER
                    && u.x.to_int_floor() == cx
                    && u.y.to_int_floor() == cy
            }) {
                self.selected.insert(u.id);
            }
        }
        self.view.dirty = true;
    }

    fn select_box(&mut self, a: (f64, f64), b: (f64, f64), additive: bool) {
        if !additive {
            self.selected.clear();
        }
        let (Some(map), Some(session)) = (&self.map, &self.session) else {
            return;
        };
        let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
        let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
        for u in session.sim.units() {
            if u.owner != yr_sim::OWNER_PLAYER {
                continue;
            }
            let (mx, my) = map.cell_to_px(
                u.x.to_int_floor() as f32 + 0.5,
                u.y.to_int_floor() as f32 + 0.5,
            );
            let (wx, wy) = (
                (mx - self.view.cam_x) * self.view.zoom,
                (my - self.view.cam_y) * self.view.zoom,
            );
            if f64::from(wx) >= x0
                && f64::from(wx) <= x1
                && f64::from(wy) >= y0
                && f64::from(wy) <= y1
            {
                self.selected.insert(u.id);
            }
        }
        self.view.dirty = true;
    }

    fn issue_order(&mut self, pos: (f64, f64)) {
        let Some(cell) = self.window_to_cell(pos) else {
            return;
        };
        if cell.0 < 0.0 || cell.1 < 0.0 || self.selected.is_empty() {
            self.attack_move = false;
            return;
        }
        let target = self.target_at(cell);
        let (lx, ly) = Self::cell_leptons(cell);
        for &id in &self.selected {
            let cmd = match (self.attack_move, target) {
                (_, Some(t)) => Command::Attack { id, target: t },
                (true, None) => Command::AttackMove { id, x: lx, y: ly },
                (false, None) => Command::Move { id, x: lx, y: ly },
            };
            self.pending.push(cmd);
        }
        self.attack_move = false;
    }

    /// Sidebar slot under a frame-space position.
    fn sidebar_slot_at(&self, pos: (f64, f64)) -> Option<usize> {
        let session = self.session.as_ref()?;
        let fw = self.pixels.as_ref()?.texture().width();
        (0..session.buildables.len()).find(|&i| {
            let (x, y) = slot_origin(fw, i);
            pos.0 >= f64::from(x)
                && pos.0 < f64::from(x + CAMEO_W)
                && pos.1 >= f64::from(y)
                && pos.1 < f64::from(y + CAMEO_H)
        })
    }

    fn in_sidebar(&self, pos: (f64, f64)) -> bool {
        self.pixels
            .as_ref()
            .is_some_and(|p| pos.0 >= f64::from(p.texture().width() - SIDEBAR_W))
    }

    fn tick_session(&mut self) {
        let Some(session) = &mut self.session else {
            return;
        };
        let cmds = std::mem::take(&mut self.pending);
        session.step(&cmds);
        let _ = session.sim.drain_events(); // audio lands in #17
        self.selected
            .retain(|id| session.sim.units().iter().any(|u| u.id == *id));
        let (launched, total) = session.waves_progress();
        let waves = match session.next_wave_in_secs() {
            Some(s) => format!("wave {}/{total} in {s}s", launched + 1),
            None => format!("waves {launched}/{total}"),
        };
        let title = format!(
            "OpenYR — {} | ${} | tick {} | {waves} | {} selected{} | {:?}",
            self.map_name,
            session.sim.credits(),
            session.sim.tick(),
            self.selected.len(),
            if self.attack_move {
                " | ATTACK-MOVE"
            } else {
                ""
            },
            session.outcome,
        );
        if title != self.title {
            self.title = title.clone();
            if session.outcome != Outcome::Ongoing && !self.announced {
                self.announced = true;
                println!("OpenYR: match {title}");
            }
            if let Some(w) = &self.window {
                w.set_title(&title);
            }
        }
        self.view.dirty = true;
    }

    fn blit(&mut self) {
        let Some(mut pixels) = self.pixels.take() else {
            return;
        };
        let (fw, fh) = (pixels.texture().width(), pixels.texture().height());
        self.compose(pixels.frame_mut(), fw, fh);
        self.pixels = Some(pixels);
        self.view.dirty = false;
    }

    /// Draw the full window frame (map, markers, sidebar, rubber band)
    /// into an RGBA buffer; shared by the window and `--frame` dumps.
    fn compose(&self, frame: &mut [u8], fw: u32, fh: u32) {
        let Some(map) = &self.map else {
            return;
        };
        let (mw, mh) = (map.width as i32, map.height as i32);
        // Copy the terrain frame through the viewport, then collect
        // entity markers (needs `map` borrowed while frame is mutably
        // borrowed, so gather screen positions first).
        struct Marker {
            wx: f32,
            wy: f32,
            half_w: f32,
            col: [u8; 4],
            selected: bool,
            hp: f32,
        }
        let mut markers: Vec<Marker> = Vec::new();
        if let Some(session) = &self.session {
            let sim = &session.sim;
            let zoom = self.view.zoom;
            let to_win = |map: &yr_render::RenderedMap, cx: f32, cy: f32, view: &View| {
                let (mx, my) = map.cell_to_px(cx, cy);
                ((mx - view.cam_x) * zoom, (my - view.cam_y) * zoom)
            };
            for b in sim.buildings() {
                // Neutral map structures are already drawn as art; mark
                // them only once they take damage.
                if b.owner == yr_sim::OWNER_NEUTRAL && b.hp >= b.max_hp {
                    continue;
                }
                let (bx, by) = b.centre();
                let (wx, wy) = to_win(
                    map,
                    bx.to_int_floor() as f32 + 0.5,
                    by.to_int_floor() as f32 + 0.5,
                    &self.view,
                );
                let half = ((f32::from(b.size.0 + b.size.1)) * 15.0 / map.scale.max(1) as f32
                    * zoom)
                    .max(8.0);
                markers.push(Marker {
                    wx,
                    wy,
                    half_w: half,
                    col: owner_col(b.owner),
                    selected: false,
                    hp: hp_frac(b.hp, b.max_hp),
                });
            }
            for u in sim.units() {
                if u.owner == yr_sim::OWNER_NEUTRAL && u.hp >= u.max_hp {
                    continue;
                }
                let (wx, wy) = to_win(
                    map,
                    u.x.to_int_floor() as f32 + 0.5,
                    u.y.to_int_floor() as f32 + 0.5,
                    &self.view,
                );
                markers.push(Marker {
                    wx,
                    wy,
                    half_w: (14.0 * zoom).max(5.0),
                    col: owner_col(u.owner),
                    selected: self.selected.contains(&u.id),
                    hp: hp_frac(u.hp, u.max_hp),
                });
            }
        }
        for sy in 0..fh as i32 {
            let my = self.view.cam_y + sy as f32 / self.view.zoom;
            for sx in 0..fw as i32 {
                let mx = self.view.cam_x + sx as f32 / self.view.zoom;
                let dst = ((sy * fw as i32 + sx) * 4) as usize;
                if mx >= 0.0 && my >= 0.0 && (mx as i32) < mw && (my as i32) < mh {
                    let src = ((my as i32 * mw + mx as i32) * 4) as usize;
                    frame[dst..dst + 4].copy_from_slice(&map.rgba[src..src + 4]);
                } else {
                    frame[dst..dst + 4].copy_from_slice(&[16, 16, 20, 255]);
                }
            }
        }
        for m in &markers {
            draw_diamond(frame, fw, fh, m.wx, m.wy, m.half_w, m.col);
            if m.selected {
                draw_diamond_outline(frame, fw, fh, m.wx, m.wy, m.half_w + 4.0, SELECT_COL);
            }
            draw_health(
                frame,
                fw,
                fh,
                m.wx,
                m.wy - m.half_w * 0.5 - 8.0,
                m.half_w * 2.0,
                m.hp,
            );
        }
        if let Some(session) = &self.session {
            draw_sidebar(frame, fw, fh, session);
        }
        // Box-select rubber band.
        if let (Some(a), false) = (self.left_down, self.right_panning) {
            if (self.cursor.0 - a.0).abs() > 6.0 || (self.cursor.1 - a.1).abs() > 6.0 {
                draw_rect_outline(frame, fw, fh, a, self.cursor, SELECT_COL);
            }
        }
    }
}

fn slot_origin(fw: u32, slot: usize) -> (i32, i32) {
    let col = (slot % 2) as i32;
    let row = (slot / 2) as i32;
    (
        fw as i32 - SIDEBAR_W as i32 + 6 + col * (CAMEO_W + CAMEO_GAP),
        SIDEBAR_TOP + row * (CAMEO_H + CAMEO_GAP),
    )
}

/// Dark panel, credits readout, then one cameo per buildable with its
/// queue count and a build-progress wipe; slots whose factory is gone
/// are dimmed.
fn draw_sidebar(frame: &mut [u8], fw: u32, fh: u32, session: &Session) {
    let x0 = (fw - SIDEBAR_W) as i32;
    for y in 0..fh as i32 {
        for x in x0..fw as i32 {
            set_px(frame, fw, fh, x, y, [24, 26, 32, 255]);
        }
    }
    draw_number(
        frame,
        fw,
        fh,
        x0 + 8,
        10,
        session.sim.credits(),
        [255, 220, 96, 255],
    );
    let sim = &session.sim;
    for (i, b) in session.buildables.iter().enumerate() {
        let (sx, sy) = slot_origin(fw, i);
        let host = sim
            .buildings()
            .iter()
            .any(|h| h.owner == yr_sim::OWNER_PLAYER && h.role == b.factory.role());
        if let Some((w, h, px)) = &b.cameo {
            for y in 0..(*h as i32).min(CAMEO_H) {
                for x in 0..(*w as i32).min(CAMEO_W) {
                    let at = ((y as u32 * w + x as u32) * 4) as usize;
                    let mut c = [px[at], px[at + 1], px[at + 2], 255];
                    if px[at + 3] == 0 {
                        c = [0, 0, 0, 255];
                    }
                    set_px(frame, fw, fh, sx + x, sy + y, c);
                }
            }
        } else {
            for y in 0..CAMEO_H {
                for x in 0..CAMEO_W {
                    set_px(frame, fw, fh, sx + x, sy + y, [70, 70, 80, 255]);
                }
            }
        }
        let (count, head) = session.slot_status(i);
        // Unbuilt share of the head order darkens from the top.
        let shade_rows = match head {
            Some((done, total)) if total > 0 => {
                (i64::from(CAMEO_H) * (total - done) / total) as i32
            }
            _ if !host => CAMEO_H,
            _ => 0,
        };
        for y in 0..shade_rows {
            for x in 0..CAMEO_W {
                darken(frame, fw, fh, sx + x, sy + y);
            }
        }
        if count > 0 {
            draw_number(
                frame,
                fw,
                fh,
                sx + 3,
                sy + 3,
                count as i64,
                [255, 255, 255, 255],
            );
        }
    }
}

fn darken(frame: &mut [u8], fw: u32, fh: u32, x: i32, y: i32) {
    if x >= 0 && y >= 0 && (x as u32) < fw && (y as u32) < fh {
        let at = ((y as u32 * fw + x as u32) * 4) as usize;
        for c in &mut frame[at..at + 3] {
            *c /= 3;
        }
    }
}

/// 3x5 digit glyphs, one row per u8 (bit 2 = left column).
const DIGITS: [[u8; 5]; 10] = [
    [7, 5, 5, 5, 7],
    [2, 6, 2, 2, 7],
    [7, 1, 7, 4, 7],
    [7, 1, 7, 1, 7],
    [5, 5, 7, 1, 1],
    [7, 4, 7, 1, 7],
    [7, 4, 7, 5, 7],
    [7, 1, 1, 1, 1],
    [7, 5, 7, 5, 7],
    [7, 5, 7, 1, 7],
];

/// Draw a non-negative number at 2x scale with a dark backing.
fn draw_number(frame: &mut [u8], fw: u32, fh: u32, x: i32, y: i32, n: i64, col: [u8; 4]) {
    let text = n.max(0).to_string();
    let w = text.len() as i32 * 8;
    for dy in -1..11 {
        for dx in -1..w {
            set_px(frame, fw, fh, x + dx, y + dy, [0, 0, 0, 255]);
        }
    }
    for (k, ch) in text.bytes().enumerate() {
        let glyph = DIGITS[usize::from(ch - b'0')];
        for (row, bits) in glyph.iter().enumerate() {
            for colx in 0..3 {
                if bits & (4 >> colx) != 0 {
                    for (ox, oy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                        let px = x + k as i32 * 8 + colx * 2 + ox;
                        set_px(frame, fw, fh, px, y + row as i32 * 2 + oy, col);
                    }
                }
            }
        }
    }
}

fn owner_col(owner: u8) -> [u8; 4] {
    match owner {
        yr_sim::OWNER_PLAYER => PLAYER_COL,
        yr_sim::OWNER_ENEMY => ENEMY_COL,
        _ => NEUTRAL_COL,
    }
}

fn hp_frac(hp: i64, max: i64) -> f32 {
    if max <= 0 {
        0.0
    } else {
        (hp as f32 / max as f32).clamp(0.0, 1.0)
    }
}

fn set_px(frame: &mut [u8], fw: u32, fh: u32, x: i32, y: i32, col: [u8; 4]) {
    if x >= 0 && y >= 0 && (x as u32) < fw && (y as u32) < fh {
        let at = ((y as u32 * fw + x as u32) * 4) as usize;
        frame[at..at + 4].copy_from_slice(&col);
    }
}

fn draw_diamond(frame: &mut [u8], fw: u32, fh: u32, cx: f32, cy: f32, half_w: f32, col: [u8; 4]) {
    let half_h = (half_w * 0.5).max(1.0);
    for dy in -(half_h as i32)..=(half_h as i32) {
        // Width shrinks linearly to the diamond points.
        let w = (half_w - (dy as f32).abs() * half_w / half_h).max(0.0);
        for dx in -(w as i32)..=(w as i32) {
            set_px(frame, fw, fh, cx as i32 + dx, cy as i32 + dy, col);
        }
    }
}

fn draw_diamond_outline(
    frame: &mut [u8],
    fw: u32,
    fh: u32,
    cx: f32,
    cy: f32,
    half_w: f32,
    col: [u8; 4],
) {
    let half_h = half_w * 0.5;
    for dy in -half_h as i32..=half_h as i32 {
        let w = (half_w - (dy as f32).abs() * half_w / half_h.max(1.0)).max(0.0);
        set_px(frame, fw, fh, cx as i32 - w as i32, cy as i32 + dy, col);
        set_px(frame, fw, fh, cx as i32 + w as i32, cy as i32 + dy, col);
    }
}

fn draw_health(frame: &mut [u8], fw: u32, fh: u32, cx: f32, cy: f32, width: f32, frac: f32) {
    let w = width.max(10.0) as i32;
    let filled = (w as f32 * frac) as i32;
    let col = if frac > 0.5 {
        [64, 255, 64, 255]
    } else if frac > 0.25 {
        [255, 200, 64, 255]
    } else {
        [255, 64, 64, 255]
    };
    for dx in -w / 2..=w / 2 {
        set_px(frame, fw, fh, cx as i32 + dx, cy as i32, [32, 32, 32, 255]);
        if dx + w / 2 <= filled {
            set_px(frame, fw, fh, cx as i32 + dx, cy as i32 + 1, col);
        }
    }
}

fn draw_rect_outline(
    frame: &mut [u8],
    fw: u32,
    fh: u32,
    a: (f64, f64),
    b: (f64, f64),
    col: [u8; 4],
) {
    let (x0, x1) = (a.0.min(b.0) as i32, a.0.max(b.0) as i32);
    let (y0, y1) = (a.1.min(b.1) as i32, a.1.max(b.1) as i32);
    for x in x0..=x1 {
        set_px(frame, fw, fh, x, y0, col);
        set_px(frame, fw, fh, x, y1, col);
    }
    for y in y0..=y1 {
        set_px(frame, fw, fh, x0, y, col);
        set_px(frame, fw, fh, x1, y, col);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title(format!("OpenYR — {}", self.map_name))
            .with_inner_size(LogicalSize::new(WIN_W, WIN_H));
        let window = std::sync::Arc::new(el.create_window(attrs).expect("create window"));
        let size = window.inner_size();
        let surface = SurfaceTexture::new(size.width.max(1), size.height.max(1), window.clone());
        let pixels = Pixels::new(WIN_W, WIN_H, surface).expect("pixels surface");
        match yr_render::render_map(&self.gamedir, &self.map_name, self.scale) {
            Ok(map) => {
                println!("OpenYR: rendered {}", map.summary);
                self.view.cam_x = (map.width as f32 - WIN_W as f32) / 2.0;
                self.view.cam_y = (map.height as f32 - WIN_H as f32) / 2.0;
                self.map = Some(map);
            }
            Err(e) => {
                eprintln!("OpenYR: cannot render {}: {e}", self.map_name);
                el.exit();
                return;
            }
        }
        match yr_app::session::load(&self.gamedir, &self.map_name, 42) {
            Ok(s) => {
                println!(
                    "OpenYR: session live — {} units, {} buildings, ${}",
                    s.sim.units().len(),
                    s.sim.buildings().len(),
                    s.sim.credits()
                );
                self.session = Some(s);
            }
            Err(e) => eprintln!("OpenYR: session unavailable ({e}); view-only"),
        }
        println!("OpenYR: left-click/box select, right-click order, A=attack-move, S=stop, H=harvest, right-drag/arrows pan, wheel zoom; sidebar: left-click build, right-click cancel");
        self.window = Some(window);
        self.pixels = Some(pixels);
        self.view.dirty = true;
        self.last_tick = Instant::now();
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(size) => {
                if let Some(p) = &mut self.pixels {
                    let _ = p.resize_surface(size.width.max(1), size.height.max(1));
                    self.view.dirty = true;
                }
            }
            WindowEvent::ModifiersChanged(m) => {
                self.shift = m.state().shift_key();
            }
            WindowEvent::MouseInput { state, button, .. } => match (button, state) {
                (MouseButton::Left, ElementState::Pressed) if self.in_sidebar(self.cursor) => {
                    if let Some(cmd) = self
                        .sidebar_slot_at(self.cursor)
                        .and_then(|i| self.session.as_ref()?.produce(i))
                    {
                        self.pending.push(cmd);
                    }
                }
                (MouseButton::Right, ElementState::Pressed) if self.in_sidebar(self.cursor) => {
                    if let Some(cmd) = self
                        .sidebar_slot_at(self.cursor)
                        .and_then(|i| self.session.as_ref()?.cancel(i))
                    {
                        self.pending.push(cmd);
                    }
                }
                (MouseButton::Left, ElementState::Pressed) => self.left_down = Some(self.cursor),
                (MouseButton::Left, ElementState::Released) => {
                    if let Some(a) = self.left_down.take() {
                        if (self.cursor.0 - a.0).abs() > 6.0 || (self.cursor.1 - a.1).abs() > 6.0 {
                            self.select_box(a, self.cursor, self.shift);
                        } else {
                            self.select_at(self.cursor, self.shift);
                        }
                    }
                }
                (MouseButton::Right, ElementState::Pressed) => {
                    self.right_down = Some(self.cursor);
                    self.right_panning = false;
                }
                (MouseButton::Right, ElementState::Released) => {
                    if self.right_down.take().is_some() && !self.right_panning {
                        self.issue_order(self.cursor);
                    }
                    self.right_panning = false;
                }
                _ => {}
            },
            WindowEvent::CursorMoved { position, .. } => {
                // Work in frame pixels (the surface may be HiDPI-scaled).
                let position = match &self.pixels {
                    Some(p) => {
                        let (x, y) =
                            match p.window_pos_to_pixel((position.x as f32, position.y as f32)) {
                                Ok((x, y)) => (x as f64, y as f64),
                                Err((x, y)) => (x as f64, y as f64),
                            };
                        winit::dpi::PhysicalPosition::new(x, y)
                    }
                    None => position,
                };
                self.cursor = (position.x, position.y);
                if self.right_down.is_some() {
                    let dx = (position.x - self.last_cursor.0) as f32;
                    let dy = (position.y - self.last_cursor.1) as f32;
                    if dx.abs() + dy.abs() > 2.0 {
                        self.right_panning = true;
                    }
                    if self.right_panning {
                        self.view.cam_x -= dx / self.view.zoom;
                        self.view.cam_y -= dy / self.view.zoom;
                        self.view.dirty = true;
                    }
                }
                if self.left_down.is_some() {
                    self.view.dirty = true;
                }
                self.last_cursor = (position.x, position.y);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let d = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => (p.y / 50.0) as f32,
                };
                self.view.zoom = (self.view.zoom * (1.0 + d * 0.1)).clamp(0.25, 4.0);
                self.view.dirty = true;
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            self.keys.insert(code);
                            match code {
                                KeyCode::Escape => {
                                    if self.attack_move {
                                        self.attack_move = false;
                                    } else {
                                        el.exit();
                                    }
                                }
                                KeyCode::Equal => self.view.zoom = (self.view.zoom * 1.2).min(4.0),
                                KeyCode::Minus => self.view.zoom = (self.view.zoom / 1.2).max(0.25),
                                KeyCode::KeyA => self.attack_move = true,
                                KeyCode::KeyS => {
                                    for &id in &self.selected {
                                        self.pending.push(Command::Stop { id });
                                    }
                                }
                                KeyCode::KeyH => {
                                    for &id in &self.selected {
                                        self.pending.push(Command::Harvest { id });
                                    }
                                }
                                _ => {}
                            }
                            self.view.dirty = true;
                        }
                        ElementState::Released => {
                            self.keys.remove(&code);
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                // Fixed 15 tps sim, decoupled from frame rate.
                let now = Instant::now();
                self.accum += now.duration_since(self.last_tick);
                self.last_tick = now;
                let mut stepped = false;
                while self.accum >= TICK {
                    self.accum -= TICK;
                    if self.session.is_some() {
                        self.tick_session();
                        stepped = true;
                    }
                }
                // Camera: arrows pan (A/S are order hotkeys, right-drag pans).
                let step = 18.0 / self.view.zoom;
                for (k, dx, dy) in [
                    (KeyCode::ArrowLeft, -step, 0.0),
                    (KeyCode::ArrowRight, step, 0.0),
                    (KeyCode::ArrowUp, 0.0, -step),
                    (KeyCode::ArrowDown, 0.0, step),
                ] {
                    if self.keys.contains(&k) {
                        self.view.cam_x += dx;
                        self.view.cam_y += dy;
                        self.view.dirty = true;
                    }
                }
                if self.view.dirty || stepped {
                    self.blit();
                }
                if let Some(p) = &mut self.pixels {
                    let _ = p.render();
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn new_app(gamedir: String, map_name: String, scale: u32) -> App {
    App {
        gamedir,
        map_name,
        scale,
        window: None,
        pixels: None,
        map: None,
        session: None,
        view: View {
            cam_x: 0.0,
            cam_y: 0.0,
            zoom: 1.0,
            dirty: true,
        },
        selected: BTreeSet::new(),
        pending: Vec::new(),
        attack_move: false,
        shift: false,
        cursor: (0.0, 0.0),
        left_down: None,
        right_down: None,
        right_panning: false,
        last_cursor: (0.0, 0.0),
        keys: std::collections::HashSet::new(),
        last_tick: Instant::now(),
        accum: Duration::ZERO,
        title: String::new(),
        announced: false,
    }
}

/// `--frames <dir> <tick>...`: play a scripted demo headlessly and dump
/// the exact window frame at each tick. Script: queue Grizzlies and GIs
/// at tick 1, then every 450 ticks send the whole player army (armed
/// units) to attack-move on the enemy base; the camera follows the army.
fn headless_frames(mut app: App, dir: &str, ticks: &[u64]) -> Result<(), String> {
    app.map = Some(yr_render::render_map(
        &app.gamedir,
        &app.map_name,
        app.scale,
    )?);
    app.session = Some(yr_app::session::load(&app.gamedir, &app.map_name, 42)?);
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let (fw, fh) = (WIN_W, WIN_H);
    let last = ticks.iter().copied().max().unwrap_or(0);
    let mut frame = vec![0u8; (fw * fh * 4) as usize];
    for t in 1..=last {
        let session = app.session.as_ref().ok_or("no session")?;
        if t == 1 {
            for slot in [0, 0, 0, 4, 4, 4, 1, 5] {
                if let Some(cmd) = session.produce(slot) {
                    app.pending.push(cmd);
                }
            }
        }
        let army: Vec<u32> = session
            .sim
            .units()
            .iter()
            .filter(|u| u.owner == yr_sim::OWNER_PLAYER && u.weapon.is_some())
            .map(|u| u.id)
            .collect();
        if t % 450 == 0 {
            let enemy: Vec<_> = session
                .sim
                .buildings()
                .iter()
                .filter(|b| b.owner == yr_sim::OWNER_ENEMY)
                .map(|b| b.centre())
                .collect();
            if let Some(&(x, y)) = enemy.first() {
                for &id in &army {
                    app.pending.push(Command::AttackMove { id, x, y });
                }
            }
        }
        app.selected = army.iter().copied().collect();
        app.tick_session();
        if ticks.contains(&t) {
            let session = app.session.as_ref().ok_or("no session")?;
            let map = app.map.as_ref().ok_or("no map")?;
            let sim = &session.sim;
            let mine: Vec<_> = sim
                .units()
                .iter()
                .filter(|u| u.owner == yr_sim::OWNER_PLAYER)
                .collect();
            if !mine.is_empty() {
                let n = mine.len() as f32;
                let cx = mine.iter().map(|u| u.x.to_int_floor() as f32).sum::<f32>() / n;
                let cy = mine.iter().map(|u| u.y.to_int_floor() as f32).sum::<f32>() / n;
                let (mx, my) = map.cell_to_px(cx + 0.5, cy + 0.5);
                app.view.cam_x = mx - (fw - SIDEBAR_W) as f32 / 2.0 / app.view.zoom;
                app.view.cam_y = my - fh as f32 / 2.0 / app.view.zoom;
            }
            app.compose(&mut frame, fw, fh);
            let out = format!("{dir}/frame-{t:05}.png");
            std::fs::write(&out, yr_render::png::encode_rgba(fw, fh, &frame))
                .map_err(|e| e.to_string())?;
            let count = |o: u8| sim.units().iter().filter(|u| u.owner == o).count();
            println!(
                "{out}: tick {t} ${} player {} units / enemy {} units, {} enemy buildings, queue {}, {:?}",
                sim.credits(),
                count(yr_sim::OWNER_PLAYER),
                count(yr_sim::OWNER_ENEMY),
                sim.buildings().iter().filter(|b| b.owner == yr_sim::OWNER_ENEMY).count(),
                sim.production().len(),
                session.outcome,
            );
        }
    }
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if let [g, m, flag, dir, rest @ ..] = args.as_slice() {
        if flag == "--frames" {
            let ticks: Vec<u64> = rest.iter().filter_map(|t| t.parse().ok()).collect();
            let app = new_app(g.clone(), m.clone(), 2);
            if let Err(e) = headless_frames(app, dir, &ticks) {
                eprintln!("openyr: {e}");
                std::process::exit(1);
            }
            return;
        }
    }
    let (gamedir, map_name, scale) = match args.as_slice() {
        [g, m] => (g.clone(), m.clone(), 2),
        [g, m, s] => (g.clone(), m.clone(), s.parse().unwrap_or(2)),
        _ => {
            eprintln!(
                "usage: openyr <gamedir> <map> [scale] | <gamedir> <map> --frames <dir> <tick>..."
            );
            std::process::exit(2);
        }
    };
    let event_loop = EventLoop::new().expect("event loop");
    let mut app = new_app(gamedir, map_name, scale);
    event_loop.run_app(&mut app).expect("run app");
}
