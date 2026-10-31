//! OpenYR window: a live skirmish session over a rendered retail map.
//!
//! Usage: `openyr <gamedir> <map> [scale]`
//! Left-click / box-drag selects own units. With units selected, left-click
//! moves or attacks; right-click cancels. `A` then left-click = attack-move, `S` stop,
//! `H` harvest, right-drag / arrows pan, wheel or +/- zoom,
//! K toggles repair, L toggles sell, M toggles audio, F5 saves, F9 loads, Esc quits. Sidebar tabs select a category.
//! Left-click a cameo to queue it at the
//! war factory/barracks/construction yard, right-click cancels (refund). A finished
//! structure (green outline) is placed by clicking its cameo, then a map cell under a
//! green ghost; right-click or Esc backs out of placement (issue #46). Orders go through
//! `yr_sim::Command` only, the UI never mutates sim state directly (issue #15).

mod launcher;

use pixels::{Pixels, SurfaceTexture};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};
use yr_app::session::{Outcome, Session};
use yr_app::sidebar::{self, Category, ServiceMode, WIDTH as SIDEBAR_W};
use yr_sim::{Command, Fix};

const WIN_W: u32 = 1280;
const WIN_H: u32 = 800;
const TICK: Duration = Duration::from_millis(1000 / 15);

const PLAYER_COL: [u8; 4] = [64, 255, 128, 255];
const ENEMY_COL: [u8; 4] = [255, 64, 64, 255];
const NEUTRAL_COL: [u8; 4] = [180, 180, 180, 255];
const SELECT_COL: [u8; 4] = [255, 255, 255, 255];
/// Placement ghost tints (RGB + blend weight out of 255).
const GHOST_OK: [u8; 4] = [64, 255, 96, 110];
const GHOST_BAD: [u8; 4] = [255, 48, 48, 130];

struct View {
    cam_x: f32,
    cam_y: f32,
    zoom: f32,
    dirty: bool,
}

impl View {
    fn center_on_cell(
        &mut self,
        map: &yr_render::RenderedMap,
        cell: (f32, f32),
        viewport: (u32, u32),
    ) {
        let (x, y) = map.cell_to_px(cell.0, cell.1);
        self.cam_x = x - viewport.0 as f32 / (2.0 * self.zoom);
        self.cam_y = y - viewport.1 as f32 / (2.0 * self.zoom);
        self.dirty = true;
    }
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
    animations: yr_app::animation::UnitAnimations,
    view: View,
    selected: BTreeSet<u32>,
    pending: Vec<Command>,
    attack_move: bool,
    shift: bool,
    control: bool,
    cursor: (f64, f64),
    pointer: yr_app::cursor::Presence,
    left_down: Option<(f64, f64)>,
    right_down: Option<(f64, f64)>,
    right_panning: bool,
    radar_drag: bool,
    last_cursor: (f64, f64),
    keys: std::collections::HashSet<KeyCode>,
    last_tick: Instant,
    accum: Duration,
    title: String,
    /// The match result has been printed once.
    announced: bool,
    /// Sidebar slot of the finished structure being placed.
    placing: Option<usize>,
    sidebar_category: Category,
    service_mode: ServiceMode,
    load_path: Option<PathBuf>,
    save_path: PathBuf,
    save_requested: bool,
    startup_error: Option<String>,
}

impl App {
    fn load_session(&self) -> Result<Session, String> {
        match &self.load_path {
            Some(path) => yr_app::snapshot::load(&self.gamedir, &self.map_name, path),
            None => yr_app::session::load(&self.gamedir, &self.map_name, 42),
        }
    }

    fn quicksave(&self) {
        if let Some(session) = &self.session {
            if let Err(e) = yr_app::snapshot::save(session, &self.save_path) {
                eprintln!("OpenYR: save failed ({e})");
            }
        }
    }

    fn quickload(&mut self) {
        match yr_app::snapshot::load(&self.gamedir, &self.map_name, &self.save_path) {
            Ok(session) => {
                // Rebuild all presentation and discard orders from the replaced match.
                self.audio = None;
                match yr_app::audio::AudioScene::live(&self.gamedir, &self.map_name, &session) {
                    Ok(audio) => self.audio = Some(audio),
                    Err(e) => eprintln!("OpenYR: audio unavailable ({e}); continuing silently"),
                }
                self.animations = Default::default();
                self.animations.update(&session.sim);
                self.session = Some(session);
                self.selected.clear();
                self.pending.clear();
                self.left_down = None;
                self.right_down = None;
                self.right_panning = false;
                self.service_mode = ServiceMode::None;
                self.radar_drag = false;
                self.placing = None;
                self.attack_move = false;
                self.announced = false;
                self.accum = Duration::ZERO;
                self.last_tick = Instant::now();
                self.view.dirty = true;
            }
            Err(e) => eprintln!("OpenYR: load failed ({e}); current match retained"),
        }
    }

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
        Some(yr_app::interaction::cell_at(
            map,
            self.session.as_ref().and_then(|s| s.sim.terrain()),
            (mx, my),
        ))
    }

    fn cell_leptons(cell: (f32, f32)) -> (Fix, Fix) {
        let c = |v: f32| Fix::from_int(v.floor() as i32) + Fix::from_raw(128);
        (c(cell.0), c(cell.1))
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
                if additive && self.selected.contains(&u.id) {
                    self.selected.remove(&u.id);
                } else {
                    self.selected.insert(u.id);
                }
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
            let (mx, mut my) = map.cell_to_px(
                u.x.to_int_floor() as f32 + 0.5,
                u.y.to_int_floor() as f32 + 0.5,
            );
            let level = session
                .sim
                .terrain()
                .and_then(|g| g.cell(u.x.to_int_floor() as u16, u.y.to_int_floor() as u16))
                .map_or(0, |c| c.height);
            my -= f32::from(level) * 15.0 / map.scale.max(1) as f32;
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

    fn world_action(&self, pos: (f64, f64)) -> Option<yr_app::interaction::Action> {
        let session = self.session.as_ref()?;
        let cell = self.window_to_cell(pos)?;
        if !cell.0.is_finite() || !cell.1.is_finite() {
            return None;
        }
        Some(yr_app::interaction::at(
            session,
            &self.selected,
            (cell.0.floor() as i32, cell.1.floor() as i32),
            self.attack_move || (self.control && self.shift),
            self.service_mode,
        ))
    }

    fn left_click(&mut self, pos: (f64, f64)) {
        let Some(action) = self.world_action(pos) else {
            return;
        };
        if action.select.is_some() || (self.shift && !self.control) {
            self.select_at(pos, self.shift);
        } else if !self.selected.is_empty() {
            self.pending.extend(action.commands);
            self.attack_move = false;
        } else {
            self.select_at(pos, false);
        }
        self.view.dirty = true;
    }

    fn left_release(&mut self) {
        self.radar_drag = false;
        if let Some(a) = self.left_down.take() {
            if (self.cursor.0 - a.0).abs() > 6.0 || (self.cursor.1 - a.1).abs() > 6.0 {
                self.select_box(a, self.cursor, self.shift);
            } else if !self.in_sidebar(self.cursor) {
                self.left_click(self.cursor);
            }
        }
    }

    fn right_release(&mut self) {
        if self.right_down.take().is_some() && !self.right_panning {
            self.cancel_mode_or_selection();
        }
        self.right_panning = false;
    }

    fn cancel_pointer_action(&mut self) {
        self.left_down = None;
        self.right_down = None;
        self.right_panning = false;
        self.radar_drag = false;
    }

    fn cancel_mode_or_selection(&mut self) {
        if self.placing.take().is_none()
            && self.service_mode == ServiceMode::None
            && !self.attack_move
        {
            self.selected.clear();
        }
        self.service_mode = ServiceMode::None;
        self.attack_move = false;
        self.view.dirty = true;
    }

    fn focus_changed(&mut self, focused: bool) {
        self.pointer.focused = focused;
        if !focused {
            self.keys.clear();
            self.shift = false;
            self.control = false;
            self.cancel_pointer_action();
            self.placing = None;
            self.service_mode = ServiceMode::None;
            self.attack_move = false;
        }
        self.view.dirty = true;
    }

    fn cursor_kind(&self, width: u32) -> yr_app::cursor::Kind {
        use yr_app::cursor::Kind;
        if self.cursor.0 >= f64::from(width.saturating_sub(SIDEBAR_W)) {
            return Kind::Pointer;
        }
        if self.placing.is_some() {
            return if self.placement_target().is_some_and(|(_, _, valid)| valid) {
                Kind::Pointer
            } else {
                Kind::Blocked
            };
        }
        if self.left_down.is_some() || self.right_panning {
            return Kind::Pointer;
        }
        self.world_action(self.cursor)
            .map_or(Kind::Pointer, |a| a.cursor)
    }

    fn update_pointer_visibility(&self) {
        if let Some(window) = &self.window {
            let custom =
                self.pixels
                    .as_ref()
                    .zip(self.session.as_ref())
                    .is_some_and(|(pixels, session)| {
                        self.pointer.custom(
                            &session.cursor_art,
                            (pixels.texture().width(), pixels.texture().height()),
                            self.cursor,
                        )
                    });
            window.set_cursor_visible(!custom);
        }
    }

    fn radar_target(&self, pos: (f64, f64)) -> Option<(f32, f32)> {
        let width = self.pixels.as_ref()?.texture().width();
        self.session.as_ref()?.radar.target(width, pos)
    }

    fn radar_pan(&mut self, cell: (f32, f32)) {
        let (Some(map), Some(pixels)) = (&self.map, &self.pixels) else {
            return;
        };
        self.view.center_on_cell(
            map,
            cell,
            (
                pixels.texture().width() - SIDEBAR_W,
                pixels.texture().height(),
            ),
        );
    }

    fn radar_click(&mut self, cell: (f32, f32)) {
        self.service_mode = ServiceMode::None;
        self.placing = None;
        if self.selected.is_empty() {
            self.radar_drag = true;
            self.radar_pan(cell);
        } else {
            let (x, y) = Self::cell_leptons(cell);
            for &id in &self.selected {
                self.pending.push(if self.attack_move {
                    Command::AttackMove { id, x, y }
                } else {
                    Command::Move { id, x, y }
                });
            }
            self.attack_move = false;
        }
    }

    fn toggle_service(&mut self, mode: ServiceMode) {
        self.service_mode.toggle(mode);
        self.placing = None;
        self.attack_move = false;
        self.left_down = None;
        self.right_down = None;
        self.radar_drag = false;
        self.view.dirty = true;
    }

    fn service_click(&mut self) {
        let Some(cell) = self.window_to_cell(self.cursor) else {
            return;
        };
        if let Some(command) = self.session.as_ref().and_then(|session| {
            self.service_mode
                .command(session, (cell.0.floor() as i32, cell.1.floor() as i32))
        }) {
            self.pending.push(command);
        }
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
        self.service_mode = ServiceMode::None;
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
        sidebar::slot_at(
            fw,
            self.pixels.as_ref()?.texture().height(),
            session,
            self.sidebar_category,
            pos,
        )
    }

    fn in_sidebar(&self, pos: (f64, f64)) -> bool {
        let width = self.pixels.as_ref().map_or(WIN_W, |p| p.texture().width());
        pos.0 >= f64::from(width.saturating_sub(SIDEBAR_W))
    }

    fn tick_session(&mut self) {
        let Some(session) = &mut self.session else {
            return;
        };
        let cmds = std::mem::take(&mut self.pending);
        self.animations.update(&session.sim);
        if let Some(audio) = &mut self.audio {
            audio.commands(session, &cmds);
        }
        session.step(&cmds);
        self.animations.update(&session.sim);
        let events = session.sim.drain_events();
        if let Some(audio) = &mut self.audio {
            audio.update(session, &events);
        }
        self.selected
            .retain(|id| session.sim.units().iter().any(|u| u.id == *id));
        if self.placing.is_some() && session.ready_structure() != self.placing {
            self.placing = None;
        }
        let groups = session.sim.ai().map_or(0, |ai| ai.groups_sent());
        let enemy = format!("AI groups {groups}");
        let title = format!(
            "OpenYR: {} | ${} | tick {} | {enemy} | {} selected{} | {:?}",
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
            footprint: yr_render::depth::Footprint,
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
            animation: yr_render::sprites::InfantryAnimation,
            building: Option<yr_render::sprites::BuildingAnimation>,
        }
        let tick = self.session.as_ref().map_or(0, |s| s.sim.tick());
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
                    footprint: yr_render::depth::Footprint {
                        x: f32::from(b.cell.0),
                        y: f32::from(b.cell.1),
                        w: f32::from(b.size.0),
                        h: f32::from(b.size.1),
                    },
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
                    animation: Default::default(),
                    building: Some(yr_app::animation::building_animation(sim, b)),
                });
            }
            for u in sim.units() {
                if u.owner == yr_sim::OWNER_NEUTRAL && u.hp >= u.max_hp {
                    continue;
                }
                let (fx, fy) = (u.x.raw() as f32 / 256.0, u.y.raw() as f32 / 256.0);
                let (wx, wy) = to_win(fx, fy);
                markers.push(Marker {
                    footprint: yr_render::depth::Footprint {
                        x: fx,
                        y: fy,
                        w: 0.0,
                        h: 0.0,
                    },
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
                    animation: self.animations.get(u.id),
                    building: None,
                });
            }
        }
        let order = yr_render::depth::painter_order(
            &markers.iter().map(|m| m.footprint).collect::<Vec<_>>(),
        );
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
        // Bibs belong to the ground, below every unit and building body.
        for m in &markers {
            if let (Some((id, _, owner)), Some(animation), Some(bank)) =
                (&m.art, m.building, bank.as_mut())
            {
                let remap = owner_remap(bank, *owner);
                if let Some(bib) = bank.get_building(id, remap, animation).and_then(|b| b.bib) {
                    blit_sprite(frame, fw, fh, bib, m.ax, m.ay, k, SpritePass::Body);
                }
            }
        }
        // Union the shadow masks on the ground before drawing bodies.
        // Overlapping base/overlay shadows darken a pixel only once.
        let mut shadowed = vec![false; (fw * fh) as usize];
        for m in &markers {
            if let (Some((id, facing, owner)), Some(bank)) = (&m.art, bank.as_mut()) {
                let remap = owner_remap(bank, *owner);
                if let Some(animation) = m.building {
                    if let Some(building) = bank.get_building(id, remap, animation) {
                        for layer in building.bib.into_iter().chain(building.layers) {
                            blit_sprite(
                                frame,
                                fw,
                                fh,
                                layer,
                                m.ax,
                                m.ay,
                                k,
                                SpritePass::Shadow(&mut shadowed),
                            );
                        }
                    }
                } else if let Some(sprite) =
                    bank.get_animated_at(id, *facing, remap, m.animation, tick)
                {
                    blit_sprite(
                        frame,
                        fw,
                        fh,
                        sprite,
                        m.ax,
                        m.ay,
                        k,
                        SpritePass::Shadow(&mut shadowed),
                    );
                }
            }
        }
        for &index in &order {
            let m = &markers[index];
            let drawn = match (&m.art, bank.as_mut()) {
                (Some((id, facing, owner)), Some(bank)) => {
                    let remap = owner_remap(bank, *owner);
                    if let Some(animation) = m.building {
                        if let Some(building) = bank.get_building(id, remap, animation) {
                            for layer in building.layers {
                                blit_sprite(frame, fw, fh, layer, m.ax, m.ay, k, SpritePass::Body);
                            }
                            true
                        } else {
                            false
                        }
                    } else {
                        match bank.get_animated_at(id, *facing, remap, m.animation, tick) {
                            Some(s) => {
                                blit_sprite(frame, fw, fh, s, m.ax, m.ay, k, SpritePass::Body);
                                true
                            }
                            None => false,
                        }
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
            sidebar::draw(frame, fw, fh, session, self.sidebar_category, self.placing);
            sidebar::draw_controls(frame, fw, fh, session, self.service_mode);
            let corners = [
                map.px_to_cell(self.view.cam_x, self.view.cam_y),
                map.px_to_cell(
                    self.view.cam_x + (fw - SIDEBAR_W) as f32 / self.view.zoom,
                    self.view.cam_y + fh as f32 / self.view.zoom,
                ),
            ];
            session.radar.draw_viewport(frame, fw, fh, corners);
            sidebar::draw_tooltip(frame, fw, fh, session, self.sidebar_category, self.cursor);
        }
        // Box-select rubber band.
        if let (Some(a), false) = (self.left_down, self.right_panning) {
            if (self.cursor.0 - a.0).abs() > 6.0 || (self.cursor.1 - a.1).abs() > 6.0 {
                draw_rect_outline(frame, fw, fh, a, self.cursor, SELECT_COL);
            }
        }
        if let Some(session) = &self.session {
            if self
                .pointer
                .custom(&session.cursor_art, (fw, fh), self.cursor)
            {
                session
                    .cursor_art
                    .draw(frame, (fw, fh), self.cursor, self.cursor_kind(fw), tick);
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

enum SpritePass<'a> {
    Body,
    Shadow(&'a mut [bool]),
}

/// Nearest-neighbour blit with `k` window pixels per sprite pixel.
/// Shadows halve the ground once, using the shared coverage mask.
#[allow(clippy::too_many_arguments)]
fn blit_sprite(
    frame: &mut [u8],
    fw: u32,
    fh: u32,
    s: &yr_render::sprites::Sprite,
    ax: f32,
    ay: f32,
    k: f32,
    mut pass: SpritePass<'_>,
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
        let py = i64::from(y0) + i64::from(dy);
        let sy = ((dy as f32 + 0.5) / k) as u32;
        if py < 0 || py >= i64::from(fh) || sy >= s.h {
            continue;
        }
        for dx in 0..dw {
            let px = i64::from(x0) + i64::from(dx);
            let sx = ((dx as f32 + 0.5) / k) as u32;
            if px < 0 || px >= i64::from(fw) || sx >= s.w {
                continue;
            }
            let src = ((sy * s.w + sx) * 4) as usize;
            let dst = ((py as u32 * fw + px as u32) * 4) as usize;
            match (s.rgba[src + 3], &mut pass) {
                (yr_render::sprites::SHADOW_ALPHA, SpritePass::Shadow(mask)) => {
                    if !mask[dst / 4] {
                        for c in &mut frame[dst..dst + 3] {
                            *c /= 2;
                        }
                        mask[dst / 4] = true;
                    }
                }
                (255, SpritePass::Body) => {
                    frame[dst..dst + 4].copy_from_slice(&s.rgba[src..src + 4])
                }
                _ => {}
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
        let window = match el.create_window(attrs) {
            Ok(window) => std::sync::Arc::new(window),
            Err(e) => {
                self.startup_error = Some(format!("cannot create window: {e}"));
                el.exit();
                return;
            }
        };
        let size = window.inner_size();
        let surface = SurfaceTexture::new(size.width.max(1), size.height.max(1), window.clone());
        let pixels = match Pixels::new(WIN_W, WIN_H, surface) {
            Ok(pixels) => pixels,
            Err(e) => {
                self.startup_error = Some(format!("cannot create display surface: {e}"));
                el.exit();
                return;
            }
        };
        match yr_render::render_map(&self.gamedir, &self.map_name, self.scale) {
            Ok(map) => {
                println!("OpenYR: rendered {}", map.summary);
                self.view.cam_x = (map.width as f32 - WIN_W as f32) / 2.0;
                self.view.cam_y = (map.height as f32 - WIN_H as f32) / 2.0;
                self.map = Some(map);
                self.sprites = load_sprites(&self.gamedir, &self.map_name);
            }
            Err(e) => {
                self.startup_error = Some(format!("cannot render {}: {e}", self.map_name));
                el.exit();
                return;
            }
        }
        let initial = self
            .session
            .take()
            .map(Ok)
            .unwrap_or_else(|| self.load_session());
        match initial {
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
            Err(e) => {
                self.startup_error = Some(format!("session unavailable ({e})"));
                el.exit();
                return;
            }
        }
        println!("OpenYR: left-click select/order, box select, right-click cancel, A=attack-move, S=stop, H=harvest, K=repair, L=sell, M=mute, F5=save, F9=load, right-drag/arrows pan, wheel zoom; sidebar: left-click build or place a ready structure, right-click cancel");
        window.request_redraw();
        self.window = Some(window);
        self.pixels = Some(pixels);
        self.update_pointer_visibility();
        self.view.dirty = true;
        self.last_tick = Instant::now();
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                if let Some(window) = &self.window {
                    window.set_cursor_visible(true);
                }
                self.report_audio();
                el.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(p) = &mut self.pixels {
                    let _ = p.resize_surface(size.width.max(1), size.height.max(1));
                    self.view.dirty = true;
                }
            }
            WindowEvent::Focused(focused) => self.focus_changed(focused),
            WindowEvent::CursorEntered { .. } => {
                self.pointer.inside = true;
                self.view.dirty = true;
            }
            WindowEvent::CursorLeft { .. } => {
                self.pointer.inside = false;
                self.cancel_pointer_action();
                self.view.dirty = true;
            }
            WindowEvent::ModifiersChanged(m) => {
                self.shift = m.state().shift_key();
                self.control = m.state().control_key();
            }
            WindowEvent::MouseInput { state, button, .. } => match (button, state) {
                (MouseButton::Left, ElementState::Pressed) if self.in_sidebar(self.cursor) => {
                    let control =
                        self.pixels
                            .as_ref()
                            .zip(self.session.as_ref())
                            .and_then(|(p, s)| {
                                sidebar::control_at(p.texture().width(), s, self.cursor)
                            });
                    if let Some(mode) = control {
                        self.toggle_service(mode);
                        return;
                    }
                    if let Some(cell) = self.radar_target(self.cursor) {
                        self.radar_click(cell);
                        return;
                    }
                    let tab = self
                        .pixels
                        .as_ref()
                        .and_then(|p| sidebar::tab_at(p.texture().width(), self.cursor));
                    if let Some(category) = tab {
                        self.sidebar_category = category;
                        self.service_mode = ServiceMode::None;
                        self.placing = None;
                        self.view.dirty = true;
                    } else if let Some(slot) = self.sidebar_slot_at(self.cursor) {
                        self.sidebar_click(slot);
                    }
                }
                (MouseButton::Right, ElementState::Pressed) if self.in_sidebar(self.cursor) => {
                    self.service_mode = ServiceMode::None;
                    if self.radar_target(self.cursor).is_some() {
                        self.selected.clear();
                        self.attack_move = false;
                        self.placing = None;
                        self.view.dirty = true;
                    }
                    if let Some(slot) = self.sidebar_slot_at(self.cursor) {
                        if self.placing == Some(slot) {
                            self.placing = None;
                        }
                        if let Some(cmd) = self.session.as_ref().and_then(|s| s.cancel(slot)) {
                            self.pending.push(cmd);
                        }
                    }
                }
                (MouseButton::Left, ElementState::Pressed)
                    if self.service_mode != ServiceMode::None =>
                {
                    self.service_click();
                }
                (MouseButton::Right, ElementState::Pressed)
                    if self.service_mode != ServiceMode::None =>
                {
                    self.service_mode = ServiceMode::None;
                }
                (MouseButton::Left, ElementState::Pressed) if self.placing.is_some() => {
                    self.try_place();
                }
                (MouseButton::Right, ElementState::Pressed) if self.placing.is_some() => {
                    self.placing = None;
                }
                (MouseButton::Left, ElementState::Pressed) => self.left_down = Some(self.cursor),
                (MouseButton::Left, ElementState::Released) => self.left_release(),
                (MouseButton::Right, ElementState::Pressed) => {
                    self.right_down = Some(self.cursor);
                    self.right_panning = false;
                }
                (MouseButton::Right, ElementState::Released) => self.right_release(),
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
                if self.radar_drag {
                    if let Some(cell) = self.radar_target(self.cursor) {
                        self.radar_pan(cell);
                    }
                }
                self.view.dirty = true;
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
                                    if self.service_mode != ServiceMode::None {
                                        self.service_mode = ServiceMode::None;
                                    } else if self.placing.is_some() {
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
                                KeyCode::KeyA => {
                                    self.service_mode = ServiceMode::None;
                                    self.attack_move = true;
                                }
                                KeyCode::KeyK if !event.repeat => {
                                    self.toggle_service(ServiceMode::Repair)
                                }
                                KeyCode::KeyL if !event.repeat => {
                                    self.toggle_service(ServiceMode::Sell)
                                }
                                KeyCode::KeyM if !event.repeat => {
                                    if let Some(audio) = &self.audio {
                                        audio.toggle_mute();
                                    }
                                }
                                KeyCode::F5 if !event.repeat => self.quicksave(),
                                KeyCode::F9 if !event.repeat => self.quickload(),
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
        self.update_pointer_visibility();
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
        animations: Default::default(),
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
        control: false,
        cursor: (0.0, 0.0),
        pointer: yr_app::cursor::Presence {
            inside: false,
            focused: true,
        },
        left_down: None,
        right_down: None,
        right_panning: false,
        radar_drag: false,
        last_cursor: (0.0, 0.0),
        keys: std::collections::HashSet::new(),
        last_tick: Instant::now(),
        accum: Duration::ZERO,
        title: String::new(),
        announced: false,
        placing: None,
        sidebar_category: Category::default(),
        service_mode: ServiceMode::None,
        load_path: None,
        save_path: yr_app::snapshot::quicksave_path(),
        save_requested: false,
        startup_error: None,
    }
}

/// Structures the headless demo builds and places, in order.
const DEMO_STRUCTURES: [&str; 2] = ["GAPOWR", "GAREFN"];
/// Ticks the demo ghost spends on a blocked spot, then on a valid one,
/// before the structure goes down; the camera stays on it a while after.
const DEMO_GHOST_TICKS: u64 = 40;
const DEMO_FOCUS_TICKS: u64 = 200;

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

/// Script decisions use absolute sim ticks and current state so a saved
/// demo resumes with the same future commands as an uninterrupted demo.
fn headless_frames(
    mut app: App,
    dir: Option<&str>,
    ticks: &[u64],
    end: u64,
    passive: bool,
) -> Result<(), String> {
    if let Some(dir) = dir {
        app.map = Some(yr_render::render_map(
            &app.gamedir,
            &app.map_name,
            app.scale,
        )?);
        app.sprites = load_sprites(&app.gamedir, &app.map_name);
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    app.session = Some(app.load_session()?);
    app.pointer.inside = true;
    app.pointer.focused = true;
    if let Some(dir) = dir {
        app.audio = Some(yr_app::audio::AudioScene::capture(
            &app.gamedir,
            &app.map_name,
            app.session.as_ref().ok_or("no session")?,
            dir,
        )?);
    }
    let (fw, fh) = (WIN_W, WIN_H);
    let start = app.session.as_ref().ok_or("no session")?.sim.tick();
    if end < start || ticks.iter().any(|t| *t < start || *t > end) {
        return Err(format!("requested tick precedes loaded tick {start}"));
    }
    let mut frame = vec![0u8; (fw * fh * 4) as usize];
    // Map cell the camera holds on until a tick (placement close-ups).
    let mut focus: Option<((u16, u16), u64)> = None;
    for t in start..=end {
        let session = app.session.as_ref().ok_or("no session")?;
        let advancing = t > start;
        if advancing && t == 2 && !passive {
            for slot in [0, 0, 0, 4, 4, 4, 1, 5] {
                if let Some(cmd) = session.produce(slot) {
                    app.pending.push(cmd);
                }
            }
        }
        if advancing && !passive {
            let structure = match t {
                2 => Some(DEMO_STRUCTURES[0]),
                1200 => Some(DEMO_STRUCTURES[1]),
                _ => None,
            };
            if let Some((id, cmd)) = structure.and_then(|id| {
                let slot = session.buildables.iter().position(|b| b.id == id)?;
                Some((id, session.produce(slot)?))
            }) {
                println!("tick {t}: queued {id}");
                app.pending.push(cmd);
            }
        }
        app.placing = None;
        if !passive {
            if let Some(slot) = session.ready_structure() {
                let yard = session
                    .sim
                    .buildings()
                    .iter()
                    .find(|b| {
                        b.owner == yr_sim::OWNER_PLAYER && b.role == yr_sim::Role::ConstructionYard
                    })
                    .map(|b| b.cell);
                if let Some((bad, good)) =
                    yard.and_then(|y| Some((y, demo_spot(session, slot, y)?)))
                {
                    // Absolute phases need no saved presentation clock.
                    let phase = t % (3 * DEMO_GHOST_TICKS);
                    let target = if phase < DEMO_GHOST_TICKS { bad } else { good };
                    let (w, h) = session.buildables[slot].footprint().unwrap_or((1, 1));
                    let cursor = (target.0 + (w - 1) / 2, target.1 + (h - 1) / 2);
                    focus = Some((cursor, t + DEMO_FOCUS_TICKS));
                    app.placing = Some(slot);
                    if advancing && phase == 2 * DEMO_GHOST_TICKS {
                        if let Some(cmd) = session.place(slot, good) {
                            println!(
                                "tick {t}: placed {} at {good:?}",
                                session.buildables[slot].id
                            );
                            app.pending.push(cmd);
                        }
                        app.placing = None;
                    }
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
        if advancing && t % 450 == 0 && !passive {
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
        if t == start {
            app.selection_audio();
        }
        if advancing {
            app.tick_session();
        }
        if ticks.contains(&t) {
            let dir = dir.ok_or("frame ticks require --frames")?;
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
            // Exercise every shared tab at the same sim tick. The default
            // Structures view keeps the existing proof filename.
            let out = format!("{dir}/frame-{t:05}.png");
            for category in Category::ALL {
                app.sidebar_category = category;
                app.compose(&mut frame, fw, fh);
                let path = if category == Category::default() {
                    out.clone()
                } else {
                    format!("{dir}/frame-{t:05}-{}.png", category.file_name())
                };
                std::fs::write(&path, yr_render::png::encode_rgba(fw, fh, &frame))
                    .map_err(|e| e.to_string())?;
                println!("{path}: {} tab", category.label());
            }
            let count = |o: u8| sim.units().iter().filter(|u| u.owner == o).count();
            println!(
                "AI: ${}, queue {}, groups {}, scout {}, hash {:016x}",
                sim.credits_of(yr_sim::OWNER_ENEMY),
                sim.production_of(yr_sim::OWNER_ENEMY).len(),
                sim.ai().map_or(0, |ai| ai.groups_sent()),
                sim.ai().is_some_and(|ai| ai.scout_sent()),
                sim.state_hash()
            );
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
    if let Some(dir) = dir {
        println!(
            "{dir}/audio.wav: stereo 44100 Hz, {} ticks; cues in {dir}/audio-cues.tsv",
            end - start
        );
    }
    let session = app.session.as_ref().ok_or("no session")?;
    println!(
        "OpenYR: headless tick {}, hash {:016x}, {:?}",
        session.sim.tick(),
        session.sim.state_hash(),
        session.outcome
    );
    if app.save_requested {
        yr_app::snapshot::save(session, &app.save_path)?;
    }
    Ok(())
}

#[derive(Default)]
struct Options {
    gamedir: String,
    map: String,
    scale: u32,
    frames: Option<String>,
    ticks: Vec<u64>,
    end: Option<u64>,
    passive: bool,
    save: Option<PathBuf>,
    load: Option<PathBuf>,
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let [g, m, rest @ ..] = args else {
        return Err("a game directory and map are required".into());
    };
    let mut out = Options {
        gamedir: g.clone(),
        map: m.clone(),
        scale: 2,
        ..Default::default()
    };
    let mut i = 0;
    while i < rest.len() {
        let flag = &rest[i];
        i += 1;
        if flag == "--passive" {
            out.passive = true;
            continue;
        }
        if !flag.starts_with('-') {
            let n = flag.parse().map_err(|_| format!("invalid number {flag}"))?;
            if out.frames.is_some() {
                out.ticks.push(n);
            } else {
                out.scale = u32::try_from(n).map_err(|_| "scale out of range")?;
            }
            continue;
        }
        let value = rest
            .get(i)
            .filter(|v| !v.starts_with("--"))
            .ok_or_else(|| format!("{flag} needs a value"))?;
        i += 1;
        match flag.as_str() {
            "--frames" => out.frames = Some(value.clone()),
            "--ticks" => out.end = Some(value.parse().map_err(|_| "invalid --ticks value")?),
            "--save" => out.save = Some(value.into()),
            "--load" => out.load = Some(value.into()),
            _ => return Err(format!("unknown option {flag}")),
        }
    }
    if out.frames.is_some() && out.ticks.is_empty() {
        return Err("--frames needs at least one absolute tick".into());
    }
    if out.scale == 0 || out.scale > 8 {
        return Err("scale must be between 1 and 8".into());
    }
    if out
        .end
        .is_some_and(|end| out.ticks.iter().any(|t| *t > end))
    {
        return Err("--ticks must include every requested frame tick".into());
    }
    Ok(out)
}

fn run(args: &[String]) -> Result<(), String> {
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{}\n\nWithout arguments, OpenYR finds your retail install and starts EB1.MMX.\nSet OPENYR_GAMEDIR to override the remembered or standard install.", launcher::USAGE);
        return Ok(());
    }
    let Some(args) = launcher::arguments(args)? else {
        return Ok(());
    };
    let opts = parse_args(&args)?;
    let mut app = new_app(opts.gamedir, opts.map, opts.scale);
    app.load_path = opts.load;
    app.save_requested = opts.save.is_some();
    app.save_path = opts
        .save
        .or_else(|| app.load_path.clone())
        .unwrap_or_else(yr_app::snapshot::quicksave_path);
    let end = opts.end.or_else(|| opts.ticks.iter().copied().max());
    // --save alone writes the starting state headlessly. --load alone opens a window.
    if end.is_some() || app.save_requested || opts.frames.is_some() {
        let end = match end {
            Some(t) => t,
            None => app.load_session()?.sim.tick(),
        };
        return headless_frames(app, opts.frames.as_deref(), &opts.ticks, end, opts.passive);
    }
    if app.load_path.is_some() {
        app.session = Some(app.load_session()?);
    }
    let event_loop = EventLoop::new().map_err(|e| e.to_string())?;
    event_loop.run_app(&mut app).map_err(|e| e.to_string())?;
    match app.startup_error.take() {
        Some(e) => Err(e),
        None => Ok(()),
    }
}

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if let Err(e) = run(&args) {
        eprintln!("openyr: {e}\nusage: {}", launcher::USAGE);
        if args.is_empty() {
            launcher::show_error(&e);
        }
        std::process::exit(1);
    }
}

#[cfg(test)]
mod render_tests {
    use super::*;

    #[test]
    fn radar_navigation_centers_the_world_camera_at_every_zoom() {
        let map = yr_render::RenderedMap {
            width: 10,
            height: 10,
            rgba: Vec::new(),
            summary: String::new(),
            origin_x: -90,
            origin_y: -200,
            scale: 2,
        };
        for zoom in [0.25, 1.0, 4.0] {
            let mut view = View {
                cam_x: 0.0,
                cam_y: 0.0,
                zoom,
                dirty: false,
            };
            view.center_on_cell(&map, (21.5, 31.5), (1112, 800));
            let cell = map.px_to_cell(view.cam_x + 556.0 / zoom, view.cam_y + 400.0 / zoom);
            assert!((cell.0 - 21.5).abs() < 0.001 && (cell.1 - 31.5).abs() < 0.001);
            assert!(view.dirty);
        }
    }

    #[test]
    fn radar_orders_use_commands_and_preserve_selection() {
        let mut app = new_app(String::new(), String::new(), 2);
        app.selected.extend([7, 9]);
        app.placing = Some(3);
        app.radar_click((12.5, 20.5));
        assert_eq!(app.selected.len(), 2);
        assert!(app.placing.is_none());
        assert_eq!(app.pending.len(), 2);
        assert!(app.pending.iter().all(|cmd| matches!(cmd,
            Command::Move { x, y, .. } if x.raw() == 12*256+128 && y.raw() == 20*256+128)));
        app.pending.clear();
        app.attack_move = true;
        app.radar_click((18.5, 7.5));
        assert!(app
            .pending
            .iter()
            .all(|cmd| matches!(cmd, Command::AttackMove { .. })));
        assert!(!app.attack_move);
        app.pending.clear();
        app.selected.clear();
        app.radar_click((18.5, 7.5));
        assert!(app.radar_drag);
        assert!(
            app.pending.is_empty(),
            "unselected radar navigation sends no sim commands"
        );
    }

    #[test]
    fn save_load_options_mix_with_absolute_frame_ticks() {
        let parse = |args: &str| {
            parse_args(
                &args
                    .split_whitespace()
                    .map(str::to_owned)
                    .collect::<Vec<_>>(),
            )
        };
        let opts = parse("game EB1.MMX --load match.oyrsave --frames proof 900 1500 --save later.oyrsave --ticks 3000 --passive").unwrap();
        assert_eq!(opts.ticks, [900, 1500]);
        assert_eq!(opts.end, Some(3000));
        assert!(opts.passive);
        assert_eq!(opts.load, Some(PathBuf::from("match.oyrsave")));
        assert!(parse("game EB1.MMX --save").is_err());
        assert!(parse("game EB1.MMX --frames proof 900 bad").is_err());
        assert!(parse("game EB1.MMX --frames proof 900 --ticks 500").is_err());
    }

    #[test]
    fn shadow_union_darkens_ground_once_and_leaves_body_to_its_pass() {
        let sprite = yr_render::sprites::Sprite {
            w: 2,
            h: 1,
            ax: 0,
            ay: 0,
            rgba: vec![0, 0, 0, 128, 240, 180, 20, 255],
        };
        let mut frame = vec![100, 80, 60, 255, 100, 80, 60, 255];
        let mut mask = vec![false; 2];
        for _ in 0..3 {
            blit_sprite(
                &mut frame,
                2,
                1,
                &sprite,
                0.0,
                0.0,
                1.0,
                SpritePass::Shadow(&mut mask),
            );
        }
        assert_eq!(frame, [50, 40, 30, 255, 100, 80, 60, 255]);
        blit_sprite(&mut frame, 2, 1, &sprite, 0.0, 0.0, 1.0, SpritePass::Body);
        assert_eq!(frame, [50, 40, 30, 255, 240, 180, 20, 255]);
    }

    #[test]
    fn extreme_attachment_offsets_clip_without_coordinate_overflow() {
        let sprite = yr_render::sprites::Sprite {
            w: 2,
            h: 2,
            ax: i32::MIN,
            ay: i32::MIN,
            rgba: vec![255; 16],
        };
        let mut frame = vec![0; 16];
        blit_sprite(&mut frame, 2, 2, &sprite, 0.0, 0.0, 1.0, SpritePass::Body);
        assert_eq!(frame, [0; 16]);
    }
}

#[cfg(test)]
mod service_input_tests {
    use super::*;

    #[test]
    fn service_modes_cancel_placement_and_stale_pointer_orders() {
        let mut app = new_app(String::new(), String::new(), 2);
        app.placing = Some(3);
        app.attack_move = true;
        app.left_down = Some((12.0, 34.0));
        app.right_down = Some((56.0, 78.0));
        app.radar_drag = true;
        app.toggle_service(ServiceMode::Repair);
        assert_eq!(app.service_mode, ServiceMode::Repair);
        assert!(
            app.placing.is_none()
                && !app.attack_move
                && app.left_down.is_none()
                && app.right_down.is_none()
                && !app.radar_drag
        );
        app.toggle_service(ServiceMode::Sell);
        assert_eq!(app.service_mode, ServiceMode::Sell);
        app.toggle_service(ServiceMode::Sell);
        assert_eq!(app.service_mode, ServiceMode::None);
    }
}

impl Drop for App {
    fn drop(&mut self) {
        if let Some(window) = &self.window {
            window.set_cursor_visible(true);
        }
    }
}

#[cfg(test)]
mod input_tests;
