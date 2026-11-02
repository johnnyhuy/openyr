use super::*;
use std::io::Read;
use yr_app::menu::{Effect, Intent, Layout, Page};

pub(super) enum Prepared {
    Match {
        session: Box<Session>,
        map: yr_render::RenderedMap,
        map_name: String,
        save_path: Option<PathBuf>,
    },
    Preview(yr_app::frontend::Preview),
}
pub(super) struct LoadJob {
    pub(super) receiver: std::sync::mpsc::Receiver<Result<Prepared, String>>,
    pub(super) cancelled: bool,
    pub(super) back: Page,
}
#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Preferences {
    muted: bool,
    zoom: u16,
    #[serde(default)]
    edge_scroll: bool,
}

impl App {
    fn save_directory(&self) -> &std::path::Path {
        self.save_path
            .parent()
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or(std::path::Path::new("."))
    }
    pub(super) fn escape_command_or_pause(&mut self) {
        self.order_feedback.clear();
        self.cancel_pointer_action();
        self.keys.clear();
        self.shift = false;
        self.control = false;
        if self.service_mode != ServiceMode::None {
            self.service_mode = ServiceMode::None;
        } else if self.placing.take().is_some() {
        } else if self.attack_move {
            self.attack_move = false;
        } else {
            self.open_menu(Page::Pause);
        }
        self.view.dirty = true;
    }
    pub(super) fn reset_menu_clock(&mut self) {
        self.accum = Duration::ZERO;
        self.last_tick = Instant::now();
        self.view.dirty = true;
    }
    pub(super) fn open_menu(&mut self, page: Page) {
        self.groups.interrupt();
        self.order_feedback.clear();
        if page != Page::Loading {
            if let Some(job) = &mut self.load_job {
                job.cancelled = true;
            }
        }
        self.cancel_pointer_action();
        self.keys.clear();
        self.shift = false;
        self.control = false;
        self.attack_move = false;
        self.placing = None;
        self.service_mode = ServiceMode::None;
        self.menu.zoom = (self.view.zoom * 100.0).round().clamp(25.0, 400.0) as u16;
        if matches!(page, Page::Main | Page::Pause | Page::Save | Page::Load) {
            match yr_app::menu::slots(self.save_directory()) {
                Ok(slots) => {
                    self.menu.slots = slots;
                    self.menu.selected = None;
                    self.menu.scroll = 0;
                }
                Err(e) => {
                    self.menu.message(
                        e,
                        if self.menu.front {
                            Page::Main
                        } else {
                            Page::Pause
                        },
                    );
                    self.reset_menu_clock();
                    return;
                }
            }
            if self.menu.name.is_empty() {
                self.menu.name = format!(
                    "Match {}",
                    self.session.as_ref().map_or(0, |s| s.sim.tick())
                );
            }
        }
        self.menu.set(page);
        self.reset_menu_clock();
    }
    pub(super) fn advance_clock(&mut self, now: Instant) -> bool {
        if self.menu.visible() {
            self.last_tick = now;
            self.accum = Duration::ZERO;
            return false;
        }
        self.accum += now.saturating_duration_since(self.last_tick);
        self.last_tick = now;
        let mut stepped = false;
        while self.accum >= TICK {
            self.accum -= TICK;
            if self.session.is_some() {
                self.tick_session();
                stepped = true;
            }
        }
        stepped
    }
    pub(super) fn menu_mouse(&mut self, state: ElementState, button: MouseButton) {
        let layout = Layout::for_state(&self.menu, WIN_W, WIN_H);
        if button == MouseButton::Left {
            match state {
                ElementState::Pressed => {
                    self.menu.keyboard_focus = false;
                    self.menu.pressed = layout.hit(&self.menu, self.cursor);
                    if self.menu.front && self.menu.page == Page::Skirmish {
                        self.menu.setup.focus = None;
                        if let Some((x, y)) = layout.local(self.cursor) {
                            if let Some(field) = yr_app::menu::setup::Setup::hit((x, y)) {
                                self.menu.setup.focus = Some(field);
                                self.menu.setup.press(field, x);
                            }
                        }
                    }
                    if self.menu.page == Page::ChooseMap {
                        if let Some(point) = layout.local(self.cursor) {
                            if let Some(index) = self.menu.maps.row_at(point) {
                                self.menu.maps.select(index);
                                self.menu.maps.list_focus = true;
                            } else if (515..535).contains(&point.0) && (127..472).contains(&point.1)
                            {
                                let (thumb_y, height) = self.menu.maps.thumb();
                                if point.1 < 150 {
                                    self.menu.maps.scroll(-1);
                                } else if point.1 >= 449 {
                                    self.menu.maps.scroll(1);
                                } else if point.1 < thumb_y {
                                    self.menu.maps.scroll(-18);
                                } else if point.1 >= thumb_y + height {
                                    self.menu.maps.scroll(18);
                                } else {
                                    self.menu.maps.drag = Some((point.1, self.menu.maps.top));
                                }
                            } else if self.menu.pressed.is_some() {
                                self.menu.maps.list_focus = false;
                            }
                        }
                        self.poll_map_chooser();
                    }
                    if let Some(index) = layout.slot(&self.menu, self.cursor) {
                        self.menu.select_slot(index);
                    }
                }
                ElementState::Released => {
                    self.menu.maps.drag = None;
                    self.menu.setup.drag = None;
                    let hit = layout.hit(&self.menu, self.cursor);
                    if let Some(button) = self.menu.pressed.take().filter(|b| Some(*b) == hit) {
                        let effect = self.menu.activate(button);
                        self.apply_menu_effect(effect);
                    }
                }
            }
        } else if button == MouseButton::Right && state == ElementState::Pressed {
            let effect = self.menu.escape();
            self.apply_menu_effect(effect);
        }
        self.reset_menu_clock();
    }
    pub(super) fn menu_key(&mut self, event: &winit::event::KeyEvent) {
        if event.state != ElementState::Pressed {
            return;
        }
        let PhysicalKey::Code(code) = event.physical_key else {
            return;
        };
        self.menu_key_code(code, event.text.as_deref(), event.repeat);
    }
    pub(super) fn menu_key_code(&mut self, code: KeyCode, text: Option<&str>, repeat: bool) {
        if self.menu.front && matches!(self.menu.page, Page::Message { .. }) {
            let delta = match code {
                KeyCode::ArrowDown => Some(1),
                KeyCode::ArrowUp => Some(-1),
                KeyCode::PageDown => Some(10),
                KeyCode::PageUp => Some(-10),
                KeyCode::Home => Some(i32::MIN),
                KeyCode::End => Some(i32::MAX),
                _ => None,
            };
            if let Some(delta) = delta {
                self.menu_scroll(delta);
                return;
            }
        }
        if self.menu.front && self.menu.page == Page::Skirmish {
            if let Some(field) = self.menu.setup.focus {
                let delta = match code {
                    KeyCode::ArrowLeft | KeyCode::ArrowDown => Some(-1),
                    KeyCode::ArrowRight | KeyCode::ArrowUp => Some(1),
                    KeyCode::Home => Some(i32::MIN),
                    KeyCode::End => Some(i32::MAX),
                    _ => None,
                };
                if let Some(delta) = delta {
                    self.menu.keyboard_focus = true;
                    self.menu.setup.adjust(field, delta);
                    self.reset_menu_clock();
                    return;
                }
            }
        }
        match code {
            KeyCode::Escape if !repeat => {
                let e = self.menu.escape();
                self.apply_menu_effect(e);
            }
            KeyCode::ArrowDown
            | KeyCode::ArrowUp
            | KeyCode::PageDown
            | KeyCode::PageUp
            | KeyCode::Home
            | KeyCode::End
                if self.menu.page == Page::ChooseMap && self.menu.maps.list_focus =>
            {
                let delta = match code {
                    KeyCode::ArrowDown => 1,
                    KeyCode::ArrowUp => -1,
                    KeyCode::PageDown => 18,
                    KeyCode::PageUp => -18,
                    KeyCode::Home => -4096,
                    _ => 4096,
                };
                self.menu.maps.move_selection(delta);
                self.poll_map_chooser();
            }
            KeyCode::Enter | KeyCode::NumpadEnter
                if !repeat && self.menu.page == Page::ChooseMap && self.menu.maps.list_focus =>
            {
                let effect = self.menu.activate(yr_app::menu::Button::UseMap);
                self.apply_menu_effect(effect);
            }
            KeyCode::ArrowDown if self.menu.page == Page::Load => self.menu.navigate_slot(1),
            KeyCode::ArrowUp if self.menu.page == Page::Load => self.menu.navigate_slot(-1),
            KeyCode::Tab => self.menu.navigate(if self.shift { -1 } else { 1 }),
            KeyCode::ArrowDown => self.menu.navigate(1),
            KeyCode::ArrowUp => self.menu.navigate(-1),
            KeyCode::Enter | KeyCode::NumpadEnter if !repeat => {
                if let Some((button, _)) = self.menu.buttons().get(self.menu.focus).copied() {
                    let e = self.menu.activate(button);
                    self.apply_menu_effect(e);
                }
            }
            KeyCode::Backspace if self.menu.page == Page::Save => {
                self.menu.name.pop();
            }
            _ => {
                if let Some(text) = text {
                    self.menu.text(text);
                }
            }
        }
        self.reset_menu_clock();
    }
    pub(super) fn menu_scroll(&mut self, delta: i32) {
        if !yr_app::frontend::scroll_message(&self.front_art, &mut self.menu, delta) {
            self.menu.scroll_by(delta);
        }
        self.reset_menu_clock();
    }
    pub(super) fn apply_menu_effect(&mut self, effect: Effect) {
        match effect {
            Effect::None => {}
            Effect::MainMenu => self.enter_front(),
            Effect::PrepareSkirmish => self.start_front_preview(),
            Effect::StartMatch => self.start_menu_load(None),
            Effect::ChooseFolder => self.choose_front_directory(),
            Effect::OpenMapChooser => self.open_map_chooser(),
            Effect::UseMap => self.use_chosen_map(),
            Effect::CancelMap => self.cancel_map_chooser(),
            Effect::Resume => {
                self.cancel_pointer_action();
                self.keys.clear();
                self.shift = false;
                self.control = false;
            }
            Effect::Quit => self.quit_requested = true,
            Effect::Restart => self.start_menu_load(None),
            Effect::RequestSave => {
                if let Some(name) = yr_app::menu::save_name(&self.menu.name) {
                    let path = self.save_directory().join(name);
                    if path.exists() {
                        self.menu.set(Page::Confirm(Intent::Save(path)));
                    } else {
                        self.save_menu_path(path);
                    }
                }
            }
            Effect::RequestLoad => {
                if let Some(path) = self
                    .menu
                    .selected
                    .and_then(|i| self.menu.slots.get(i))
                    .map(|s| s.path.clone())
                {
                    self.menu.set(Page::Confirm(Intent::Load(path)));
                }
            }
            Effect::Commit(Intent::Save(path)) => self.save_menu_path(path),
            Effect::Commit(Intent::Load(path)) => self.start_menu_load(Some(path)),
            Effect::Mute => {
                self.menu.zoom = (self.view.zoom * 100.0).round().clamp(25.0, 400.0) as u16;
                self.menu.muted = !self.menu.muted;
                if let Some(audio) = &self.audio {
                    audio.set_muted(self.menu.muted);
                }
                self.write_preferences();
            }
            Effect::Zoom(delta) => {
                self.menu.zoom =
                    (i32::from(self.menu.zoom) + i32::from(delta)).clamp(25, 400) as u16;
                self.zoom_camera(f32::from(self.menu.zoom) / 100.0);
                self.write_preferences();
            }
            Effect::EdgeScroll => {
                self.menu.edge_scroll = !self.menu.edge_scroll;
                self.camera.edge_armed = false;
                self.write_preferences();
            }
            Effect::CancelLoad => {
                if let Some(job) = &mut self.load_job {
                    job.cancelled = true;
                }
                let back = self
                    .load_job
                    .as_ref()
                    .map_or(Page::Pause, |job| job.back.clone());
                self.menu.set(back);
            }
        }
        self.reset_menu_clock();
    }
    pub(super) fn save_menu_path(&mut self, path: PathBuf) {
        self.order_feedback.clear();
        let Some(session) = &self.session else {
            return;
        };
        match yr_app::snapshot::save(session, &path) {
            Ok(()) => {
                self.save_path = path;
                if let Ok(slots) = yr_app::menu::slots(self.save_directory()) {
                    self.menu.slots = slots;
                }
                self.menu.message("Match saved".into(), Page::Pause);
            }
            Err(e) => self.menu.message(e, Page::Save),
        }
        self.cancel_pointer_action();
        self.keys.clear();
        self.reset_menu_clock();
    }
    pub(super) fn start_menu_load(&mut self, path: Option<PathBuf>) {
        if self.load_job.is_some() {
            self.menu.message(
                "Please wait for the previous load to finish".into(),
                if self.menu.front {
                    Page::Load
                } else {
                    Page::Pause
                },
            );
            return;
        }
        let directory = self.gamedir.clone();
        let map = self.map_name.clone();
        let scale = self.scale;
        let seed = self.session.as_ref().map_or(42, Session::seed);
        let options = if self.menu.front {
            yr_app::session::StartOptions {
                shared_forces: true,
                credits: Some(self.menu.setup.value(yr_app::menu::setup::Field::Credits)),
                units: Some(self.menu.setup.value(yr_app::menu::setup::Field::Units) as u16),
            }
        } else {
            self.session
                .as_ref()
                .map_or(self.menu.setup.options, |s| s.start_options)
        };
        let (sender, receiver) = std::sync::mpsc::channel();
        let back = if self.menu.front {
            if path.is_some() {
                Page::Load
            } else {
                Page::Skirmish
            }
        } else {
            Page::Pause
        };
        let spawned = std::thread::Builder::new()
            .name("openyr-match-load".into())
            .spawn(move || {
                let result = (|| {
                    let (session, map_name) = match &path {
                        Some(path) => yr_app::snapshot::load_from_menu(&directory, &map, path)?,
                        None => (
                            yr_app::session::load_with_options(&directory, &map, seed, options)?,
                            map,
                        ),
                    };
                    let map = yr_render::render_map(&directory, &map_name, scale)?;
                    Ok(Prepared::Match {
                        session: Box::new(session),
                        map,
                        map_name,
                        save_path: path,
                    })
                })();
                let _ = sender.send(result);
            });
        match spawned {
            Ok(_) => {
                self.load_job = Some(LoadJob {
                    receiver,
                    cancelled: false,
                    back,
                });
                self.open_menu(Page::Loading);
            }
            Err(e) => self.menu.message(e.to_string(), back),
        }
    }
    pub(super) fn poll_menu_load(&mut self) {
        let Some(job) = self.load_job.as_ref() else {
            return;
        };
        let result = match job.receiver.try_recv() {
            Ok(result) => result,
            Err(std::sync::mpsc::TryRecvError::Empty) => return,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                Err("The match loader stopped".into())
            }
        };
        let job = self.load_job.take().unwrap();
        let cancelled = job.cancelled;
        let back = job.back;
        if cancelled {
            return;
        }
        match result {
            Ok(Prepared::Preview(preview)) => {
                self.menu.setup.update_profile(preview.setup);
                self.front_preview = Some(preview);
                self.menu.set(Page::Skirmish);
            }
            Ok(Prepared::Match {
                session,
                map,
                map_name,
                save_path,
            }) => {
                self.menu.front = false;
                self.movie = None;
                self.front_preview = None;
                self.front_art = Default::default();
                if let Some(path) = save_path {
                    self.save_path = path;
                }
                self.menu.setup.options = session.start_options;
                self.session = Some(*session);
                self.map = Some(map);
                self.menu.map_name = map_name.clone();
                self.map_name = map_name;
                self.sprites = load_sprites(&self.gamedir, &self.map_name);
                self.animations = Default::default();
                self.effects = Default::default();
                self.selected.clear();
                self.groups = Default::default();
                self.order_feedback.clear();
                self.pending.clear();
                self.keys.clear();
                self.shift = false;
                self.control = false;
                self.cancel_pointer_action();
                self.announced = false;
                if let Some(session) = &self.session {
                    self.animations.update(&session.sim);
                    self.audio = None;
                    if self.window.is_some() {
                        match yr_app::audio::AudioScene::live(
                            &self.gamedir,
                            &self.map_name,
                            session,
                        ) {
                            Ok(audio) => {
                                audio.set_muted(self.menu.muted);
                                self.audio = Some(audio);
                            }
                            Err(e) => {
                                eprintln!("OpenYR: audio unavailable ({e}); continuing silently")
                            }
                        }
                    }
                }
                if let Some(map) = &self.map {
                    self.view.center_map(map, (WIN_W - SIDEBAR_W, WIN_H));
                }
                self.menu.set(Page::Closed);
            }
            Err(e) => self.menu.message(e, back),
        }
        self.reset_menu_clock();
    }
    pub(super) fn read_preferences(&mut self) {
        let Some(home) = std::env::var_os("HOME") else {
            return;
        };
        let path = PathBuf::from(home).join("Library/Application Support/OpenYR/interface.json");
        self.read_preferences_file(path);
    }
    fn read_preferences_file(&mut self, path: PathBuf) {
        self.preferences_path = Some(path.clone());
        let result = (|| {
            let file = match std::fs::File::open(path) {
                Ok(f) => f,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                Err(e) => return Err(e.to_string()),
            };
            let mut bytes = Vec::new();
            file.take(16 * 1024 + 1)
                .read_to_end(&mut bytes)
                .map_err(|e| e.to_string())?;
            if bytes.len() > 16 * 1024 {
                return Err("Interface preferences exceed 16 KiB".into());
            }
            let preferences: Preferences =
                serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
            if !(25..=400).contains(&preferences.zoom) {
                return Err("Invalid saved world zoom".into());
            }
            Ok(Some(preferences))
        })();
        match result {
            Ok(Some(p)) => {
                self.menu.muted = p.muted;
                self.menu.zoom = p.zoom;
                self.menu.edge_scroll = p.edge_scroll;
                self.view.zoom = f32::from(p.zoom) / 100.0;
            }
            Ok(None) => {}
            Err(e) => self.menu.message(e, Page::Pause),
        }
    }
    fn write_preferences(&mut self) {
        let Some(path) = &self.preferences_path else {
            return;
        };
        let result = (|| {
            let bytes = serde_json::to_vec(&Preferences {
                muted: self.menu.muted,
                zoom: self.menu.zoom,
                edge_scroll: self.menu.edge_scroll,
            })
            .map_err(|e| e.to_string())?;
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let temporary = path.with_extension(format!("{}.tmp", std::process::id()));
            std::fs::write(&temporary, bytes)
                .and_then(|()| std::fs::rename(&temporary, path))
                .map_err(|e| e.to_string())
        })();
        if let Err(e) = result {
            self.menu.message(e, Page::Controls);
        }
    }
}

#[cfg(test)]
mod tests;
