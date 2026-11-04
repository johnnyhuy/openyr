use super::*;
use yr_app::menu::Page;
pub(super) mod chooser;
impl App {
    pub(super) fn poll_front_presentation(&mut self) {
        self.poll_map_chooser();
        if !self.menu.front {
            return;
        }
        let phase = (self.front_epoch.elapsed().as_nanos() * 15 / 1_000_000_000) as u64;
        if phase != self.front_phase {
            self.front_phase = phase;
            self.view.dirty = true;
        }
        if self.movie.as_mut().is_some_and(|v| v.poll()) {
            self.view.dirty = true;
        }
        if let Some(error) = self.movie.as_mut().and_then(|v| v.error.take()) {
            self.menu.message(error, Page::Main);
        }
    }
    pub(super) fn enter_front(&mut self) {
        if let Some(job) = &mut self.load_job {
            job.cancel();
        }
        self.report_audio();
        self.audio = None;
        self.session = None;
        self.map = None;
        self.sprites = None;
        self.animations = Default::default();
        self.effects = Default::default();
        self.selected.clear();
        self.groups = Default::default();
        self.pending.clear();
        self.front_preview = None;
        self.loading_preview = None;
        self.chooser_preview = None;
        self.chooser_epoch = self.chooser_epoch.wrapping_add(1);
        self.menu.map_name = self.map_name.clone();
        self.menu.setup.options.shared_forces = true;
        self.menu.setup.touched = [false; 2];
        self.menu.front = true;
        self.movie = None;
        self.front_art = Default::default();
        self.front_epoch = Instant::now();
        self.front_phase = 0;
        self.open_menu(Page::Main);
        match self.load_front_art() {
            Ok(()) => {}
            Err(e) => self.menu.message(e, Page::Main),
        }
        self.start_front_audio();
        if let Some(w) = &self.window {
            w.set_title("OpenYR");
            w.request_redraw();
        }
        self.update_pointer_visibility();
        self.reset_menu_clock();
        println!("OpenYR: native main menu ready; match not started");
    }
    fn start_front_audio(&mut self) {
        self.audio = None;
        if self.window.is_some() {
            match yr_app::audio::AudioScene::front_live(&self.gamedir) {
                Ok(audio) => {
                    audio.set_muted(self.menu.muted);
                    self.audio = Some(audio);
                }
                Err(e) => eprintln!("OpenYR: menu audio unavailable ({e}); continuing silently"),
            }
        }
    }
    fn load_front_art(&mut self) -> Result<(), String> {
        let fs = yr_assets::AssetFs::open(std::path::Path::new(&self.gamedir))
            .map_err(|e| e.to_string())?;
        self.front_art = yr_app::frontend::Art::load(&fs);
        self.loading_art = yr_app::loading::Art::load(&fs);
        self.menu_art = yr_app::menu::Art::load(&fs);
        self.menu.maps = yr_app::menu::maps::MapList {
            entries: yr_app::menu::maps::catalog(&fs),
            ..Default::default()
        };
        self.chooser_epoch = self.chooser_epoch.wrapping_add(1);
        self.chooser_preview = None;
        let bytes = fs
            .read("RA2TS_L.BIK")
            .ok_or("Menu movie RA2TS_L.BIK is missing")?;
        self.movie = Some(yr_app::movie::Movie::new(bytes.to_vec())?);
        Ok(())
    }
    pub(super) fn start_front_preview(&mut self) {
        if self.front_preview.is_some() {
            self.open_menu(Page::Skirmish);
            return;
        }
        self.start_load(menu_flow::LoadKind::Preview);
    }
    pub(super) fn choose_front_directory(&mut self) {
        self.cancel_pointer_action();
        self.keys.clear();
        self.shift = false;
        self.control = false;
        self.pointer.inside = false;
        if let Some(w) = &self.window {
            w.set_cursor_visible(true);
        }
        match launcher::choose_and_remember_install() {
            Ok(Some(directory)) => {
                self.gamedir = directory;
                self.movie = None;
                self.front_preview = None;
                if let Err(e) = self.load_front_art() {
                    self.menu.message(e, Page::Controls);
                } else {
                    self.menu
                        .message("Game data folder updated".into(), Page::Controls);
                }
                self.start_front_audio();
            }
            Ok(None) => {}
            Err(e) => self.menu.message(e, Page::Controls),
        }
        self.reset_menu_clock();
    }
}

#[cfg(test)]
mod tests;
