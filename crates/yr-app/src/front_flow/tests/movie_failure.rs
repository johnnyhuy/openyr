use super::*;
use crate::menu_flow::{LoadJob, LoadProgress, Prepared};
use yr_app::menu::{Effect, LoadStage, LoadStatus};

fn broken_movie() -> yr_app::movie::Movie {
    // Bounded synthetic BIKi container with a deliberately invalid video packet.
    let mut bytes = b"BIKi".to_vec();
    for value in [48u32, 1, 4, 1, 8, 8, 15, 1, 0, 0, 53, 56] {
        bytes.extend(value.to_le_bytes());
    }
    bytes.extend([0; 4]);
    yr_app::movie::Movie::new(bytes).unwrap()
}

fn poll_failure(app: &mut App) {
    app.movie = Some(broken_movie());
    let deadline = Instant::now() + Duration::from_secs(5);
    while !matches!(app.menu.page, Page::Message { .. }) {
        app.poll_front_presentation();
        assert!(
            Instant::now() < deadline,
            "movie worker did not report failure"
        );
        std::thread::yield_now();
    }
}

fn preview() -> yr_app::frontend::Preview {
    yr_app::frontend::Preview {
        width: 1,
        height: 1,
        rgba: vec![255, 0, 0, 255],
        authored: true,
        setup: Default::default(),
        starts: [None; 8],
        map_rect: (0, 0, 1, 1),
        local_rect: (0, 0, 1, 1),
    }
}

#[test]
fn movie_failure_recovery_ignores_late_load_results_and_progress() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        for success in [true, false] {
            let mut app = new_app(String::new(), "EB1.MMX".into(), 1);
            app.frame_size = size;
            app.menu.front = true;
            let (job, sender, progress) = LoadJob::channel(Page::SinglePlayer);
            app.load_job = Some(job);
            app.open_menu(Page::Loading);
            app.menu.loading.name = "Synthetic map".into();
            if size == (640, 480) && success {
                capture(&app, "00-loading");
            }
            poll_failure(&mut app);
            let message = app.menu.page.clone();
            assert!(matches!(message, Page::Message { ref back, .. } if **back == Page::Main));
            if size == (640, 480) && success {
                capture(&app, "01-movie-error");
            }
            progress
                .send(LoadProgress::Status(LoadStatus {
                    name: "obsolete worker".into(),
                    stage: LoadStage::Artwork,
                    ..Default::default()
                }))
                .unwrap();
            progress
                .send(LoadProgress::Preview(Box::new(preview())))
                .unwrap();
            sender
                .send(if success {
                    Ok(Prepared::Preview(preview()))
                } else {
                    Err("obsolete load error".into())
                })
                .unwrap();
            app.poll_menu_load();
            assert_eq!(
                app.menu.page, message,
                "late load result must not replace movie recovery"
            );
            assert!(
                app.load_job.is_none()
                    && app.loading_preview.is_none()
                    && app.front_preview.is_none()
            );
            assert_eq!(app.menu.loading.name, "Synthetic map");
            assert!(app.session.is_none());
            if size == (640, 480) && success {
                capture(&app, "02-late-result-ignored");
            }
            app.menu_key_code(KeyCode::Enter, None, false);
            assert_eq!(app.menu.page, Page::Main);
            if size == (640, 480) && success {
                capture(&app, "03-main-recovery");
            }
        }
    }
}

#[test]
fn movie_failure_recovery_drops_queued_retry_and_clears_input() {
    let mut app = new_app("missing-install".into(), "EB1.MMX".into(), 1);
    app.menu.front = true;
    let (job, sender, _) = LoadJob::channel(Page::SinglePlayer);
    app.load_job = Some(job);
    app.open_menu(Page::Loading);
    app.apply_menu_effect(Effect::CancelLoad);
    app.start_front_preview();
    assert_eq!(app.menu.loading.stage, LoadStage::Waiting);
    app.left_down = Some((10.0, 20.0));
    app.right_down = Some((10.0, 20.0));
    app.right_panning = true;
    app.radar_drag = true;
    app.keys.insert(KeyCode::ArrowRight);
    app.shift = true;
    app.control = true;
    app.super_key = true;
    poll_failure(&mut app);
    let message = app.menu.page.clone();
    sender.send(Err("old worker stopped".into())).unwrap();
    app.poll_menu_load();
    assert!(
        app.load_job.is_none(),
        "queued retry cannot restart behind recovery dialog"
    );
    assert_eq!(app.menu.page, message);
    assert!(app.left_down.is_none() && app.right_down.is_none());
    assert!(!app.right_panning && !app.radar_drag);
    assert!(app.keys.is_empty() && !app.shift && !app.control && !app.super_key);
    app.pointer_moved((100.0, 120.0));
    app.pointer_button(ElementState::Released, MouseButton::Left);
    app.pointer_button(ElementState::Released, MouseButton::Right);
    assert_eq!(app.menu.page, message);
}

fn capture(app: &App, name: &str) {
    let Some(dir) = std::env::var_os("OPENYR_MOVIE_RECOVERY_PROOF") else {
        return;
    };
    let dir = PathBuf::from(dir);
    std::fs::create_dir_all(&dir).unwrap();
    let mut frame = vec![0; 640 * 480 * 4];
    app.compose(&mut frame, 640, 480);
    std::fs::write(
        dir.join(format!("{name}.png")),
        yr_render::png::encode_rgba(640, 480, &frame),
    )
    .unwrap();
    std::fs::write(dir.join(format!("{name}.json")), serde_json::to_vec_pretty(&serde_json::json!({
        "page": format!("{:?}", app.menu.page), "load_job_present": app.load_job.is_some(),
        "load_cancelled": app.load_job.as_ref().map(|j| j.cancelled),
        "load_name": app.menu.loading.name, "front_preview_present": app.front_preview.is_some(),
        "loading_preview_present": app.loading_preview.is_some(), "session_present": app.session.is_some(),
        "retail_art": false, "movie_input": "56-byte synthetic malformed BIKi",
    })).unwrap()).unwrap();
}
