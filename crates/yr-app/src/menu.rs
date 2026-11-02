//! Modal in-match UI. Asset observations and limits: formats/menus.md.

mod art;
mod draw;
pub mod maps;
pub use art::Art;
pub use draw::draw;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Page {
    Closed,
    Main,
    SinglePlayer,
    Skirmish,
    ChooseMap,
    Exit,
    Pause,
    Save,
    Load,
    Controls,
    Abort,
    Confirm(Intent),
    Loading,
    Result(bool),
    Message { text: String, back: Box<Page> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intent {
    Save(PathBuf),
    Load(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Button {
    SinglePlayer,
    Internet,
    Network,
    Movies,
    Campaign,
    Skirmish,
    MainMenu,
    Start,
    ChooseMap,
    UseMap,
    GenerateMap,
    Folder,
    Controls,
    Load,
    Save,
    Delete,
    Abort,
    Resume,
    Back,
    Quit,
    Restart,
    Mute,
    ZoomOut,
    ZoomIn,
    EdgeScroll,
    Accept,
    Decline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Effect {
    None,
    Resume,
    RequestSave,
    RequestLoad,
    Restart,
    Quit,
    Mute,
    Zoom(i16),
    EdgeScroll,
    Commit(Intent),
    CancelLoad,
    MainMenu,
    PrepareSkirmish,
    StartMatch,
    ChooseFolder,
    OpenMapChooser,
    UseMap,
    CancelMap,
}

#[derive(Debug, Clone)]
pub struct Slot {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone)]
pub struct State {
    pub front: bool,
    pub page: Page,
    pub name: String,
    pub slots: Vec<Slot>,
    pub selected: Option<usize>,
    pub focus: usize,
    pub keyboard_focus: bool,
    pub scroll: usize,
    pub message_scroll: usize,
    pub pressed: Option<Button>,
    pub muted: bool,
    pub zoom: u16,
    pub edge_scroll: bool,
    pub map_name: String,
    pub maps: maps::MapList,
}

impl Default for State {
    fn default() -> Self {
        Self {
            front: false,
            page: Page::Closed,
            name: String::new(),
            slots: Vec::new(),
            selected: None,
            focus: 0,
            keyboard_focus: false,
            scroll: 0,
            message_scroll: 0,
            pressed: None,
            muted: false,
            zoom: 100,
            edge_scroll: false,
            map_name: "EB1.MMX".into(),
            maps: Default::default(),
        }
    }
}

impl State {
    pub fn visible(&self) -> bool {
        self.page != Page::Closed
    }
    pub fn set(&mut self, page: Page) {
        self.page = page;
        self.message_scroll = 0;
        self.focus = 0;
        self.keyboard_focus = false;
        if self.page == Page::SinglePlayer {
            self.focus = 1;
        }
        self.pressed = None;
    }
    pub fn message(&mut self, text: String, back: Page) {
        self.set(Page::Message {
            text,
            back: Box::new(back),
        });
    }
    pub fn buttons(&self) -> Vec<(Button, bool)> {
        use Button::*;
        match &self.page {
            Page::Closed => Vec::new(),
            Page::Main => vec![
                (SinglePlayer, true),
                (Internet, false),
                (Network, false),
                (Movies, false),
                (Controls, true),
                (Quit, true),
            ],
            Page::SinglePlayer => vec![
                (Campaign, false),
                (Load, true),
                (Skirmish, true),
                (MainMenu, true),
            ],
            Page::Skirmish => vec![
                (Start, true),
                (ChooseMap, !self.maps.entries.is_empty()),
                (Back, true),
            ],
            Page::ChooseMap => vec![
                (
                    UseMap,
                    self.maps.ready == Some(self.maps.selected)
                        && self.maps.selected < self.maps.entries.len(),
                ),
                (GenerateMap, false),
                (Back, true),
            ],
            Page::Exit => vec![(Accept, true), (Decline, true)],
            Page::Pause => vec![
                (Controls, true),
                (Load, true),
                (Save, true),
                (Delete, false),
                (Abort, true),
                (Resume, true),
            ],
            Page::Save => vec![(Save, save_name(&self.name).is_some()), (Back, true)],
            Page::Load => vec![
                (Load, self.selected.is_some_and(|i| i < self.slots.len())),
                (Back, true),
            ],
            Page::Controls => {
                let mut controls = vec![
                    (Mute, true),
                    (ZoomOut, self.zoom > 25),
                    (ZoomIn, self.zoom < 400),
                    (EdgeScroll, true),
                ];
                if self.front {
                    controls.push((Folder, true));
                }
                controls.push((Back, true));
                controls
            }
            Page::Abort => vec![
                (Quit, true),
                (Restart, true),
                (MainMenu, true),
                (Resume, true),
            ],
            Page::Result(_) => vec![(Restart, true), (MainMenu, true), (Quit, true)],
            Page::Confirm(_) => vec![(Accept, true), (Decline, true)],
            Page::Loading => vec![(Back, true)],
            Page::Message { .. } => vec![(Back, true)],
        }
    }
    pub fn navigate(&mut self, delta: i32) {
        self.keyboard_focus = true;
        let buttons = self.buttons();
        if self.page == Page::ChooseMap {
            let mut stops = vec![None];
            stops.extend(
                buttons
                    .iter()
                    .enumerate()
                    .filter_map(|(i, (_, enabled))| enabled.then_some(Some(i))),
            );
            let current = if self.maps.list_focus {
                None
            } else {
                Some(self.focus)
            };
            let at = stops.iter().position(|stop| *stop == current).unwrap_or(0);
            let next = (at as i64 + i64::from(delta)).rem_euclid(stops.len() as i64) as usize;
            self.maps.list_focus = stops[next].is_none();
            if let Some(index) = stops[next] {
                self.focus = index;
            }
            return;
        }
        let n = buttons.len();
        if n == 0 {
            return;
        }
        for _ in 0..n {
            self.focus = (self.focus as i32 + delta).rem_euclid(n as i32) as usize;
            if buttons[self.focus].1 {
                break;
            }
        }
    }
    pub fn activate(&mut self, button: Button) -> Effect {
        if !self
            .buttons()
            .iter()
            .any(|&(id, enabled)| id == button && enabled)
        {
            return Effect::None;
        }
        use Button::*;
        let effect = match (&self.page, button) {
            (Page::Main, SinglePlayer) => {
                self.set(Page::SinglePlayer);
                Effect::None
            }
            (Page::Main, Controls) => {
                self.set(Page::Controls);
                Effect::None
            }
            (Page::Main, Quit) => {
                self.set(Page::Exit);
                Effect::None
            }
            (Page::Exit, Accept) => Effect::Quit,
            (Page::Exit, Decline) => {
                self.set(Page::Main);
                Effect::None
            }
            (Page::SinglePlayer, Skirmish) => Effect::PrepareSkirmish,
            (Page::SinglePlayer, Load) => {
                self.set(Page::Load);
                Effect::None
            }
            (Page::SinglePlayer, MainMenu) => {
                self.set(Page::Main);
                Effect::None
            }
            (Page::Skirmish, Start) => Effect::StartMatch,
            (Page::Skirmish, ChooseMap) => Effect::OpenMapChooser,
            (Page::ChooseMap, UseMap) => Effect::UseMap,
            (Page::ChooseMap, Back) => Effect::CancelMap,
            (Page::Skirmish, Back) => {
                self.set(Page::SinglePlayer);
                Effect::None
            }
            (Page::Controls, Folder) => Effect::ChooseFolder,
            (Page::Abort | Page::Result(_), MainMenu) => Effect::MainMenu,
            (Page::Pause, Controls) => {
                self.set(Page::Controls);
                Effect::None
            }
            (Page::Pause, Load) => {
                self.set(Page::Load);
                Effect::None
            }
            (Page::Pause, Save) => {
                self.set(Page::Save);
                Effect::None
            }
            (Page::Pause, Abort) => {
                self.set(Page::Abort);
                Effect::None
            }
            (Page::Save, Save) => Effect::RequestSave,
            (Page::Load, Load) => Effect::RequestLoad,
            (Page::Controls, Mute) => Effect::Mute,
            (Page::Controls, ZoomOut) => Effect::Zoom(-25),
            (Page::Controls, ZoomIn) => Effect::Zoom(25),
            (Page::Controls, EdgeScroll) => Effect::EdgeScroll,
            (Page::Abort | Page::Result(_), Quit) => Effect::Quit,
            (Page::Abort | Page::Result(_), Restart) => Effect::Restart,
            (Page::Pause | Page::Abort, Resume) => {
                self.set(Page::Closed);
                Effect::Resume
            }
            (Page::Confirm(intent), Accept) => Effect::Commit(intent.clone()),
            (Page::Confirm(intent), Decline) => {
                let page = match intent {
                    Intent::Save(_) => Page::Save,
                    Intent::Load(_) => Page::Load,
                };
                self.set(page);
                Effect::None
            }
            (Page::Message { back, .. }, Back) => {
                let back = *back.clone();
                self.set(back);
                Effect::None
            }
            (Page::Loading, Back) => Effect::CancelLoad,
            (_, Back) => {
                self.set(if self.front {
                    if self.page == Page::Load {
                        Page::SinglePlayer
                    } else {
                        Page::Main
                    }
                } else {
                    Page::Pause
                });
                Effect::None
            }
            _ => Effect::None,
        };
        self.pressed = None;
        effect
    }
    pub fn escape(&mut self) -> Effect {
        match self.page {
            Page::Closed | Page::Result(_) => Effect::None,
            Page::Main => self.activate(Button::Quit),
            Page::Exit => self.activate(Button::Decline),
            Page::SinglePlayer => self.activate(Button::MainMenu),
            Page::Pause | Page::Abort => self.activate(Button::Resume),
            Page::Confirm(_) => self.activate(Button::Decline),
            _ => self.activate(Button::Back),
        }
    }
    pub fn text(&mut self, text: &str) {
        if self.page != Page::Save {
            return;
        }
        for ch in text
            .chars()
            .filter(|ch| !ch.is_control() && !matches!(ch, '/' | '\\' | ':'))
        {
            if self.name.chars().count() >= 48 {
                break;
            }
            self.name.push(ch);
        }
    }
    pub fn select_slot(&mut self, index: usize) {
        if let Some(slot) = self.slots.get(index) {
            self.selected = Some(index);
            self.focus = 0;
            if self.page == Page::Save {
                self.name = slot.name.clone();
            }
        }
    }
    pub fn navigate_slot(&mut self, delta: i32) {
        if self.slots.is_empty() {
            return;
        }
        let index = self.selected.map_or(0, |i| {
            (i as i32 + delta).clamp(0, self.slots.len() as i32 - 1) as usize
        });
        self.select_slot(index);
        if index < self.scroll {
            self.scroll = index;
        }
        if index >= self.scroll + 10 {
            self.scroll = index - 9;
        }
    }
    pub fn scroll_by(&mut self, delta: i32) {
        if self.page == Page::ChooseMap {
            self.maps.scroll(delta);
            return;
        }
        self.scroll = (self.scroll as i32 + delta)
            .clamp(0, self.slots.len().saturating_sub(10) as i32) as usize;
    }
}

/// A single filename component, never a path. Keep the generated save beside
/// the app's selected save path; atomic replacement remains in snapshot.rs.
pub fn save_name(name: &str) -> Option<String> {
    if name.chars().any(char::is_control) {
        return None;
    }
    let name = name.trim();
    if name.is_empty()
        || name.starts_with('.')
        || name.chars().count() > 48
        || name
            .chars()
            .any(|ch| ch.is_control() || matches!(ch, '/' | '\\' | ':'))
    {
        return None;
    }
    Some(format!("{name}.oyrsave"))
}

/// Stable bounded listing. Symlinks and unrelated files are not menu slots.
pub fn slots(dir: &std::path::Path) -> Result<Vec<Slot>, String> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e.to_string()),
    };
    let mut slots = Vec::new();
    for entry in entries.take(4096) {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().is_ok_and(|t| t.is_file())
            || entry.path().extension().is_none_or(|e| e != "oyrsave")
        {
            continue;
        }
        let name = entry
            .path()
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .chars()
            .take(48)
            .collect();
        slots.push(Slot {
            name,
            path: entry.path(),
        });
    }
    slots.sort_by(|a, b| a.path.cmp(&b.path));
    slots.truncate(64);
    Ok(slots)
}

#[derive(Debug, Clone, Copy)]
pub struct Layout {
    pub width: u32,
    pub height: u32,
    pub scale: f64,
    pub origin: (f64, f64),
    pub front: bool,
}
impl Layout {
    pub fn new(width: u32, height: u32) -> Self {
        let (w, h) = if width >= 1024 && height >= 768 {
            (1024, 768)
        } else if width >= 800 && height >= 600 {
            (800, 600)
        } else {
            (640, 480)
        };
        let scale = (f64::from(width) / f64::from(w))
            .min(f64::from(height) / f64::from(h))
            .min(1.0);
        Self {
            front: false,
            width: w,
            height: h,
            scale,
            origin: (
                (f64::from(width) - f64::from(w) * scale) / 2.0,
                (f64::from(height) - f64::from(h) * scale) / 2.0,
            ),
        }
    }
    pub fn for_state(state: &State, width: u32, height: u32) -> Self {
        if !state.front {
            return Self::new(width, height);
        }
        let scale = (f64::from(width) / 800.0)
            .min(f64::from(height) / 600.0)
            .min(1.0);
        Self {
            width: 800,
            height: 600,
            scale,
            origin: (
                (f64::from(width) - 800.0 * scale) / 2.0,
                (f64::from(height) - 600.0 * scale) / 2.0,
            ),
            front: true,
        }
    }
    pub fn local(self, pos: (f64, f64)) -> Option<(i32, i32)> {
        if self.scale <= 0.0 || !pos.0.is_finite() || !pos.1.is_finite() {
            return None;
        }
        let p = (
            (pos.0 - self.origin.0) / self.scale,
            (pos.1 - self.origin.1) / self.scale,
        );
        (p.0 >= 0.0 && p.1 >= 0.0 && p.0 < f64::from(self.width) && p.1 < f64::from(self.height))
            .then_some((p.0.floor() as i32, p.1.floor() as i32))
    }
    pub fn button(self, index: usize, button: Button) -> (i32, i32, i32, i32) {
        if self.front {
            let y = if matches!(button, Button::MainMenu | Button::Back | Button::Quit) {
                535
            } else if matches!(button, Button::Start | Button::GenerateMap) {
                241
            } else if button == Button::ChooseMap {
                283
            } else {
                199 + index as i32 * 42
            };
            return (644, y, 156, 42);
        }
        let y = if matches!(button, Button::Resume | Button::Back) {
            self.height as i32 - 65
        } else {
            228 + index as i32 * 25
        };
        (self.width as i32 - 168 + 23, y, 125, 25)
    }
    pub fn hit(self, state: &State, pos: (f64, f64)) -> Option<Button> {
        let p = self.local(pos)?;
        state
            .buttons()
            .iter()
            .enumerate()
            .find_map(|(i, &(id, enabled))| {
                let (x, y, w, h) = self.button(i, id);
                (enabled && p.0 >= x && p.0 < x + w && p.1 >= y && p.1 < y + h).then_some(id)
            })
    }
    pub fn slot(self, state: &State, pos: (f64, f64)) -> Option<usize> {
        if !matches!(state.page, Page::Save | Page::Load) {
            return None;
        }
        let (x, y) = self.local(pos)?;
        if x < 40 || x >= self.width as i32 - 208 || !(150..390).contains(&y) {
            return None;
        }
        let index = state.scroll + ((y - 150) / 24) as usize;
        (index < state.slots.len()).then_some(index)
    }
}

#[cfg(test)]
mod tests;
