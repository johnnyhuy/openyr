//! OpenYR window: a live skirmish session over a rendered retail map.
//!
//! Usage: `openyr <gamedir> <map> [scale]`
//! Left-click / box-drag select own units, right-click move or attack
//! (enemy under cursor), `A` then right-click = attack-move, `S` stop,
//! `H` harvest, right-drag / arrows pan, wheel or +/- zoom,
//! M toggles audio, Esc quits. Right-hand sidebar: left-click a cameo queues it at the
//! war factory/barracks/construction yard, right-click cancels (refund). A finished
//! structure (green outline) is placed by clicking its cameo, then a map cell under a
//! green ghost; right-click or Esc backs out of placement (issue #46). Orders go through
//! `yr_sim::Command` only, the UI never mutates sim state directly (issue #15).

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
const READY_COL: [u8; 4] = [96, 255, 96, 255];
/// Placement ghost tints (RGB + blend weight out of 255).
const GHOST_OK: [u8; 4] = [64, 255, 96, 110];
const GHOST_BAD: [u8; 4] = [255, 48, 48, 130];

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
    /// Retail art for live units/buildings (decoded lazily while drawing).
    sprites: Option<std::cell::RefCell<yr_render::sprites::SpriteBank>>,
    session: Option<Session>,
    audio: Option<yr_app::audio::AudioScene>,
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
    /// Sidebar slot of the finished structure being placed.
    placing: Option<usize>,
}

impl App {
    fn report_audio(&self) {
        if let Some(audio) = &self.audio {
            println!(
                "OpenYR: audio callback delivered {} frames",
                audio.frames_played()
            );
        }
    }

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
        self.selection_audio();
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
        self.selection_audio();
        self.view.dirty = true;
    }

    fn selection_audio(&mut self) {
        if let (Some(audio), Some(session)) = (&mut self.audio, &self.session) {
            audio.select(session, self.selected.iter().copied());
        }
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

    /// Footprint origin and validity for the structure being placed,
    /// with the cursor over the map.
    fn placement_target(&self) -> Option<(usize, (u16, u16), bool)> {
        let slot = self.placing?;
        if self.in_sidebar(self.cursor) {
            return None;
        }
        let session = self.session.as_ref()?;
        let cell = self.window_to_cell(self.cursor)?;
        let origin =
            session.placement_origin(slot, (cell.0.floor() as i32, cell.1.floor() as i32))?;
        Some((slot, origin, session.can_place(slot, origin)))
    }

    /// Left-click on the map while placing: put the structure down when
    /// the ghost is green, otherwise keep placing.
    fn try_place(&mut self) {
        let Some((slot, origin, true)) = self.placement_target() else {
            return;
        };
        if let Some(cmd) = self.session.as_ref().and_then(|s| s.place(slot, origin)) {
            self.pending.push(cmd);
            self.placing = None;
        }
    }

    /// Left-click on a cameo: start placing a finished structure, else
    /// queue the slot.
    fn sidebar_click(&mut self, slot: usize) {
        let Some(session) = self.session.as_ref() else {
            return;
        };
        if session.ready_structure() == Some(slot) {
            self.placing = Some(slot);
        } else if let Some(cmd) = session.produce(slot) {
            self.pending.push(cmd);
        }
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
        if let Some(audio) = &mut self.audio {
            audio.commands(session, &cmds);
        }
        session.step(&cmds);
        let events = session.sim.drain_events();
        if let Some(audio) = &mut self.audio {
            audio.update(session, &events);
            if audio.device_failed() {
                eprintln!("OpenYR: audio device stopped; continuing silently");
                self.audio = None;
            }
        }
        self.selected
            .retain(|id| session.sim.units().iter().any(|u| u.id == *id));
        if self.placing.is_some() && session.ready_structure() != self.placing {
            self.placing = None;
        }
        let (launched, total) = session.waves_progress();
        let waves = match session.next_wave_in_secs() {
            Some(s) => format!("wave {}/{total} in {s}s", launched + 1),
            None => format!("waves {launched}/{total}"),
        };
        let title = format!(
            "OpenYR: {} | ${} | tick {} | {waves} | {} selected{} | {:?}",
            self.map_name,
            session.sim.credits(),
            session.sim.tick(),
            self.selected.len(),
            if self.placing.is_some() {
                " | PLACING"
            } else if self.attack_move {
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
            /// Painter-order key (cell x + y of the object's front).
            depth: f32,
            /// Window position of the sprite anchor.
            ax: f32,
            ay: f32,
            /// Window position of the ground centre (markers, bars).
            wx: f32,
            wy: f32,
            half_w: f32,
            col: [u8; 4],
            selected: bool,
            hp: f32,
            /// Rules id + sim facing + owner, when art may exist.
            art: Option<(String, u8, u8)>,
        }
        let mut markers: Vec<Marker> = Vec::new();
        // Placement ghost: window centres of the footprint cells.
        let mut ghost: Vec<(f32, f32)> = Vec::new();
        let mut ghost_ok = false;
        let cell_half = 30.0 / map.scale.max(1) as f32 * self.view.zoom;
        if let Some(session) = &self.session {
            let sim = &session.sim;
            let zoom = self.view.zoom;
            let lift = |x: i32, y: i32| {
                let level = sim
                    .terrain()
                    .and_then(|g| g.cell(x.max(0) as u16, y.max(0) as u16))
                    .map_or(0, |c| c.height);
                f32::from(level) * 15.0 / map.scale.max(1) as f32 * zoom
            };
            let to_win = |cx: f32, cy: f32| {
                let (mx, my) = map.cell_to_px(cx, cy);
                let lifted = lift(cx.floor() as i32, cy.floor() as i32);
                (
                    (mx - self.view.cam_x) * zoom,
                    (my - self.view.cam_y) * zoom - lifted,
                )
            };
            let kind_id = |k: u16| session.kinds.get(usize::from(k)).cloned();
            if let Some((slot, (ox, oy), ok)) = self.placement_target() {
                let (w, h) = session.buildables[slot].footprint().unwrap_or((1, 1));
                ghost_ok = ok;
                for dy in 0..h {
                    for dx in 0..w {
                        let (cx, cy) = (f32::from(ox + dx) + 0.5, f32::from(oy + dy) + 0.5);
                        ghost.push(to_win(cx, cy));
                    }
                }
            }
            for b in sim.buildings() {
                // Neutral map structures are already drawn as art; mark
                // them only once they take damage.
                if b.owner == yr_sim::OWNER_NEUTRAL && b.hp >= b.max_hp {
                    continue;
                }
                let (bx, by) = b.centre();
                let (wx, wy) = to_win(bx.raw() as f32 / 256.0, by.raw() as f32 / 256.0);
                let (ax, ay) = to_win(f32::from(b.cell.0) + 0.5, f32::from(b.cell.1) + 0.5);
                let half = ((f32::from(b.size.0 + b.size.1)) * 15.0 / map.scale.max(1) as f32
                    * zoom)
                    .max(8.0);
                markers.push(Marker {
                    depth: f32::from(b.cell.0 + b.size.0 + b.cell.1 + b.size.1) - 1.0,
                    ax,
                    ay,
                    wx,
                    wy,
                    half_w: half,
                    col: owner_col(b.owner),
                    selected: false,
                    hp: hp_frac(b.hp, b.max_hp),
                    art: (b.owner != yr_sim::OWNER_NEUTRAL)
                        .then(|| kind_id(b.kind).map(|k| (k, 0, b.owner)))
                        .flatten(),
                });
            }
            for u in sim.units() {
                if u.owner == yr_sim::OWNER_NEUTRAL && u.hp >= u.max_hp {
                    continue;
                }
                let (fx, fy) = (u.x.raw() as f32 / 256.0, u.y.raw() as f32 / 256.0);
                let (wx, wy) = to_win(fx, fy);
                markers.push(Marker {
                    depth: fx + fy,
                    ax: wx,
                    ay: wy,
                    wx,
                    wy,
                    half_w: (14.0 * zoom).max(5.0),
                    col: owner_col(u.owner),
                    selected: self.selected.contains(&u.id),
                    hp: hp_frac(u.hp, u.max_hp),
                    art: (u.owner != yr_sim::OWNER_NEUTRAL)
                        .then(|| kind_id(u.kind).map(|k| (k, u.facing, u.owner)))
                        .flatten(),
                });
            }
        }
        markers.sort_by(|a, b| a.depth.total_cmp(&b.depth));
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
        // Sprites in painter order; types without art fall back to a
        // filled owner-colour diamond.
        let k = self.view.zoom / map.scale.max(1) as f32;
        let mut bank = self.sprites.as_ref().map(|b| b.borrow_mut());
        for m in &markers {
            let drawn = match (&m.art, bank.as_mut()) {
                (Some((id, facing, owner)), Some(bank)) => {
                    let remap = owner_remap(bank, *owner);
                    match bank.get(id, *facing, remap) {
                        Some(s) => {
                            blit_sprite(frame, fw, fh, s, m.ax, m.ay, k);
                            true
                        }
                        None => false,
                    }
                }
                _ => false,
            };
            if !drawn {
                draw_diamond(frame, fw, fh, m.wx, m.wy, m.half_w, m.col);
            }
        }
        drop(bank);
        for m in &markers {
            if m.selected {
                draw_diamond_outline(frame, fw, fh, m.wx, m.wy, m.half_w + 4.0, SELECT_COL);
            }
            // As in game: health only on selected or damaged objects.
            if m.selected || m.hp < 1.0 {
                draw_health(
                    frame,
                    fw,
                    fh,
                    m.wx,
                    m.wy - m.half_w * 0.5 - 8.0,
                    m.half_w * 1.2,
                    m.hp,
                );
            }
        }
        let tint = if ghost_ok { GHOST_OK } else { GHOST_BAD };
        for &(gx, gy) in &ghost {
            blend_diamond(frame, fw, fh, gx, gy, cell_half - 1.0, tint);
            draw_diamond_outline(
                frame,
                fw,
                fh,
                gx,
                gy,
                cell_half,
                [tint[0], tint[1], tint[2], 255],
            );
        }
        if let Some(session) = &self.session {
            draw_sidebar(frame, fw, fh, session, self.placing);
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
/// queue count and a build-progress wipe; slots whose factory is gone or
/// whose prerequisites are missing are dimmed. A finished structure gets
/// a green outline, the one being placed a white one.
fn draw_sidebar(frame: &mut [u8], fw: u32, fh: u32, session: &Session, placing: Option<usize>) {
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
            .any(|h| h.owner == yr_sim::OWNER_PLAYER && h.role == b.factory.role())
            && session.prerequisites_met(i);
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
        let outline = if placing == Some(i) {
            Some(SELECT_COL)
        } else if session.ready_structure() == Some(i) {
            Some(READY_COL)
        } else {
            None
        };
        if let Some(col) = outline {
            for inset in 0..2 {
                let a = (f64::from(sx + inset), f64::from(sy + inset));
                let b = (
                    f64::from(sx + CAMEO_W - 1 - inset),
                    f64::from(sy + CAMEO_H - 1 - inset),
                );
                draw_rect_outline(frame, fw, fh, a, b, col);
            }
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

/// House colour for sim owners: the player plays the Allies' first
/// country, the scripted enemy the Soviets' (`[Colors]` via rules).
fn owner_remap(bank: &yr_render::sprites::SpriteBank, owner: u8) -> [u8; 3] {
    match owner {
        yr_sim::OWNER_PLAYER => bank.house_color("Americans"),
        yr_sim::OWNER_ENEMY => bank.house_color("Russians"),
        _ => bank.house_color(""),
    }
}

/// Nearest-neighbour blit of `s` with its anchor at window `(ax, ay)`,
/// `k` window pixels per sprite pixel. Shadow pixels halve the frame.
fn blit_sprite(
    frame: &mut [u8],
    fw: u32,
    fh: u32,
    s: &yr_render::sprites::Sprite,
    ax: f32,
    ay: f32,
    k: f32,
) {
    if k <= 0.0 {
        return;
    }
    let x0 = (ax - s.ax as f32 * k).round() as i32;
    let y0 = (ay - s.ay as f32 * k).round() as i32;
    let (dw, dh) = (
        (s.w as f32 * k).ceil() as i32,
        (s.h as f32 * k).ceil() as i32,
    );
    for dy in 0..dh {
        let py = y0 + dy;
        let sy = ((dy as f32 + 0.5) / k) as u32;
        if py < 0 || py >= fh as i32 || sy >= s.h {
            continue;
        }
        for dx in 0..dw {
            let px = x0 + dx;
            let sx = ((dx as f32 + 0.5) / k) as u32;
            if px < 0 || px >= fw as i32 || sx >= s.w {
                continue;
            }
            let src = ((sy * s.w + sx) * 4) as usize;
            let dst = ((py as u32 * fw + px as u32) * 4) as usize;
            match s.rgba[src + 3] {
                0 => {}
                yr_render::sprites::SHADOW_ALPHA => {
                    for c in &mut frame[dst..dst + 3] {
                        *c /= 2;
                    }
                }
                _ => frame[dst..dst + 4].copy_from_slice(&s.rgba[src..src + 4]),
            }
        }
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

/// Fill a cell diamond, blending `col` (alpha = weight) over the frame.
fn blend_diamond(frame: &mut [u8], fw: u32, fh: u32, cx: f32, cy: f32, half_w: f32, col: [u8; 4]) {
    let half_h = (half_w * 0.5).max(1.0);
    let a = u32::from(col[3]);
    for dy in -(half_h as i32)..=(half_h as i32) {
        let w = (half_w - (dy as f32).abs() * half_w / half_h).max(0.0);
        for dx in -(w as i32)..=(w as i32) {
            let (x, y) = (cx as i32 + dx, cy as i32 + dy);
            if x < 0 || y < 0 || x as u32 >= fw || y as u32 >= fh {
                continue;
            }
            let at = ((y as u32 * fw + x as u32) * 4) as usize;
            for c in 0..3 {
                let v = u32::from(frame[at + c]) * (255 - a) + u32::from(col[c]) * a;
                frame[at + c] = (v / 255) as u8;
            }
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
            .with_title(format!("OpenYR: {}", self.map_name))
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
                self.sprites = load_sprites(&self.gamedir, &self.map_name);
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
                    "OpenYR: session live, {} units, {} buildings, ${}",
                    s.sim.units().len(),
                    s.sim.buildings().len(),
                    s.sim.credits()
                );
                match yr_app::audio::AudioScene::live(&self.gamedir, &self.map_name, &s) {
                    Ok(audio) => self.audio = Some(audio),
                    Err(e) => eprintln!("OpenYR: audio unavailable ({e}); continuing silently"),
                }
                self.session = Some(s);
            }
            Err(e) => eprintln!("OpenYR: session unavailable ({e}); view-only"),
        }
        println!("OpenYR: left-click/box select, right-click order, A=attack-move, S=stop, H=harvest, M=mute, right-drag/arrows pan, wheel zoom; sidebar: left-click build or place a ready structure, right-click cancel");
        window.request_redraw();
        self.window = Some(window);
        self.pixels = Some(pixels);
        self.view.dirty = true;
        self.last_tick = Instant::now();
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                self.report_audio();
                el.exit();
            }
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
                    if let Some(slot) = self.sidebar_slot_at(self.cursor) {
                        self.sidebar_click(slot);
                    }
                }
                (MouseButton::Right, ElementState::Pressed) if self.in_sidebar(self.cursor) => {
                    if let Some(slot) = self.sidebar_slot_at(self.cursor) {
                        if self.placing == Some(slot) {
                            self.placing = None;
                        }
                        if let Some(cmd) = self.session.as_ref().and_then(|s| s.cancel(slot)) {
                            self.pending.push(cmd);
                        }
                    }
                }
                (MouseButton::Left, ElementState::Pressed) if self.placing.is_some() => {
                    self.try_place();
                }
                (MouseButton::Right, ElementState::Pressed) if self.placing.is_some() => {
                    self.placing = None;
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
                                    if self.placing.is_some() {
                                        self.placing = None;
                                    } else if self.attack_move {
                                        self.attack_move = false;
                                    } else {
                                        self.report_audio();
                                        el.exit();
                                    }
                                }
                                KeyCode::Equal => self.view.zoom = (self.view.zoom * 1.2).min(4.0),
                                KeyCode::Minus => self.view.zoom = (self.view.zoom / 1.2).max(0.25),
                                KeyCode::KeyA => self.attack_move = true,
                                KeyCode::KeyM if !event.repeat => {
                                    if let Some(audio) = &self.audio {
                                        audio.toggle_mute();
                                    }
                                }
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

fn load_sprites(
    gamedir: &str,
    map_name: &str,
) -> Option<std::cell::RefCell<yr_render::sprites::SpriteBank>> {
    match yr_render::sprites::SpriteBank::open(gamedir, map_name) {
        Ok(b) => Some(std::cell::RefCell::new(b)),
        Err(e) => {
            eprintln!("OpenYR: no object art ({e}); drawing markers");
            None
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
        sprites: None,
        session: None,
        audio: None,
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
        placing: None,
    }
}

/// Structures the headless demo builds and places, in order.
const DEMO_STRUCTURES: [&str; 2] = ["GAPOWR", "GAREFN"];
/// Ticks the demo ghost spends on a blocked spot, then on a valid one,
/// before the structure goes down; the camera stays on it a while after.
const DEMO_GHOST_TICKS: u64 = 40;
const DEMO_FOCUS_TICKS: u64 = 200;

/// A finished structure the headless demo is showing as a ghost.
#[derive(Clone, Copy)]
struct DemoGhost {
    slot: usize,
    ready: u64,
    /// Over the construction yard (red), then the valid spot (green).
    blocked: (u16, u16),
    valid: (u16, u16),
}

/// First valid footprint origin for `slot` in rings around `near`.
fn demo_spot(session: &Session, slot: usize, near: (u16, u16)) -> Option<(u16, u16)> {
    for r in 0..16i32 {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs() != r && dy.abs() != r {
                    continue;
                }
                let x = u16::try_from(i32::from(near.0) + dx).ok()?;
                let y = u16::try_from(i32::from(near.1) + dy).ok()?;
                if session.can_place(slot, (x, y)) {
                    return Some((x, y));
                }
            }
        }
    }
    None
}

/// `--frames <dir> <tick>...`: play a scripted demo headlessly and dump
/// the exact window frame at each tick. Script: queue Grizzlies and GIs
/// at tick 1, then every 450 ticks send the whole player army (armed
/// units) to attack-move on the enemy base; the camera follows the army.
/// Alongside, the construction yard builds `DEMO_STRUCTURES` one after
/// another: each finished one is shown as a red ghost over the yard, then
/// a green ghost on the nearest valid spot, then placed, with the camera
/// on it.
fn headless_frames(mut app: App, dir: &str, ticks: &[u64]) -> Result<(), String> {
    app.map = Some(yr_render::render_map(
        &app.gamedir,
        &app.map_name,
        app.scale,
    )?);
    app.sprites = load_sprites(&app.gamedir, &app.map_name);
    app.session = Some(yr_app::session::load(&app.gamedir, &app.map_name, 42)?);
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    app.audio = Some(yr_app::audio::AudioScene::capture(
        &app.gamedir,
        &app.map_name,
        app.session.as_ref().ok_or("no session")?,
        dir,
    )?);
    let (fw, fh) = (WIN_W, WIN_H);
    let last = ticks.iter().copied().max().unwrap_or(0);
    let mut frame = vec![0u8; (fw * fh * 4) as usize];
    let mut next_structure = 0usize;
    let mut queue_at = Some(1u64);
    let mut demo: Option<DemoGhost> = None;
    // Map cell the camera holds on until a tick (placement close-ups).
    let mut focus: Option<((u16, u16), u64)> = None;
    for t in 1..=last {
        let session = app.session.as_ref().ok_or("no session")?;
        if t == 1 {
            for slot in [0, 0, 0, 4, 4, 4, 1, 5] {
                if let Some(cmd) = session.produce(slot) {
                    app.pending.push(cmd);
                }
            }
        }
        if queue_at == Some(t) {
            queue_at = None;
            let slot = DEMO_STRUCTURES
                .get(next_structure)
                .and_then(|id| session.buildables.iter().position(|b| b.id == *id));
            if let Some(cmd) = slot.and_then(|s| session.produce(s)) {
                println!("tick {t}: queued {}", DEMO_STRUCTURES[next_structure]);
                app.pending.push(cmd);
            }
        }
        let yard = session
            .sim
            .buildings()
            .iter()
            .find(|b| b.owner == yr_sim::OWNER_PLAYER && b.role == yr_sim::Role::ConstructionYard)
            .map(|b| b.cell);
        if let (None, Some(slot), Some(yard)) = (demo, session.ready_structure(), yard) {
            if let Some(good) = demo_spot(session, slot, yard) {
                println!(
                    "tick {t}: {} ready; blocked ghost at {yard:?}, valid spot {good:?}",
                    session.buildables[slot].id
                );
                demo = Some(DemoGhost {
                    slot,
                    ready: t,
                    blocked: yard,
                    valid: good,
                });
                app.placing = Some(slot);
            }
        }
        if let Some(DemoGhost {
            slot,
            ready,
            blocked: bad,
            valid: good,
        }) = demo
        {
            let (w, h) = session.buildables[slot].footprint().unwrap_or((1, 1));
            let target = if t - ready < DEMO_GHOST_TICKS {
                bad
            } else {
                good
            };
            // Cursor cell that puts the footprint's top-left on `target`.
            let cursor = (target.0 + (w - 1) / 2, target.1 + (h - 1) / 2);
            focus = Some((cursor, t + DEMO_FOCUS_TICKS));
            if t - ready == 2 * DEMO_GHOST_TICKS {
                debug_assert!(!session.can_place(slot, bad) && session.can_place(slot, good));
                if let Some(cmd) = session.place(slot, good) {
                    println!(
                        "tick {t}: placed {} at {good:?}",
                        session.buildables[slot].id
                    );
                    app.pending.push(cmd);
                }
                app.placing = None;
                demo = None;
                next_structure += 1;
                queue_at = Some(t + 1);
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
        if t == 1 {
            app.selection_audio();
        }
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
            let held = focus.filter(|(_, until)| t <= *until).map(|(c, _)| c);
            let centre = match held {
                Some(c) => Some((f32::from(c.0), f32::from(c.1))),
                None if !mine.is_empty() => {
                    let n = mine.len() as f32;
                    let cx = mine.iter().map(|u| u.x.to_int_floor() as f32).sum::<f32>() / n;
                    let cy = mine.iter().map(|u| u.y.to_int_floor() as f32).sum::<f32>() / n;
                    Some((cx, cy))
                }
                None => None,
            };
            if let Some((cx, cy)) = centre {
                let (mx, my) = map.cell_to_px(cx + 0.5, cy + 0.5);
                let (vw, vh) = ((fw - SIDEBAR_W) as f32 / 2.0, fh as f32 / 2.0);
                app.view.cam_x = mx - vw / app.view.zoom;
                app.view.cam_y = my - vh / app.view.zoom;
                // The ghost follows the cursor: park it mid-view.
                app.cursor = (f64::from(vw), f64::from(vh));
            }
            app.compose(&mut frame, fw, fh);
            let out = format!("{dir}/frame-{t:05}.png");
            std::fs::write(&out, yr_render::png::encode_rgba(fw, fh, &frame))
                .map_err(|e| e.to_string())?;
            let count = |o: u8| sim.units().iter().filter(|u| u.owner == o).count();
            let ghost = match app.placement_target() {
                Some((_, origin, true)) => format!(", green ghost at {origin:?}"),
                Some((_, origin, false)) => format!(", red ghost at {origin:?}"),
                None => String::new(),
            };
            println!(
                "{out}: tick {t} ${} player {} units / {} buildings / enemy {} units, {} enemy buildings, power {:?}, queue {}{ghost}, {:?}",
                sim.credits(),
                count(yr_sim::OWNER_PLAYER),
                sim.buildings().iter().filter(|b| b.owner == yr_sim::OWNER_PLAYER).count(),
                count(yr_sim::OWNER_ENEMY),
                sim.buildings().iter().filter(|b| b.owner == yr_sim::OWNER_ENEMY).count(),
                sim.power(),
                sim.production().len(),
                session.outcome,
            );
        }
    }
    if let Some(audio) = app.audio.take() {
        audio.finish()?;
    }
    println!("{dir}/audio.wav: stereo 44100 Hz, {last} ticks; cues in {dir}/audio-cues.tsv");
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
