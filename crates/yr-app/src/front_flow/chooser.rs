use super::*;

pub(crate) struct Job {
    index: usize,
    epoch: u64,
    asset: String,
    receiver: std::sync::mpsc::Receiver<Result<yr_app::frontend::Preview, String>>,
}

impl App {
    pub(crate) fn open_map_chooser(&mut self) {
        if self.menu.maps.entries.is_empty() {
            self.menu
                .message("No Battle maps were found".into(), Page::Skirmish);
            return;
        }
        self.chooser_epoch = self.chooser_epoch.wrapping_add(1);
        self.chooser_preview = None;
        self.menu.maps.open(&self.map_name);
        self.open_menu(Page::ChooseMap);
        self.poll_map_chooser();
    }

    pub(crate) fn cancel_map_chooser(&mut self) {
        self.chooser_epoch = self.chooser_epoch.wrapping_add(1);
        self.chooser_preview = None;
        self.menu.maps.drag = None;
        self.open_menu(Page::Skirmish);
    }

    pub(crate) fn use_chosen_map(&mut self) {
        if self.menu.maps.ready != Some(self.menu.maps.selected) {
            return;
        }
        let Some(entry) = self.menu.maps.entries.get(self.menu.maps.selected) else {
            return;
        };
        let Some(preview) = self.chooser_preview.take() else {
            return;
        };
        self.map_name = entry.asset.clone();
        self.menu.map_name = self.map_name.clone();
        self.menu.setup.update_profile(preview.setup);
        self.menu.setup.set_starts(&preview.starts);
        self.front_preview = Some(preview);
        self.chooser_epoch = self.chooser_epoch.wrapping_add(1);
        self.menu.maps.drag = None;
        self.open_menu(Page::Skirmish);
    }

    /// One preview worker at a time. Rapid navigation replaces the pending
    /// selection, and an obsolete result cannot replace the current preview.
    pub(crate) fn poll_map_chooser(&mut self) {
        if let Some(job) = &self.chooser_job {
            let result = match job.receiver.try_recv() {
                Ok(result) => result,
                Err(std::sync::mpsc::TryRecvError::Empty) => return,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    Err("Map preview worker stopped".into())
                }
            };
            let job = self.chooser_job.take().unwrap();
            if self.menu.front
                && self.menu.page == Page::ChooseMap
                && job.epoch == self.chooser_epoch
                && job.index == self.menu.maps.selected
                && self
                    .menu
                    .maps
                    .entries
                    .get(job.index)
                    .is_some_and(|m| m.asset == job.asset)
            {
                match result {
                    Ok(preview) => {
                        self.chooser_preview = Some(preview);
                        self.menu.maps.ready = Some(job.index);
                    }
                    Err(error) => self.menu.maps.error = Some(error),
                }
                self.view.dirty = true;
            }
        }
        if !self.menu.front
            || self.menu.page != Page::ChooseMap
            || self.menu.maps.ready == Some(self.menu.maps.selected)
            || self.menu.maps.error.is_some()
        {
            return;
        }
        let Some(entry) = self.menu.maps.entries.get(self.menu.maps.selected) else {
            return;
        };
        self.chooser_preview = None;
        let asset = entry.asset.clone();
        let worker_asset = asset.clone();
        let directory = self.gamedir.clone();
        let scale = self.scale;
        let (sender, receiver) = std::sync::mpsc::channel();
        match std::thread::Builder::new()
            .name("openyr-chooser-preview".into())
            .spawn(move || {
                let result = yr_app::frontend::load_preview(&directory, &worker_asset, scale);
                let _ = sender.send(result);
            }) {
            Ok(_) => {
                self.chooser_job = Some(Job {
                    index: self.menu.maps.selected,
                    epoch: self.chooser_epoch,
                    asset,
                    receiver,
                })
            }
            Err(error) => self.menu.maps.error = Some(error.to_string()),
        }
    }
}
