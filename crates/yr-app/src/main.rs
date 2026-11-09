//! OpenYR window: a live skirmish session over a rendered retail map.
//!
//! Usage: `openyr <gamedir> <map> [scale]`
//! Left-click / box-drag selects own units. With units selected, left-click
//! moves or attacks; right-click cancels. `A` then left-click = attack-move, `S` stop,
//! `H` centers the base, Ctrl+H harvests, Ctrl+1-9 assigns teams, 1-9 recalls, two presses center,
//! Shift+number adds a team; T expands visible matching types, two T presses select them across the map; right-drag / arrows pan, wheel or +/- zoom,
//! K toggles repair, L toggles sell, M toggles audio, F5 saves, F9 loads, Esc pauses. Sidebar tabs select a category.
//! Left-click a cameo to queue it at the
//! war factory/barracks/construction yard, right-click holds, then cancels (refund). A finished
//! structure (green outline) is placed by clicking its cameo, then a map cell under a
//! green ghost; right-click or Esc backs out of placement (issue #46). Orders go through
//! `yr_sim::Command` only, the UI never mutates sim state directly (issue #15).

mod base_navigation;
mod camera;
#[cfg(test)]
mod combat_depth_tests;
mod control_groups;
#[cfg(test)]
mod death_tests;
mod front_flow;
mod hover_feedback;
mod keyboard_input;
mod launcher;
mod menu_flow;
#[cfg(test)]
mod missile_tests;
mod order_feedback;
#[cfg(test)]
mod production_tests;
#[cfg(test)]
mod scrolling_tests;
mod selection_box;
#[cfg(test)]
mod selection_box_tests;
mod surface_input;
mod type_selection;
mod unit_status;
mod window_size;

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
    fn center_map(&mut self, map: &yr_render::RenderedMap, viewport: (u32, u32)) {
        self.cam_x = (map.width as f32 - viewport.0 as f32 / self.zoom) / 2.0;
        self.cam_y = (map.height as f32 - viewport.1 as f32 / self.zoom) / 2.0;
        self.dirty = true;
    }

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

    fn center_on_ground(
        &mut self,
        map: &yr_render::RenderedMap,
        terrain: Option<&yr_sim::TerrainGrid>,
        cell: (f32, f32),
        viewport: (u32, u32),
    ) {
        self.center_on_cell(map, cell, viewport);
        let (_, flat_y) = map.cell_to_px(cell.0, cell.1);
        let (_, ground_y) = yr_app::interaction::ground_point(map, terrain, cell);
        self.cam_y += ground_y - flat_y;
    }
}

struct App {
    frame_size: (u32, u32),
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
    effects: yr_app::effects::EffectScene,
    view: View,
    camera: camera::Input,
    selected: BTreeSet<u32>,
    groups: control_groups::ControlGroups,
    /// Last deliberate T press: time, represented types and resulting selection.
    type_press: Option<(Instant, BTreeSet<u16>, BTreeSet<u32>)>,
    order_feedback: order_feedback::Feedback,
    pending: Vec<Command>,
    attack_move: bool,
    shift: bool,
    control: bool,
    super_key: bool,
    cursor: (f64, f64),
    pointer: yr_app::cursor::Presence,
    left_down: Option<(f64, f64)>,
    right_down: Option<(f64, f64)>,
    right_panning: bool,
    radar_drag: bool,
    last_cursor: (f64, f64),
    keys: std::collections::HashSet<KeyCode>,
    suppressed_keys: std::collections::HashSet<KeyCode>,
    held_keys: std::collections::HashSet<KeyCode>,
    last_tick: Instant,
    accum: Duration,
    title: String,
    /// The match result has been printed once.
    announced: bool,
    /// Sidebar slot of the finished structure being placed.
    placing: Option<usize>,
    sidebar_view: sidebar::View,
    service_mode: ServiceMode,
    load_path: Option<PathBuf>,
    save_path: PathBuf,
    save_requested: bool,
    startup_error: Option<String>,
    menu: yr_app::menu::State,
    menu_art: yr_app::menu::Art,
    load_job: Option<menu_flow::LoadJob>,
    loading_art: yr_app::loading::Art,
    loading_preview: Option<Box<yr_app::frontend::Preview>>,
    preferences_path: Option<PathBuf>,
    quit_requested: bool,
    startup_menu: bool,
    front_art: yr_app::frontend::Art,
    front_preview: Option<yr_app::frontend::Preview>,
    chooser_preview: Option<yr_app::frontend::Preview>,
    chooser_job: Option<front_flow::chooser::Job>,
    chooser_epoch: u64,
    movie: Option<yr_app::movie::Movie>,
    front_epoch: Instant,
    front_phase: u64,
}

impl App {
    fn load_session(&self) -> Result<Session, String> {
        match &self.load_path {
            Some(path) => yr_app::snapshot::load(&self.gamedir, &self.map_name, path),
            None => yr_app::session::load(&self.gamedir, &self.map_name, 42),
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
        self.groups.interrupt();
        self.interrupt_type_press();
        self.order_feedback.clear();
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
        self.groups.interrupt();
        self.interrupt_type_press();
        self.order_feedback.clear();
        if !additive {
            self.selected.clear();
        }
        let Some(bounds) = selection_box::Bounds::new(a, b, self.camera_size()) else {
            return;
        };
        let (Some(map), Some(session)) = (&self.map, &self.session) else {
            return;
        };
        self.selected
            .extend(bounds.ids(map, &session.sim, &self.view));
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
            self.shift && !self.control,
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
            self.acknowledge_order(&action, pos, Instant::now());
            self.pending.extend(action.commands);
            self.attack_move = false;
        } else {
            self.select_at(pos, false);
        }
        self.view.dirty = true;
    }

    fn left_press(&mut self) {
        // Edge arrows own the gesture, including blocked map boundaries.
        if self.edge_cursor(self.camera_size()).is_some() {
            return;
        }
        if self.service_mode != ServiceMode::None {
            self.service_click();
        } else if self.placing.is_some() {
            self.try_place();
        } else {
            self.left_down = Some(self.cursor);
        }
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
        self.sidebar_view.clear_wheel();
        self.menu.wheel_remainder = 0.0;
        self.camera.edge_armed = false;
        self.menu.maps.drag = None;
        self.menu.setup.drag = None;
        self.menu.setup.popup = None;
        self.menu.pressed = None;
        self.left_down = None;
        self.right_down = None;
        self.right_panning = false;
        self.radar_drag = false;
    }

    fn cancel_mode_or_selection(&mut self) {
        self.groups.interrupt();
        self.interrupt_type_press();
        self.order_feedback.clear();
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
            self.menu.name_selected = false;
            self.menu.setup.name_selected = false;
            self.menu.refresh_heading();
            self.groups.interrupt();
            self.interrupt_type_press();
            self.order_feedback.clear();
            self.invalidate_pointer();
            self.suppress_held_keys();
            self.held_keys.clear();
            self.shift = false;
            self.control = false;
            self.super_key = false;
            self.placing = None;
            self.service_mode = ServiceMode::None;
            self.attack_move = false;
        }
        self.view.dirty = true;
    }

    /// Feedback follows the native gesture owner, including before a redraw.
    fn world_pointer_busy(&self) -> bool {
        self.keyboard_pan() != (0.0, 0.0)
            || self.left_down.is_some()
            || self.right_down.is_some()
            || self.right_panning
            || self.radar_drag
    }

    fn cursor_kind(&self, size: (u32, u32)) -> yr_app::cursor::Kind {
        use yr_app::cursor::Kind;
        if let Some(kind) = self.edge_cursor(size) {
            return kind;
        }
        if self.cursor.0 >= f64::from(size.0.saturating_sub(SIDEBAR_W)) {
            return Kind::Pointer;
        }
        if self.world_pointer_busy() {
            return Kind::Pointer;
        }
        if self.placing.is_some() {
            return if self.placement_target().is_some_and(|(_, _, valid)| valid) {
                Kind::Pointer
            } else {
                Kind::Blocked
            };
        }
        self.world_action(self.cursor)
            .map_or(Kind::Pointer, |a| a.cursor)
    }

    fn update_pointer_visibility(&self) {
        if let Some(window) = &self.window {
            let art = if self.menu.front {
                Some(&self.front_art.cursor)
            } else {
                self.session.as_ref().map(|s| &s.cursor_art)
            };
            let custom = self.pixels.as_ref().zip(art).is_some_and(|(pixels, art)| {
                self.pointer.custom(
                    art,
                    (pixels.texture().width(), pixels.texture().height()),
                    self.cursor,
                )
            });
            window.set_cursor_visible(!custom);
        }
    }

    fn radar_target(&self, pos: (f64, f64)) -> Option<(f32, f32)> {
        self.session
            .as_ref()?
            .radar
            .target(self.camera_size().0, pos)
    }

    fn radar_pan(&mut self, cell: (f32, f32)) {
        let size = self.camera_size();
        self.center_world_at(cell, (size.0.saturating_sub(SIDEBAR_W), size.1));
    }

    fn center_world_at(&mut self, cell: (f32, f32), viewport: (u32, u32)) {
        let Some(map) = &self.map else {
            return;
        };
        self.view.center_on_ground(
            map,
            self.session.as_ref().and_then(|s| s.sim.terrain()),
            cell,
            viewport,
        );
    }

    fn radar_click(&mut self, cell: (f32, f32)) {
        self.order_feedback.clear();
        self.service_mode = ServiceMode::None;
        self.placing = None;
        if self.selected.is_empty() {
            self.radar_drag = true;
            self.radar_pan(cell);
        } else {
            let (x, y) = Self::cell_leptons(cell);
            let commands: Vec<_> = self
                .selected
                .iter()
                .map(|&id| {
                    if self.attack_move {
                        Command::AttackMove { id, x, y }
                    } else {
                        Command::Move { id, x, y }
                    }
                })
                .collect();
            self.acknowledge_radar_order(&commands, cell, self.attack_move, Instant::now());
            self.pending.extend(commands);
            self.attack_move = false;
        }
        self.view.dirty = true;
    }

    fn toggle_service(&mut self, mode: ServiceMode) {
        self.order_feedback.clear();
        self.cancel_pointer_action();
        self.service_mode.toggle(mode);
        self.placing = None;
        self.attack_move = false;
        self.view.dirty = true;
    }

    fn service_click(&mut self) {
        let Some(cell) = self.window_to_cell(self.cursor) else {
            return;
        };
        let command = self.session.as_ref().and_then(|session| {
            self.service_mode
                .command(session, (cell.0.floor() as i32, cell.1.floor() as i32))
        });
        self.acknowledge_service_order(command.as_ref(), cell, Instant::now());
        self.pending.extend(command);
        self.view.dirty = true;
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
        if self.placing.is_none() {
            return;
        }
        let Some(cell) = self.window_to_cell(self.cursor) else {
            return;
        };
        let command = self.placement_target().and_then(|(slot, origin, valid)| {
            valid
                .then(|| self.session.as_ref()?.place(slot, origin))
                .flatten()
        });
        self.acknowledge_placement_order(command.as_ref(), cell, Instant::now());
        if command.is_some() {
            self.placing = None;
        }
        self.pending.extend(command);
        self.view.dirty = true;
    }

    /// Left-click on a cameo: start placing a finished structure, else
    /// resume held work or queue the slot.
    fn sidebar_click(&mut self, slot: usize) {
        self.service_mode = ServiceMode::None;
        let Some(session) = self.session.as_ref() else {
            self.order_feedback.clear();
            return;
        };
        if session.ready_structure() == Some(slot) {
            self.attack_move = false;
            self.placing = Some(slot);
            self.acknowledge_placement_start(Instant::now());
        } else {
            let cmd = session.production_click(slot, false, &self.pending);
            self.acknowledge_production_order(cmd.as_ref(), false, Instant::now());
            self.pending.extend(cmd);
        }
        self.view.dirty = true;
    }

    fn sidebar_right_click(&mut self, slot: usize) {
        self.service_mode = ServiceMode::None;
        if self.placing == Some(slot) {
            self.placing = None;
        }
        let cmd = self
            .session
            .as_ref()
            .and_then(|s| s.production_click(slot, true, &self.pending));
        self.acknowledge_production_order(cmd.as_ref(), true, Instant::now());
        self.pending.extend(cmd);
        self.view.dirty = true;
    }

    /// Sidebar slot under a frame-space position.
    fn sidebar_slot_at(&self, pos: (f64, f64), size: (u32, u32)) -> Option<usize> {
        sidebar::slot_at(
            size.0,
            size.1,
            self.session.as_ref()?,
            self.sidebar_view,
            pos,
        )
    }

    fn sidebar_left_press(&mut self, size: (u32, u32)) {
        if self.sidebar_scroll_click(size) {
            return;
        }
        if let Some(mode) = self
            .session
            .as_ref()
            .and_then(|s| sidebar::control_at(size.0, s, self.cursor))
        {
            self.toggle_service(mode);
            return;
        }
        if let Some(cell) = self.radar_target(self.cursor) {
            self.radar_click(cell);
            return;
        }
        if let Some(category) = self
            .session
            .as_ref()
            .and_then(|session| sidebar::session_tab_at(size.0, session, self.cursor))
        {
            self.sidebar_view.select(category);
            self.service_mode = ServiceMode::None;
            self.placing = None;
            self.view.dirty = true;
        } else if let Some(slot) = self.sidebar_slot_at(self.cursor, size) {
            self.sidebar_click(slot);
        }
    }

    fn sidebar_right_press(&mut self, size: (u32, u32)) {
        if self.service_mode != ServiceMode::None {
            self.order_feedback.clear();
            self.view.dirty = true;
        }
        self.service_mode = ServiceMode::None;
        if self.radar_target(self.cursor).is_some() {
            self.order_feedback.clear();
            self.interrupt_type_press();
            self.selected.clear();
            self.attack_move = false;
            self.placing = None;
            self.view.dirty = true;
        }
        if let Some(slot) = self.sidebar_slot_at(self.cursor, size) {
            self.sidebar_right_click(slot);
        }
    }

    fn in_sidebar(&self, pos: (f64, f64)) -> bool {
        let width = self.camera_size().0;
        pos.0 >= f64::from(width.saturating_sub(SIDEBAR_W))
    }

    fn sidebar_scroll_click(&mut self, size: (u32, u32)) -> bool {
        let Some(session) = &self.session else {
            return false;
        };
        let Some(rows) = self
            .sidebar_view
            .scroll_at(session, size.0, size.1, self.cursor)
        else {
            return false;
        };
        self.sidebar_view.scroll(session, size.1, rows);
        self.view.dirty = true;
        true
    }

    fn match_wheel(&mut self, delta: MouseScrollDelta, size: (u32, u32)) {
        if !self.pointer.focused
            || !self.pointer.inside
            || self.menu.visible()
            || self.left_down.is_some()
            || self.right_down.is_some()
            || self.radar_drag
        {
            return;
        }
        let rows = match delta {
            MouseScrollDelta::LineDelta(_, y) => f64::from(y),
            MouseScrollDelta::PixelDelta(p) => p.y / 50.0,
        };
        if !rows.is_finite() || rows == 0.0 {
            return;
        }
        self.interrupt_type_press();
        if self.cursor.0 >= f64::from(size.0.saturating_sub(SIDEBAR_W)) {
            if let Some(session) = &self.session {
                self.sidebar_view.wheel(session, size.1, -rows);
            }
        } else {
            self.zoom_camera((self.view.zoom * (1.0 + rows as f32 * 0.1)).clamp(0.25, 4.0));
        }
        self.view.dirty = true;
    }

    fn tick_session(&mut self) {
        if self.menu.visible() {
            return;
        }
        let Some(session) = &mut self.session else {
            return;
        };
        let cmds = std::mem::take(&mut self.pending);
        self.effects.remember(session);
        self.animations.update(&session.sim);
        if let Some(audio) = &mut self.audio {
            audio.commands(session, &cmds);
        }
        session.step(&cmds);
        self.animations.update(&session.sim);
        let events = session.sim.drain_events();
        if let Some(bank) = &self.sprites {
            self.effects
                .update(session, &events, &mut bank.borrow_mut());
        }
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
        self.compose_at(frame, fw, fh, Instant::now());
    }

    fn compose_at(&self, frame: &mut [u8], fw: u32, fh: u32, now: Instant) {
        if self.menu.page == yr_app::menu::Page::Loading && self.menu.loading.battle {
            let preview = self.loading_preview.as_deref().or_else(|| {
                self.menu
                    .loading
                    .map_name
                    .eq_ignore_ascii_case(&self.map_name)
                    .then_some(self.front_preview.as_ref())
                    .flatten()
            });
            yr_app::loading::draw(frame, fw, fh, &self.loading_art, &self.menu, preview);
            if self
                .pointer
                .custom(&self.loading_art.cursor, (fw, fh), self.cursor)
            {
                self.loading_art.cursor.draw(
                    frame,
                    (fw, fh),
                    self.cursor,
                    yr_app::cursor::Kind::Pointer,
                    self.front_phase,
                );
            }
            return;
        }
        if self.menu.front {
            yr_app::frontend::draw(
                frame,
                fw,
                fh,
                &self.front_art,
                yr_app::frontend::Scene {
                    state: &self.menu,
                    movie: self.movie.as_ref().and_then(|v| v.current.as_ref()),
                    preview: if self.menu.page == yr_app::menu::Page::ChooseMap {
                        self.chooser_preview
                            .as_ref()
                            .filter(|_| self.menu.maps.ready == Some(self.menu.maps.selected))
                    } else {
                        self.front_preview.as_ref()
                    },
                    phase: self.front_phase,
                    cursor: self.cursor,
                },
            );
            if self
                .pointer
                .custom(&self.front_art.cursor, (fw, fh), self.cursor)
            {
                self.front_art.cursor.draw(
                    frame,
                    (fw, fh),
                    self.cursor,
                    yr_app::cursor::Kind::Pointer,
                    self.front_phase,
                );
            }
            return;
        }
        if self.menu.visible() {
            yr_app::menu::draw(frame, fw, fh, &self.menu_art, &self.menu, self.cursor);
            if let Some(session) = &self.session {
                if self
                    .pointer
                    .custom(&session.cursor_art, (fw, fh), self.cursor)
                {
                    session.cursor_art.draw(
                        frame,
                        (fw, fh),
                        self.cursor,
                        yr_app::cursor::Kind::Pointer,
                        session.sim.tick(),
                    );
                }
            }
            return;
        }
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
            death: Option<(u8, u64)>,
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
            let to_win = |cx: f32, cy: f32| {
                let (mx, my) = yr_app::interaction::ground_point(map, sim.terrain(), (cx, cy));
                ((mx - self.view.cam_x) * zoom, (my - self.view.cam_y) * zoom)
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
                    col: selected_owner_col(self.session.as_ref(), b.owner),
                    selected: false,
                    hp: hp_frac(b.hp, b.max_hp),
                    art: (b.owner != yr_sim::OWNER_NEUTRAL)
                        .then(|| kind_id(b.kind).map(|k| (k, 0, b.owner)))
                        .flatten(),
                    animation: Default::default(),
                    building: Some(yr_app::animation::building_animation(sim, b)),
                    death: None,
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
                    col: selected_owner_col(self.session.as_ref(), u.owner),
                    selected: self.selected.contains(&u.id),
                    hp: hp_frac(u.hp, u.max_hp),
                    art: (u.owner != yr_sim::OWNER_NEUTRAL)
                        .then(|| kind_id(u.kind).map(|k| (k, u.facing, u.owner)))
                        .flatten(),
                    animation: self.animations.get(u.id),
                    building: None,
                    death: None,
                });
            }
            for death in &self.effects.deaths {
                let (fx, fy) = (death.x.raw() as f32 / 256.0, death.y.raw() as f32 / 256.0);
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
                    half_w: 0.0,
                    col: [0; 4],
                    selected: false,
                    hp: 1.0,
                    art: Some((death.kind.clone(), death.facing, death.owner)),
                    animation: Default::default(),
                    building: None,
                    death: Some((death.sequence, tick.saturating_sub(death.born))),
                });
            }
        }
        // Ground-position combat art shares entity depth instead of painting
        // through every foreground body. Equal-position shots follow entities;
        // trails precede projectile bodies, which precede other transients.
        #[derive(Clone, Copy)]
        enum WorldArt<'a> {
            Entity(usize),
            Projectile(u64, Option<(i32, i32)>),
            Effect(&'a yr_app::effects::Effect),
            Trail(&'a yr_app::effects::Trail),
        }
        let mut world: Vec<_> = markers
            .iter()
            .enumerate()
            .map(|(index, m)| (m.footprint, WorldArt::Entity(index)))
            .collect();
        let point = |x: Fix, y: Fix| yr_render::depth::Footprint {
            x: x.raw() as f32 / 256.0,
            y: y.raw() as f32 / 256.0,
            w: 0.0,
            h: 0.0,
        };
        world.extend(self.effects.trails.iter().map(|trail| {
            (
                point(
                    Fix::from_raw(
                        ((i64::from(trail.from.0.raw()) + i64::from(trail.to.0.raw())) / 2) as i32,
                    ),
                    Fix::from_raw(
                        ((i64::from(trail.from.1.raw()) + i64::from(trail.to.1.raw())) / 2) as i32,
                    ),
                ),
                WorldArt::Trail(trail),
            )
        }));
        if let Some(session) = &self.session {
            world.extend(session.sim.projectiles().iter().filter_map(|projectile| {
                Some((
                    point(projectile.x, projectile.y),
                    WorldArt::Projectile(
                        projectile.origin?.weapon?,
                        session
                            .sim
                            .projectile_motion(projectile)
                            .map(|(x, y)| (x.raw(), y.raw())),
                    ),
                ))
            }));
        }
        world.extend(self.effects.active.iter().map(|effect| {
            (
                effect.foundation.map_or_else(
                    || point(effect.x, effect.y),
                    |foundation| yr_render::depth::Footprint {
                        x: f32::from(foundation.cell.0),
                        y: f32::from(foundation.cell.1),
                        w: f32::from(foundation.size.0),
                        h: f32::from(foundation.size.1),
                    },
                ),
                WorldArt::Effect(effect),
            )
        }));
        let order = yr_render::depth::painter_order(
            &world
                .iter()
                .map(|(footprint, _)| *footprint)
                .collect::<Vec<_>>(),
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
                let remap = owner_remap(bank, *owner, self.session.as_ref());
                if let Some(bib) = bank.get_building(id, remap, animation).and_then(|b| b.bib) {
                    blit_sprite(frame, fw, fh, bib, m.ax, m.ay, k, SpritePass::Body);
                }
            }
        }
        // Union the shadow masks on the ground before drawing bodies.
        // Overlapping base/overlay shadows darken a pixel only once.
        let mut shadowed = vec![false; (fw * fh) as usize];
        for m in &markers {
            // Generic death sheets contain effect pixels, not unit shadows.
            if m.death.is_some_and(|(sequence, _)| sequence > 2) {
                continue;
            }
            if let (Some((id, facing, owner)), Some(bank)) = (&m.art, bank.as_mut()) {
                let remap = owner_remap(bank, *owner, self.session.as_ref());
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
                } else if let Some(sprite) = match m.death {
                    Some((sequence, age)) => {
                        bank.get_infantry_death(id, sequence, *facing, remap, age)
                    }
                    None => bank.get_animated_at(id, *facing, remap, m.animation, tick),
                } {
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
            let (footprint, art) = world[index];
            let WorldArt::Entity(index) = art else {
                if let WorldArt::Trail(trail) = art {
                    yr_app::trails::draw(
                        frame,
                        trail,
                        tick,
                        yr_app::trails::View {
                            size: (fw, fh),
                            viewport_width: fw.saturating_sub(SIDEBAR_W),
                            map,
                            terrain: self.session.as_ref().and_then(|s| s.sim.terrain()),
                            camera: (self.view.cam_x, self.view.cam_y, self.view.zoom),
                        },
                    );
                    continue;
                }
                if let Some(bank) = bank.as_mut() {
                    let (sprite, position) = match art {
                        WorldArt::Projectile(source, motion) => (
                            bank.get_projectile_directed_at(source, motion, tick),
                            (footprint.x, footprint.y),
                        ),
                        WorldArt::Effect(effect) => (
                            bank.get_effect(&effect.name, tick.saturating_sub(effect.born)),
                            (effect.x.raw() as f32 / 256.0, effect.y.raw() as f32 / 256.0),
                        ),
                        WorldArt::Entity(_) | WorldArt::Trail(_) => unreachable!(),
                    };
                    if let Some(sprite) = sprite {
                        let (mx, my) = yr_app::interaction::ground_point(
                            map,
                            self.session.as_ref().and_then(|s| s.sim.terrain()),
                            position,
                        );
                        blit_sprite(
                            frame,
                            fw,
                            fh,
                            sprite,
                            (mx - self.view.cam_x) * self.view.zoom,
                            (my - self.view.cam_y) * self.view.zoom,
                            k,
                            SpritePass::Effect,
                        );
                    }
                }
                continue;
            };
            let m = &markers[index];
            let drawn = match (&m.art, bank.as_mut()) {
                (Some((id, facing, owner)), Some(bank)) => {
                    let remap = owner_remap(bank, *owner, self.session.as_ref());
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
                        let sprite = match m.death {
                            Some((sequence, age)) => {
                                bank.get_infantry_death(id, sequence, *facing, remap, age)
                            }
                            None => bank.get_animated_at(id, *facing, remap, m.animation, tick),
                        };
                        if let Some(s) = sprite {
                            let pass = if m.death.is_some_and(|(sequence, _)| sequence > 2) {
                                SpritePass::Effect
                            } else {
                                SpritePass::Body
                            };
                            blit_sprite(frame, fw, fh, s, m.ax, m.ay, k, pass);
                            true
                        } else {
                            false
                        }
                    }
                }
                _ => false,
            };
            if !drawn && m.death.is_none() {
                draw_diamond(frame, fw, fh, m.wx, m.wy, m.half_w, m.col);
            }
        }
        drop(bank);
        for m in &markers {
            if m.selected {
                unit_status::draw_brackets(
                    frame,
                    (fw, fh),
                    (m.wx, m.wy),
                    m.half_w.max(8.0) + 6.0,
                    SELECT_COL,
                );
            }
            // Preserve the existing selected-or-damaged visibility policy.
            if m.selected || m.hp < 1.0 {
                unit_status::draw_health(
                    frame,
                    (fw, fh),
                    (m.wx, m.wy - (m.half_w.max(8.0) + 6.0) * 0.5 - 6.0),
                    m.half_w * 2.0,
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
        self.draw_order_marker(frame, (fw, fh), now);
        self.draw_attack_targets(frame, (fw, fh));
        self.draw_hover_target(frame, (fw, fh));
        if let Some(session) = &self.session {
            sidebar::draw(frame, fw, fh, session, self.sidebar_view, self.placing);
            sidebar::draw_controls(frame, fw, fh, session, self.service_mode);
            let corners = [
                map.px_to_cell(self.view.cam_x, self.view.cam_y),
                map.px_to_cell(
                    self.view.cam_x + (fw - SIDEBAR_W) as f32 / self.view.zoom,
                    self.view.cam_y + fh as f32 / self.view.zoom,
                ),
            ];
            session.radar.draw_viewport(frame, fw, fh, corners);
            sidebar::draw_tooltip(frame, fw, fh, session, self.sidebar_view, self.cursor);
            if let Some(message) = self
                .order_feedback
                .message(now)
                .or_else(|| self.groups.message(now))
            {
                yr_app::interaction::draw_feedback(frame, fw, fh, session, message);
            }
        }
        // Box-select rubber band.
        if let (Some(a), false) = (self.left_down, self.right_panning) {
            if (self.cursor.0 - a.0).abs() > 6.0 || (self.cursor.1 - a.1).abs() > 6.0 {
                if let Some(bounds) = selection_box::Bounds::new(a, self.cursor, (fw, fh)) {
                    bounds.draw(frame, fw, fh, SELECT_COL);
                }
            }
        }
        if let Some(session) = &self.session {
            if self
                .pointer
                .custom(&session.cursor_art, (fw, fh), self.cursor)
            {
                session.cursor_art.draw(
                    frame,
                    (fw, fh),
                    self.cursor,
                    self.cursor_kind((fw, fh)),
                    tick,
                );
            }
        }
    }
}

fn selected_owner_col(session: Option<&Session>, owner: u8) -> [u8; 4] {
    session
        .and_then(|s| s.owner_colors.get(usize::from(owner)))
        .map_or_else(|| owner_col(owner), |c| [c[0], c[1], c[2], 255])
}

fn owner_col(owner: u8) -> [u8; 4] {
    match owner {
        yr_sim::OWNER_PLAYER => PLAYER_COL,
        yr_sim::OWNER_ENEMY => ENEMY_COL,
        _ => NEUTRAL_COL,
    }
}

/// Selected house colors for live participants, with neutral artwork fallback.
fn owner_remap(
    bank: &yr_render::sprites::SpriteBank,
    owner: u8,
    session: Option<&Session>,
) -> [u8; 3] {
    match owner {
        yr_sim::OWNER_PLAYER => {
            session.map_or_else(|| bank.house_color("Americans"), |s| s.owner_colors[0])
        }
        yr_sim::OWNER_ENEMY => {
            session.map_or_else(|| bank.house_color("Russians"), |s| s.owner_colors[1])
        }
        _ => bank.house_color(""),
    }
}

enum SpritePass<'a> {
    Body,
    Effect,
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
                (alpha, SpritePass::Effect) if alpha > 0 => {
                    for c in 0..3 {
                        frame[dst + c] = ((u32::from(s.rgba[src + c]) * u32::from(alpha)
                            + u32::from(frame[dst + c]) * u32::from(255 - alpha))
                            / 255) as u8;
                    }
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

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title(format!("OpenYR: {}", self.map_name))
            .with_inner_size(LogicalSize::new(WIN_W, WIN_H))
            .with_min_inner_size(LogicalSize::new(640, 480));
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
        let frame =
            window_size::frame_for_surface((size.width, size.height), window.scale_factor())
                .unwrap_or((1, 1));
        let pixels = match Pixels::new(frame.0, frame.1, surface) {
            Ok(pixels) => pixels,
            Err(e) => {
                self.startup_error = Some(format!("cannot create display surface: {e}"));
                el.exit();
                return;
            }
        };
        self.frame_size = frame;
        if let Ok(fs) = yr_assets::AssetFs::open(std::path::Path::new(&self.gamedir)) {
            self.menu_art = yr_app::menu::Art::load(&fs);
            self.loading_art = yr_app::loading::Art::load(&fs);
        }
        self.pointer.focused = window.has_focus();
        if self.startup_menu {
            self.window = Some(window);
            self.pixels = Some(pixels);
            let preferences_error = match &self.menu.page {
                yr_app::menu::Page::Message { text, .. } => Some(text.clone()),
                _ => None,
            };
            self.enter_front();
            if let Some(error) = preferences_error {
                self.menu.message(error, yr_app::menu::Page::Main);
            }
            return;
        }
        match yr_render::render_map(&self.gamedir, &self.map_name, self.scale) {
            Ok(map) => {
                println!("OpenYR: rendered {}", map.summary);
                self.view
                    .center_map(&map, (frame.0.saturating_sub(SIDEBAR_W), frame.1));
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
                    Ok(audio) => {
                        audio.set_muted(self.menu.muted);
                        self.audio = Some(audio);
                    }
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
        println!("OpenYR: left-click select/order, box select, T=select visible matching types, Ctrl+1-9=assign team, 1-9=recall (twice=center), Shift+number=add team, right-click cancel, A=attack-move, S=stop, H=center base, Ctrl+H=harvest, K=repair, L=sell, M=mute, F5=save, F9=load, Esc=pause, right-drag/arrows pan, wheel zoom; sidebar: left-click build or place a ready structure, right-click hold/cancel");
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
                if self
                    .session
                    .as_ref()
                    .is_some_and(|s| s.outcome == Outcome::Ongoing)
                {
                    self.open_menu(yr_app::menu::Page::Abort);
                    return;
                }
                if let Some(window) = &self.window {
                    window.set_cursor_visible(true);
                }
                self.report_audio();
                el.exit();
            }
            WindowEvent::Resized(size) => {
                let scale = self
                    .window
                    .as_ref()
                    .map_or(1.0, |window| window.scale_factor());
                if let Err(error) = self.resize_window((size.width, size.height), scale) {
                    eprintln!("OpenYR: display resize failed ({error})");
                }
            }
            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(window) = &self.window {
                    let size = window.inner_size();
                    if let Err(error) = self.resize_window((size.width, size.height), scale_factor)
                    {
                        eprintln!("OpenYR: display scale change failed ({error})");
                    }
                } else {
                    self.invalidate_pointer();
                }
            }
            WindowEvent::Focused(focused) => self.focus_changed(focused),
            WindowEvent::CursorEntered { .. } | WindowEvent::CursorLeft { .. } => {
                self.invalidate_pointer()
            }
            WindowEvent::ModifiersChanged(m) => {
                self.shift = m.state().shift_key();
                self.control = m.state().control_key();
                self.super_key = m.state().super_key();
            }
            WindowEvent::MouseInput { state, button, .. } => self.pointer_button(state, button),
            WindowEvent::CursorMoved { position, .. } => {
                self.pointer_moved((position.x, position.y))
            }
            WindowEvent::MouseWheel { delta, .. } => self.pointer_wheel(delta),
            WindowEvent::KeyboardInput {
                event,
                is_synthetic,
                ..
            } => self.keyboard_input(
                event.physical_key,
                event.state,
                event.text.as_deref(),
                event.repeat,
                is_synthetic,
            ),
            WindowEvent::RedrawRequested => {
                self.view.dirty |= self.order_feedback.expire(Instant::now());
                self.poll_front_presentation();
                self.poll_menu_load();
                let stepped = self.advance_clock(Instant::now());
                if !self.menu.visible()
                    && self
                        .session
                        .as_ref()
                        .is_some_and(|s| s.outcome != Outcome::Ongoing)
                {
                    self.open_menu(yr_app::menu::Page::Result(
                        self.session
                            .as_ref()
                            .is_some_and(|s| s.outcome == Outcome::Won),
                    ));
                }
                self.update_camera(Instant::now(), self.camera_size());
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
        if self.quit_requested {
            if let Some(window) = &self.window {
                window.set_cursor_visible(true);
            }
            self.report_audio();
            el.exit();
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
        frame_size: (WIN_W, WIN_H),
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
        effects: Default::default(),
        view: View {
            cam_x: 0.0,
            cam_y: 0.0,
            zoom: 1.0,
            dirty: true,
        },
        camera: Default::default(),
        selected: BTreeSet::new(),
        groups: Default::default(),
        type_press: None,
        order_feedback: Default::default(),
        pending: Vec::new(),
        attack_move: false,
        shift: false,
        control: false,
        super_key: false,
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
        suppressed_keys: std::collections::HashSet::new(),
        held_keys: std::collections::HashSet::new(),
        last_tick: Instant::now(),
        accum: Duration::ZERO,
        title: String::new(),
        announced: false,
        placing: None,
        sidebar_view: Default::default(),
        service_mode: ServiceMode::None,
        load_path: None,
        save_path: yr_app::snapshot::quicksave_path(),
        save_requested: false,
        startup_error: None,
        menu: Default::default(),
        menu_art: Default::default(),
        load_job: None,
        loading_art: Default::default(),
        loading_preview: None,
        preferences_path: None,
        quit_requested: false,
        startup_menu: false,
        front_art: Default::default(),
        front_preview: None,
        chooser_preview: None,
        chooser_job: None,
        chooser_epoch: 0,
        movie: None,
        front_epoch: Instant::now(),
        front_phase: 0,
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
                let (vw, vh) = ((fw - SIDEBAR_W) as f32 / 2.0, fh as f32 / 2.0);
                app.view.center_on_ground(
                    map,
                    sim.terrain(),
                    (cx + 0.5, cy + 0.5),
                    (fw - SIDEBAR_W, fh),
                );
                // The ghost follows the cursor: park it mid-view.
                app.cursor = (f64::from(vw), f64::from(vh));
            }
            // Exercise every shared tab at the same sim tick. The default
            // Structures view keeps the existing proof filename.
            let out = format!("{dir}/frame-{t:05}.png");
            for category in Category::ALL {
                app.sidebar_view.select(category);
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
        scale: 1,
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
    if !matches!(out.scale, 1 | 2 | 4) {
        return Err("scale must be 1, 2 or 4".into());
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
        println!("{}\n\nWithout arguments, OpenYR finds your retail install and opens the main menu.\nSet OPENYR_GAMEDIR to override the remembered or standard install.", launcher::USAGE);
        return Ok(());
    }
    let startup_menu = args.is_empty();
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
    app.startup_menu = startup_menu;
    app.read_preferences();
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
    fn default_view_keeps_native_art_pixels() {
        let opts = parse_args(&["retail".into(), "EB1.MMX".into()]).unwrap();
        assert_eq!(
            opts.scale, 1,
            "the default must not discard three of every four source pixels"
        );
    }

    #[test]
    fn render_scale_validation_matches_the_renderer() {
        for scale in [0, 1, 2, 3, 4, 5, 6, 7, 8, u32::MAX] {
            let result = parse_args(&["game".into(), "EB1.MMX".into(), scale.to_string()]);
            assert_eq!(result.is_ok(), matches!(scale, 1 | 2 | 4));
        }
    }

    #[test]
    fn effect_translucency_blends_color_without_painting_a_shadow() {
        let sprite = yr_render::sprites::Sprite {
            w: 2,
            h: 1,
            ax: 0,
            ay: 0,
            rgba: vec![240, 120, 0, 128, 255, 255, 255, 0],
        };
        let mut frame = vec![100, 80, 60, 255, 100, 80, 60, 255];
        blit_sprite(&mut frame, 2, 1, &sprite, 0.0, 0.0, 1.0, SpritePass::Effect);
        assert_eq!(frame, [170, 100, 29, 255, 100, 80, 60, 255]);
    }

    #[test]
    fn retail_sparse_boundary_cursor_proof() {
        let dir = PathBuf::from(std::env::var_os("HOME").unwrap())
            .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
        if !dir.is_dir() {
            return;
        }
        let dir = dir.to_str().unwrap();
        let mut app = new_app(dir.into(), "EB1.MMX".into(), 1);
        app.map = Some(yr_render::render_map(dir, "EB1.MMX", 1).unwrap());
        app.sprites = load_sprites(dir, "EB1.MMX");
        let mut session = app.load_session().unwrap();
        session.sim.set_ai(None);
        let grid = session.sim.terrain().unwrap();
        let map = app.map.as_ref().unwrap();
        let (land, hole) = (1..grid.height())
            .find_map(|ly| {
                (1..grid.width()).find_map(|lx| {
                    let land = grid.cell_at_local(lx, ly);
                    let hole = (land.0 - 1, land.1);
                    let center = (f32::from(hole.0) + 0.5, f32::from(hole.1) + 0.5);
                    let pixel = map.cell_to_px(center.0, center.1);
                    (grid.passable(yr_sim::terrain::Loco::Track, land.0, land.1)
                        && grid.cell(land.0, land.1).unwrap().height == 0
                        && grid.cell(hole.0, hole.1).is_none()
                        && yr_app::interaction::cell_at(map, Some(grid), pixel) == center)
                        .then_some((land, hole))
                })
            })
            .expect("flat retail map edge with an unobscured missing cell");
        let tank = session
            .sim
            .units()
            .iter()
            .find(|u| u.owner == yr_sim::OWNER_PLAYER)
            .unwrap()
            .clone();
        let id = session.sim.next_id_peek();
        session.sim.step(&[
            Command::Spawn {
                kind: tank.kind,
                x: Fix::from_int(i32::from(land.0)) + Fix::from_raw(128),
                y: Fix::from_int(i32::from(land.1)) + Fix::from_raw(128),
                loco: tank.loco,
                speed: tank.speed,
                hp: tank.max_hp,
                armor: tank.armor,
                weapon: None,
            },
            Command::AssignOwner {
                id,
                owner: yr_sim::OWNER_PLAYER,
            },
        ]);
        app.session = Some(session);
        app.selected.insert(id);
        app.view.center_on_cell(
            map,
            (f32::from(land.0) + 0.5, f32::from(land.1) + 0.5),
            (WIN_W - SIDEBAR_W, WIN_H),
        );
        let point = map.cell_to_px(f32::from(hole.0) + 0.5, f32::from(hole.1) + 0.5);
        app.cursor = (
            f64::from(point.0 - app.view.cam_x),
            f64::from(point.1 - app.view.cam_y),
        );
        app.pointer.inside = true;
        assert_eq!(
            app.cursor_kind((WIN_W, WIN_H)),
            yr_app::cursor::Kind::Blocked
        );
        let hash = app.session.as_ref().unwrap().sim.state_hash();
        app.left_click(app.cursor);
        assert!(
            app.pending.is_empty(),
            "blocked cursor and click share validation"
        );
        let mut frame = vec![0; (WIN_W * WIN_H * 4) as usize];
        app.compose(&mut frame, WIN_W, WIN_H);
        assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), hash);
        if let Some(out) = std::env::var_os("OPENYR_TERRAIN_DUMP") {
            std::fs::create_dir_all(&out).unwrap();
            std::fs::write(
                PathBuf::from(&out).join("blocked-boundary-t00002.png"),
                yr_render::png::encode_rgba(WIN_W, WIN_H, &frame),
            )
            .unwrap();
            std::fs::write(PathBuf::from(out).join("boundary.json"), serde_json::to_vec_pretty(&serde_json::json!({
                "tick": app.session.as_ref().unwrap().sim.tick(), "land": land, "hole": hole,
                "cursor": "Blocked", "pending_commands": app.pending.len(), "state_hash": format!("{hash:016x}"),
            })).unwrap()).unwrap();
        }
        println!("EB1 boundary tick 2: selected unit at {land:?}, blocked cursor over {hole:?}, click queues no orders, composition preserves hash {hash:016x}");
    }

    #[test]
    fn retail_native_detail_and_event_effect_proof() {
        let dir = PathBuf::from(std::env::var("HOME").unwrap())
            .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
        if !dir.is_dir() {
            return;
        }
        let dir = dir.to_str().unwrap();
        let prepare = |scale| {
            let mut app = new_app(dir.into(), "EB1.MMX".into(), scale);
            app.map = Some(yr_render::render_map(dir, "EB1.MMX", scale).unwrap());
            app.sprites = load_sprites(dir, "EB1.MMX");
            let mut session = app.load_session().unwrap();
            session.sim.set_ai(None);
            app.session = Some(session);
            app
        };
        let capture = |app: &App, name: &str, full: bool| {
            let hash = app.session.as_ref().unwrap().sim.state_hash();
            let save = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
            let mut frame = vec![0; (WIN_W * WIN_H * 4) as usize];
            app.compose(&mut frame, WIN_W, WIN_H);
            if let Some(out) = std::env::var_os("OPENYR_VISUAL_DUMP") {
                let out = PathBuf::from(out);
                std::fs::create_dir_all(&out).unwrap();
                if full {
                    std::fs::write(
                        out.join(format!("{name}.png")),
                        yr_render::png::encode_rgba(WIN_W, WIN_H, &frame),
                    )
                    .unwrap();
                }
                let mut crop = Vec::with_capacity(400 * 280 * 4);
                for y in 260..540 {
                    let at = ((y * WIN_W + 356) * 4) as usize;
                    crop.extend_from_slice(&frame[at..at + 400 * 4]);
                }
                std::fs::write(
                    out.join(format!("{name}-detail.png")),
                    yr_render::png::encode_rgba(400, 280, &crop),
                )
                .unwrap();
                let metadata = serde_json::json!({
                    "tick": app.session.as_ref().unwrap().sim.tick(),
                    "state_hash": format!("{hash:016x}"),
                    "country": yr_app::session::PLAYER_COUNTRY,
                    "projectiles": app.session.as_ref().unwrap().sim.projectiles(),
                    "player_units": app.session.as_ref().unwrap().sim.units().iter()
                        .filter(|u| u.owner == yr_sim::OWNER_PLAYER)
                        .map(|u| serde_json::json!({
                            "id": u.id, "kind": app.session.as_ref().unwrap().kinds[usize::from(u.kind)],
                            "role": u.role, "hp": u.max_hp,
                            "x_raw": u.x.raw(), "y_raw": u.y.raw(),
                        })).collect::<Vec<_>>(),
                    "effects": app.effects.active.iter().map(|effect| serde_json::json!({
                        "name": effect.name, "born": effect.born,
                        "x_raw": effect.x.raw(), "y_raw": effect.y.raw(),
                    })).collect::<Vec<_>>(),
                });
                std::fs::write(
                    out.join(format!("{name}.json")),
                    serde_json::to_vec_pretty(&metadata).unwrap(),
                )
                .unwrap();
            }
            assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), hash);
            assert!(yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap() == save);
            frame
        };
        let opts = parse_args(&[dir.into(), "EB1.MMX".into()]).unwrap();
        let mut app = prepare(opts.scale);
        let tank = app
            .session
            .as_ref()
            .unwrap()
            .sim
            .units()
            .iter()
            .find(|u| {
                u.owner == yr_sim::OWNER_PLAYER
                    && app.session.as_ref().unwrap().kinds[usize::from(u.kind)] == "MTNK"
                    && u.weapon.is_some()
                    && app
                        .sprites
                        .as_ref()
                        .unwrap()
                        .borrow()
                        .muzzle_effect(
                            &app.session.as_ref().unwrap().kinds[usize::from(u.kind)],
                            u.facing,
                        )
                        .as_deref()
                        == Some("GUNFIRE")
            })
            .unwrap()
            .clone();
        let focus = (tank.x.raw() as f32 / 256.0, tank.y.raw() as f32 / 256.0);
        app.view
            .center_on_cell(app.map.as_ref().unwrap(), focus, (WIN_W - SIDEBAR_W, WIN_H));
        let native = capture(&app, "native-default-t00001", true);
        let mut before = prepare(2);
        before.view.center_on_cell(
            before.map.as_ref().unwrap(),
            focus,
            (WIN_W - SIDEBAR_W, WIN_H),
        );
        capture(&before, "old-default-t00001", true);
        before.view.zoom = 2.0;
        before.view.center_on_cell(
            before.map.as_ref().unwrap(),
            focus,
            (WIN_W - SIDEBAR_W, WIN_H),
        );
        let lossy = capture(&before, "old-matched-magnification-t00001", true);
        assert!(
            native.iter().zip(&lossy).filter(|(a, b)| a != b).count() > 10_000,
            "half-resolution terrain cannot recover its source detail by zooming back in"
        );
        drop(before);
        let session = app.session.as_ref().unwrap();
        let grid = session.sim.terrain().unwrap();
        let fs = yr_assets::AssetFs::open(std::path::Path::new(dir)).unwrap();
        let retail_map =
            yr_formats::MapFile::parse(&String::from_utf8_lossy(fs.read("EB1.MMX").unwrap()))
                .unwrap();
        let present: BTreeSet<_> = retail_map.tiles.iter().map(|t| (t.x, t.y)).collect();
        let (cx, cy) = (6..grid.height().saturating_sub(6))
            .find_map(|ly| {
                (6..grid.width().saturating_sub(6))
                    .map(|lx| grid.cell_at_local(lx, ly))
                    .find(|&(x, y)| {
                        (-5..=5).all(|dy| {
                            (-5..=5).all(|dx| {
                                let (Some(px), Some(py)) = (
                                    u16::try_from(i32::from(x) + dx).ok(),
                                    u16::try_from(i32::from(y) + dy).ok(),
                                ) else {
                                    return false;
                                };
                                present.contains(&(px, py))
                                    && grid.passable(yr_sim::terrain::Loco::Track, px, py)
                                    && grid.cell(px, py).unwrap().height
                                        == grid.cell(x, y).unwrap().height
                            })
                        }) && session.sim.units().iter().all(|u| {
                            u.x.to_int_floor().abs_diff(i32::from(x)) > 6
                                || u.y.to_int_floor().abs_diff(i32::from(y)) > 6
                        })
                    })
            })
            .expect("open, level retail combat patch");
        let (x, y) = (
            yr_sim::Fix::from_int(i32::from(cx)) + yr_sim::Fix::from_raw(128),
            yr_sim::Fix::from_int(i32::from(cy)) + yr_sim::Fix::from_raw(128),
        );
        app.pending.push(Command::Spawn {
            kind: tank.kind,
            x,
            y,
            loco: tank.loco,
            speed: yr_sim::Fix::ZERO,
            hp: tank.max_hp,
            armor: tank.armor,
            weapon: tank.weapon,
        });
        app.tick_session();
        let shooter = app.session.as_ref().unwrap().sim.units().last().unwrap().id;
        app.pending.push(Command::AssignOwner {
            id: shooter,
            owner: yr_sim::OWNER_PLAYER,
        });
        app.view.zoom = 4.0;
        app.center_world_at(
            (f32::from(cx) + 1.5, f32::from(cy) + 0.5),
            (WIN_W - SIDEBAR_W, WIN_H),
        );
        app.pending.push(Command::Spawn {
            kind: tank.kind,
            x: x + yr_sim::Fix::from_int(2),
            y,
            loco: yr_sim::terrain::Loco::Track,
            speed: yr_sim::Fix::ZERO,
            hp: 2000,
            armor: tank.armor,
            weapon: None,
        });
        app.tick_session();
        let target = app.session.as_ref().unwrap().sim.units().last().unwrap().id;
        assert_eq!(
            app.session
                .as_ref()
                .unwrap()
                .sim
                .units()
                .iter()
                .find(|u| u.id == shooter)
                .unwrap()
                .owner,
            yr_sim::OWNER_PLAYER
        );
        app.pending.extend([
            Command::AssignOwner {
                id: target,
                owner: yr_sim::OWNER_ENEMY,
            },
            Command::Attack {
                id: shooter,
                target,
            },
        ]);
        app.tick_session();
        assert_eq!(
            app.session
                .as_ref()
                .unwrap()
                .sim
                .units()
                .iter()
                .find(|u| u.id == target)
                .unwrap()
                .owner,
            yr_sim::OWNER_ENEMY
        );
        assert!(
            app.effects.active.iter().any(|e| e.name == "GUNFIRE"),
            "actual shot emits the muzzle art"
        );
        let launch_tick = app.session.as_ref().unwrap().sim.tick();
        for phase in 0..3 {
            let session = app.session.as_ref().unwrap();
            assert!(!app.effects.active.iter().any(|e| e.name == "S_CLSN22"));
            assert_eq!(session.sim.projectiles().len(), 1);
            assert_eq!(
                session.sim.projectiles()[0].x.raw(),
                x.raw() + 160 * (phase + 1)
            );
            let tick = session.sim.tick();
            let with = capture(&app, &format!("flight-{phase:02}-t{tick:05}"), true);
            if phase == 1 {
                let session = app.session.as_mut().unwrap();
                let original = session.sim.clone();
                let hash = original.state_hash();
                // Hide only visual provenance in a controlled comparison.
                // Gameplay fields and the authoritative state hash stay equal.
                let mut hidden = serde_json::to_value(&original).unwrap();
                for projectile in hidden["projectiles"].as_array_mut().unwrap() {
                    projectile["origin"] = serde_json::Value::Null;
                }
                session.sim = serde_json::from_value(hidden).unwrap();
                assert_eq!(session.sim.state_hash(), hash);
                let without = capture(&app, "projectile-without-source-same-tick", true);
                app.session.as_mut().unwrap().sim = original;
                assert_ne!(
                    with, without,
                    "the retained cannon source draws visible pixels"
                );
                capture(&app, "projectile-with-source-same-tick", true);
                let saved = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
                if let Some(out) = std::env::var_os("OPENYR_VISUAL_DUMP") {
                    std::fs::write(PathBuf::from(out).join("flight-t00005.oyrsave"), &saved)
                        .unwrap();
                }
                yr_app::snapshot::restore(app.session.as_mut().unwrap(), &saved).unwrap();
                app.sprites = load_sprites(dir, "EB1.MMX");
                assert_eq!(
                    capture(&app, "projectile-restored-same-tick", true),
                    with,
                    "in-flight save/load and a fresh sprite bank reconstruct the same pixels"
                );
            }
            app.tick_session();
        }
        assert!(app.session.as_ref().unwrap().sim.projectiles().is_empty());
        assert!(
            app.effects.active.iter().any(|e| e.name == "S_CLSN22"
                && e.x == x + yr_sim::Fix::from_int(2)
                && e.y == y
                && e.born == app.session.as_ref().unwrap().sim.tick()),
            "actual shell arrival emits AP impact art"
        );
        println!("cannon proof: launch {launch_tick}, arrival {}, shooter {shooter}, target {target}, raw velocity {}, weapon source {:?}, delivered damage {}, S_CLSN22", app.session.as_ref().unwrap().sim.tick(), tank.weapon.unwrap().projectile_speed.raw(), tank.weapon.unwrap().source, tank.weapon.unwrap().damage);
        if let Some(out) = std::env::var_os("OPENYR_VISUAL_DUMP") {
            let weapon = tank.weapon.unwrap();
            let metadata = serde_json::json!({
                "actor_kind": app.session.as_ref().unwrap().kinds[usize::from(tank.kind)],
                "country": yr_app::session::PLAYER_COUNTRY,
                "shooter_id": shooter, "target_id": target,
                "weapon": weapon,
                "weapon_source_hex": weapon.source.map(|source| format!("{source:016x}")),
                "launch_tick": launch_tick, "zoom": app.view.zoom,
                "impact": "S_CLSN22", "arrival_tick": app.session.as_ref().unwrap().sim.tick(),
            });
            std::fs::write(
                PathBuf::from(out).join("impact-scene.json"),
                serde_json::to_vec_pretty(&metadata).unwrap(),
            )
            .unwrap();
        }
        for phase in 0..36 {
            let tick = app.session.as_ref().unwrap().sim.tick();
            capture(&app, &format!("fire-{phase:02}-t{tick:05}"), phase == 1);
            if phase == 4 {
                let active = std::mem::take(&mut app.effects.active);
                app.effects.active = active
                    .iter()
                    .filter(|e| e.name != "S_CLSN22")
                    .map(|e| yr_app::effects::Effect {
                        name: e.name.clone(),
                        x: e.x,
                        y: e.y,
                        born: e.born,
                        foundation: e.foundation,
                    })
                    .collect();
                let without = capture(&app, "impact-without-same-tick", true);
                app.effects.active = active;
                let with = capture(&app, "impact-with-same-tick", true);
                assert_ne!(
                    with, without,
                    "impact changes composed pixels without changing simulation state"
                );
            }
            app.tick_session();
        }
        assert!(app.effects.active.iter().all(|e| e.name != "S_CLSN22"));
        app.pending.push(Command::Destroy { id: target });
        app.tick_session();
        assert!(app.effects.active.iter().any(|e| e.name == "TWLT070"));
        for phase in 0..36 {
            let tick = app.session.as_ref().unwrap().sim.tick();
            if phase % 2 == 0 {
                capture(&app, &format!("death-{phase:02}-t{tick:05}"), phase == 6);
            }
            app.tick_session();
        }
        assert!(app.effects.active.iter().all(|e| e.name != "TWLT070"));
    }

    #[test]
    fn elevated_camera_targets_land_at_the_drawn_world_viewport_center() {
        let mut grid = yr_sim::TerrainGrid::new(0, 0, 64, 64);
        grid.cell_mut(21, 31).unwrap().height = 4;
        for scale in [1, 2, 4] {
            let map = yr_render::RenderedMap {
                width: 4000 / scale,
                height: 4000 / scale,
                rgba: Vec::new(),
                summary: String::new(),
                origin_x: -90,
                origin_y: -200,
                scale,
            };
            for zoom in [0.25, 1.0, 1.25, 4.0] {
                for viewport in [(1112, 800), (632, 600), (1592, 1000)] {
                    let mut view = View {
                        cam_x: 0.0,
                        cam_y: 0.0,
                        zoom,
                        dirty: false,
                    };
                    view.center_on_ground(&map, Some(&grid), (21.5, 31.5), viewport);
                    let (px, py) = map.cell_to_px(21.5, 31.5);
                    let drawn = (
                        (px - view.cam_x) * zoom,
                        (py - 60.0 / scale as f32 - view.cam_y) * zoom,
                    );
                    assert!((drawn.0 - viewport.0 as f32 / 2.0).abs() < 0.01);
                    assert!(
                        (drawn.1 - viewport.1 as f32 / 2.0).abs() < 0.01,
                        "scale {scale}, zoom {zoom}, viewport {viewport:?}: drawn {drawn:?}"
                    );
                    assert!(view.dirty);
                }
            }
        }
    }

    #[test]
    fn retail_elevated_camera_uses_the_drawn_anchor_for_centering_and_selection() {
        let dir = PathBuf::from(std::env::var("HOME").unwrap_or_default())
            .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
        if !dir.is_dir() {
            return;
        }
        let dir = dir.to_str().unwrap();
        let mut app = new_app(dir.into(), "EB1.MMX".into(), 1);
        app.session = Some(app.load_session().unwrap());
        app.sprites = load_sprites(dir, "EB1.MMX");
        assert!(app.sprites.is_some());
        let session = app.session.as_mut().unwrap();
        session.sim.set_ai(None);
        let template = session
            .sim
            .units()
            .iter()
            .find(|u| {
                u.owner == yr_sim::OWNER_PLAYER && session.kinds[usize::from(u.kind)] == "MTNK"
            })
            .unwrap()
            .clone();
        let cell = (62.5, 27.5);
        assert_eq!(
            session.sim.terrain().unwrap().cell(62, 27).unwrap().height,
            4
        );
        let id = session.sim.next_id_peek();
        session.step(&[
            Command::Spawn {
                kind: template.kind,
                x: Fix::from_raw(16000),
                y: Fix::from_raw(7040),
                loco: template.loco,
                speed: Fix::ZERO,
                hp: template.max_hp,
                armor: template.armor,
                weapon: template.weapon,
            },
            Command::AssignOwner {
                id,
                owner: yr_sim::OWNER_PLAYER,
            },
        ]);
        let hash = session.sim.state_hash();
        let save = yr_app::snapshot::encode(session).unwrap();
        let capture = |app: &App, size: (u32, u32), name: &str, point: (f32, f32)| {
            let mut frame = vec![0; (size.0 * size.1 * 4) as usize];
            app.compose(&mut frame, size.0, size.1);
            assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), hash);
            assert_eq!(
                yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
                save
            );
            if let Some(out) = std::env::var_os("OPENYR_CAMERA_PROOF") {
                let out = PathBuf::from(out);
                std::fs::create_dir_all(&out).unwrap();
                std::fs::write(
                    out.join(format!("{name}.png")),
                    yr_render::png::encode_rgba(size.0, size.1, &frame),
                )
                .unwrap();
                std::fs::write(out.join(format!("{name}.json")),serde_json::to_vec_pretty(&serde_json::json!({
                    "tick": app.session.as_ref().unwrap().sim.tick(), "hash": format!("{hash:016x}"), "size": size,
                    "target": cell, "height":4, "actor_id":id, "scale":app.map.as_ref().unwrap().scale,
                    "zoom":app.view.zoom, "camera":[app.view.cam_x,app.view.cam_y],"drawn_ground":point,
                })).unwrap()).unwrap();
            }
        };
        let mut matrix = Vec::new();
        for scale in [1, 2, 4] {
            app.scale = scale;
            app.map = Some(yr_render::render_map(dir, "EB1.MMX", scale).unwrap());
            for zoom in [0.25, 1.0, 1.25, 4.0] {
                app.view.zoom = zoom;
                for size in [(1280, 800), (800, 600), (1760, 1000)] {
                    let viewport = (size.0 - SIDEBAR_W, size.1);
                    let map = app.map.as_ref().unwrap();
                    // Expected ground pixels are independent of the new helper.
                    let flat = map.cell_to_px(cell.0, cell.1);
                    let expected_ground = (flat.0, flat.1 - 60.0 / scale as f32);
                    app.view.center_on_cell(map, cell, viewport);
                    let before = (
                        (expected_ground.0 - app.view.cam_x) * zoom,
                        (expected_ground.1 - app.view.cam_y) * zoom,
                    );
                    app.selected = [id].into();
                    let export = size == (1280, 800) && (zoom == 1.0 || zoom == 4.0)
                        || scale == 1 && zoom == 4.0;
                    let name = format!("s{scale}-z{zoom}-{}x{}", size.0, size.1);
                    if export {
                        capture(&app, size, &format!("{name}-flat"), before);
                    }
                    app.center_world_at(cell, viewport);
                    let after = (
                        (expected_ground.0 - app.view.cam_x) * zoom,
                        (expected_ground.1 - app.view.cam_y) * zoom,
                    );
                    assert!(
                        (after.0 - viewport.0 as f32 / 2.0).abs() < 0.01
                            && (after.1 - viewport.1 as f32 / 2.0).abs() < 0.01
                    );
                    let hit = app
                        .window_to_cell((f64::from(after.0), f64::from(after.1)))
                        .unwrap();
                    assert!((hit.0 - cell.0).abs() < 0.001 && (hit.1 - cell.1).abs() < 0.001);
                    app.selected.clear();
                    app.select_at((f64::from(after.0), f64::from(after.1)), false);
                    assert_eq!(app.selected, [id].into());
                    app.selected.clear();
                    app.select_box(
                        (f64::from(after.0) - 2.0, f64::from(after.1) - 2.0),
                        (f64::from(after.0) + 2.0, f64::from(after.1) + 2.0),
                        false,
                    );
                    assert_eq!(app.selected, [id].into());
                    assert_eq!(app.session.as_ref().unwrap().sim.state_hash(), hash);
                    assert_eq!(
                        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
                        save
                    );
                    if export {
                        capture(&app, size, &format!("{name}-ground"), after);
                    }
                    matrix.push(serde_json::json!({"scale":scale,"zoom":zoom,"size":size,"before":before,"after":after}));
                }
            }
        }
        if let Some(out) = std::env::var_os("OPENYR_CAMERA_PROOF") {
            std::fs::write(
                PathBuf::from(out).join("camera-matrix.json"),
                serde_json::to_vec_pretty(&matrix).unwrap(),
            )
            .unwrap();
        }
        println!("EB1 height-4 camera: {} scale/zoom/viewport cases; centered ground, click/box selection, hash {hash:016x} and save bytes preserved",matrix.len());
    }

    #[test]
    fn ground_camera_without_an_authored_height_keeps_flat_centering() {
        let map = yr_render::RenderedMap {
            width: 100,
            height: 100,
            rgba: Vec::new(),
            summary: String::new(),
            origin_x: -20,
            origin_y: -40,
            scale: 2,
        };
        let grid = yr_sim::TerrainGrid::from_tiles(0, 0, 2, 2, [(0, 0, 0, 13, 0)].into_iter());
        for (terrain, cell) in [
            (None, (0.5, 0.5)),
            (Some(&grid), (0.5, 0.5)),
            (Some(&grid), (1.5, 1.5)),
            (Some(&grid), (-0.5, 0.5)),
            (Some(&grid), (65536.5, 0.5)),
        ] {
            let mut flat = View {
                cam_x: 0.0,
                cam_y: 0.0,
                zoom: 1.25,
                dirty: false,
            };
            flat.center_on_cell(&map, cell, (632, 600));
            let mut ground = View {
                cam_x: 0.0,
                cam_y: 0.0,
                zoom: 1.25,
                dirty: false,
            };
            ground.center_on_ground(&map, terrain, cell, (632, 600));
            assert_eq!((ground.cam_x, ground.cam_y), (flat.cam_x, flat.cam_y));
        }
    }

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
