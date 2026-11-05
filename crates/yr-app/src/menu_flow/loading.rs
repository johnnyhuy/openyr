use super::*;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc, Arc,
};
use yr_app::menu::{LoadStage, LoadStatus};

pub(crate) enum LoadKind {
    Preview,
    Match(Option<PathBuf>),
}

pub(super) struct LoadRequest {
    kind: LoadKind,
    directory: String,
    map: String,
    scale: u32,
    seed: u64,
    options: yr_app::session::StartOptions,
    config: yr_app::match_config::MatchConfig,
    back: Page,
    status: LoadStatus,
}

impl LoadJob {
    pub(crate) fn channel(
        back: Page,
    ) -> (
        Self,
        mpsc::Sender<Result<Prepared, String>>,
        mpsc::Sender<LoadProgress>,
    ) {
        let (sender, receiver) = mpsc::channel();
        let (progress_sender, progress) = mpsc::channel();
        (
            Self {
                receiver,
                cancelled: false,
                back,
                progress,
                stop: Arc::new(AtomicBool::new(false)),
                next: None,
            },
            sender,
            progress_sender,
        )
    }

    pub(crate) fn cancel(&mut self) {
        self.cancelled = true;
        self.stop.store(true, Ordering::Relaxed);
        self.next = None;
    }
}

impl App {
    pub(crate) fn start_load(&mut self, kind: LoadKind) {
        // A cancelled worker keeps its slot until it stops. Only the latest
        // requested replacement waits behind it; workers never accumulate.
        if self.load_job.as_ref().is_some_and(|job| !job.cancelled) {
            return;
        }
        let back = match &kind {
            LoadKind::Preview => Page::SinglePlayer,
            LoadKind::Match(_) if !self.menu.front => Page::Pause,
            LoadKind::Match(Some(_)) => Page::Load,
            LoadKind::Match(None) => Page::Skirmish,
        };
        let name = match &kind {
            LoadKind::Match(Some(path)) => path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned(),
            _ => self
                .menu
                .maps
                .entries
                .iter()
                .find(|map| map.asset.eq_ignore_ascii_case(&self.map_name))
                .map_or_else(|| self.map_name.clone(), |map| map.title.clone()),
        };
        let stage = match kind {
            LoadKind::Preview => LoadStage::Preview,
            LoadKind::Match(_) => LoadStage::Session,
        };
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
        let battle = matches!(kind, LoadKind::Match(_));
        let map_name = if matches!(kind, LoadKind::Match(Some(_))) {
            String::new()
        } else {
            self.map_name.clone()
        };
        let request = LoadRequest {
            kind,
            directory: self.gamedir.clone(),
            map: self.map_name.clone(),
            scale: self.scale,
            seed: self.session.as_ref().map_or(42, Session::seed),
            options,
            config: if self.menu.front {
                self.menu.setup.config.clone()
            } else {
                self.session
                    .as_ref()
                    .map_or_else(Default::default, |session| session.match_config.clone())
            },
            back: back.clone(),
            status: LoadStatus {
                name,
                stage,
                map_name,
                battle,
                config: if self.menu.front {
                    self.menu.setup.config.clone()
                } else {
                    self.session
                        .as_ref()
                        .map_or_else(Default::default, |session| session.match_config.clone())
                },
            },
        };
        self.loading_preview = None;
        self.menu.loading = request.status.clone();
        if let Some(job) = &mut self.load_job {
            job.back = back;
            job.next = Some(request);
            self.menu.loading.stage = LoadStage::Waiting;
            self.open_menu(Page::Loading);
        } else {
            self.spawn_load(request);
        }
        self.reset_menu_clock();
    }

    pub(super) fn spawn_load(&mut self, request: LoadRequest) {
        let back = request.back.clone();
        let (job, sender, progress) = LoadJob::channel(back.clone());
        let stop = job.stop.clone();
        self.menu.loading = request.status.clone();
        let spawned = std::thread::Builder::new()
            .name("openyr-menu-load".into())
            .spawn(move || {
                let result = request.prepare(&progress, &stop);
                let _ = sender.send(result);
            });
        match spawned {
            Ok(_) => {
                self.load_job = Some(job);
                self.open_menu(Page::Loading);
            }
            Err(error) => self.menu.message(error.to_string(), back),
        }
    }
}

impl LoadRequest {
    fn prepare(
        self,
        progress: &mpsc::Sender<LoadProgress>,
        stop: &AtomicBool,
    ) -> Result<Prepared, String> {
        let report = |stage, map_name: &str, config: &yr_app::match_config::MatchConfig| {
            if stop.load(Ordering::Relaxed) {
                return Err("Loading cancelled".to_string());
            }
            let _ = progress.send(LoadProgress::Status(LoadStatus {
                name: self.status.name.clone(),
                stage,
                map_name: map_name.to_owned(),
                battle: self.status.battle,
                config: config.clone(),
            }));
            Ok(())
        };
        match self.kind {
            LoadKind::Preview => {
                report(LoadStage::Preview, &self.map, &self.config)?;
                yr_app::frontend::load_preview(&self.directory, &self.map, self.scale)
                    .map(Prepared::Preview)
            }
            LoadKind::Match(path) => {
                report(LoadStage::Session, &self.status.map_name, &self.config)?;
                let (session, map_name) = match &path {
                    Some(path) => yr_app::snapshot::load_from_menu_with_metadata(
                        &self.directory,
                        &self.map,
                        path,
                        |map, config| report(LoadStage::Session, map, config),
                    )?,
                    None => (
                        yr_app::session::load_with_config(
                            &self.directory,
                            &self.map,
                            self.seed,
                            self.options,
                            self.config.clone(),
                        )?,
                        self.map,
                    ),
                };
                report(LoadStage::Terrain, &map_name, &session.match_config)?;
                if let Ok(preview) =
                    yr_app::frontend::load_preview(&self.directory, &map_name, self.scale)
                {
                    let _ = progress.send(LoadProgress::Preview(Box::new(preview)));
                }
                report(LoadStage::Terrain, &map_name, &session.match_config)?;
                let map = yr_render::render_map(&self.directory, &map_name, self.scale)?;
                report(LoadStage::Artwork, &map_name, &session.match_config)?;
                let sprites = load_sprites(&self.directory, &map_name).map(Box::new);
                Ok(Prepared::Match {
                    session: Box::new(session),
                    map,
                    map_name,
                    save_path: path,
                    sprites,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cancelling_a_queued_retry_drops_it_and_ignores_stale_progress() {
        let mut app = new_app("missing-install".into(), "EB1.MMX".into(), 1);
        app.menu.front = true;
        let (job, sender, progress) = LoadJob::channel(Page::SinglePlayer);
        app.load_job = Some(job);
        app.open_menu(Page::Loading);
        app.apply_menu_effect(Effect::CancelLoad);
        assert_eq!(app.menu.page, Page::SinglePlayer);
        assert!(app.load_job.as_ref().unwrap().stop.load(Ordering::Relaxed));

        app.start_load(LoadKind::Preview);
        assert_eq!(app.menu.page, Page::Loading);
        assert_eq!(app.menu.loading.stage, LoadStage::Waiting);
        assert!(app.load_job.as_ref().unwrap().next.is_some());
        progress
            .send(LoadProgress::Status(LoadStatus {
                name: "stale map".into(),
                stage: LoadStage::Artwork,
                ..Default::default()
            }))
            .unwrap();
        app.poll_menu_load();
        assert_eq!(app.menu.loading.name, "EB1.MMX");
        assert_eq!(app.menu.loading.stage, LoadStage::Waiting);

        app.apply_menu_effect(Effect::CancelLoad);
        assert!(app.load_job.as_ref().unwrap().next.is_none());
        sender.send(Err("cancelled worker failure".into())).unwrap();
        app.poll_menu_load();
        assert!(app.load_job.is_none() && app.session.is_none());
        assert_eq!(app.menu.page, Page::SinglePlayer);
    }

    #[test]
    fn retry_keeps_its_map_options_and_recovery_route_until_worker_finishes() {
        let mut app = new_app("missing-install".into(), "EB1.MMX".into(), 1);
        app.menu.front = true;
        let (job, sender, _) = LoadJob::channel(Page::SinglePlayer);
        app.load_job = Some(job);
        app.open_menu(Page::Loading);
        app.apply_menu_effect(Effect::CancelLoad);
        app.menu
            .setup
            .set(yr_app::menu::setup::Field::Credits, 7000);
        app.start_load(LoadKind::Match(None));
        let job = app.load_job.as_ref().unwrap();
        let request = job.next.as_ref().unwrap();
        assert_eq!(request.map, "EB1.MMX");
        assert_eq!(request.options.credits, Some(7000));
        assert_eq!(job.back, Page::Skirmish);
        // A later value must not change the queued request's captured setup.
        app.map_name = "another.map".into();
        app.menu
            .setup
            .set(yr_app::menu::setup::Field::Credits, 9000);
        sender.send(Err("old load failed".into())).unwrap();
        app.poll_menu_load();
        assert_eq!(app.menu.loading.name, "EB1.MMX");
        let deadline = Instant::now() + Duration::from_secs(10);
        while app.load_job.is_some() {
            app.poll_menu_load();
            assert!(Instant::now() < deadline);
            std::thread::yield_now();
        }
        assert!(matches!(
            app.menu.page,
            Page::Message { ref back, .. } if **back == Page::Skirmish
        ));
        assert!(app.session.is_none());
    }

    #[test]
    fn saved_map_preview_replaces_the_old_thumbnail_only_for_the_active_load() {
        fn preview(color: [u8; 4]) -> Box<yr_app::frontend::Preview> {
            Box::new(yr_app::frontend::Preview {
                width: 1,
                height: 1,
                rgba: color.to_vec(),
                authored: true,
                setup: Default::default(),
                starts: [None; 8],
                map_rect: (0, 0, 1, 1),
                local_rect: (0, 0, 1, 1),
            })
        }
        let mut app = new_app(String::new(), "EB1.MMX".into(), 1);
        app.menu.front = true;
        app.front_preview = Some(*preview([255, 0, 0, 255]));
        let (job, sender, progress) = LoadJob::channel(Page::Load);
        app.load_job = Some(job);
        app.menu.loading = LoadStatus {
            name: "Alamo save".into(),
            battle: true,
            ..Default::default()
        };
        app.open_menu(Page::Loading);
        let mut frame = vec![0; (WIN_W * WIN_H * 4) as usize];
        let at = (((100 + 450) * WIN_W + 240 + 608) * 4) as usize;
        app.compose(&mut frame, WIN_W, WIN_H);
        assert_eq!(
            &frame[at..at + 4],
            &[0, 0, 0, 255],
            "no stale EB1 preview before saved map is known"
        );
        progress
            .send(LoadProgress::Status(LoadStatus {
                name: "Alamo save".into(),
                map_name: "XMP29U2.MAP".into(),
                battle: true,
                stage: LoadStage::Terrain,
                config: Default::default(),
            }))
            .unwrap();
        progress
            .send(LoadProgress::Preview(preview([0, 0, 255, 255])))
            .unwrap();
        app.poll_menu_load();
        assert_eq!(app.menu.loading.map_name, "XMP29U2.MAP");
        app.compose(&mut frame, WIN_W, WIN_H);
        assert_eq!(&frame[at..at + 4], &[0, 0, 248, 255]);
        assert_eq!(
            app.map_name, "EB1.MMX",
            "loading has not committed a replacement"
        );
        app.apply_menu_effect(Effect::CancelLoad);
        progress
            .send(LoadProgress::Preview(preview([0, 255, 0, 255])))
            .unwrap();
        sender.send(Err("cancelled".into())).unwrap();
        app.poll_menu_load();
        assert!(app.loading_preview.is_none());
        assert_eq!(app.menu.page, Page::Load);
        assert_eq!(app.map_name, "EB1.MMX");
    }
}
