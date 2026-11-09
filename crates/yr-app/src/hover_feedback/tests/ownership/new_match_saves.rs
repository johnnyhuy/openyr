use super::*;
use crate::menu_flow::{LoadJob, Prepared};
use yr_app::menu::Intent;

fn prepared(fresh: Session, path: Option<PathBuf>) -> Prepared {
    Prepared::Match {
        session: Box::new(fresh),
        map: yr_render::RenderedMap {
            width: 1600,
            height: 1200,
            rgba: Vec::new(),
            origin_x: -600,
            origin_y: -200,
            scale: 1,
            summary: "Authored save-target canvas".into(),
        },
        map_name: "NEW.MAP".into(),
        save_path: path,
        sprites: None,
    }
}

fn queued(app: &mut App, back: Page) -> std::sync::mpsc::Sender<Result<Prepared, String>> {
    let (job, sender, _) = LoadJob::channel(back);
    app.load_job = Some(job);
    app.open_menu(Page::Loading);
    sender
}

fn complete(app: &mut App, result: Result<Prepared, String>) {
    queued(app, Page::Skirmish).send(result).ok().unwrap();
    app.poll_menu_load();
}

#[test]
fn new_match_save_target_preserves_named_file_and_writes_fresh_quicksave() {
    for size in [(640, 480), (800, 600), (1280, 800)] {
        let (mut app, _, _) = scene(size);
        let directory = PreferenceDirectory::new();
        let named = directory.0.join("Earlier campaign.oyrsave");
        let quick = directory.0.join("quicksave.oyrsave");
        yr_app::snapshot::save(app.session.as_ref().unwrap(), &named).unwrap();
        let original = std::fs::read(&named).unwrap();
        std::fs::write(&quick, b"previous quicksave fixture").unwrap();
        app.save_path = named.clone();
        let capture = size == (640, 480);
        if capture {
            proof(&mut app, "00-earlier-named-match", &named, &original);
        }
        app.apply_menu_effect(yr_app::menu::Effect::MainMenu);
        assert!(app.session.is_none() && app.menu.front);
        assert_eq!(
            app.save_path, named,
            "returning to Main alone keeps the target"
        );
        let fresh = session();
        let saved = yr_app::snapshot::encode(&fresh).unwrap();
        assert_ne!(saved, original);
        // Exercise the ordinary commit handler with a controlled worker result.
        // No retail loader or save reconstruction is claimed by this fixture.
        complete(&mut app, Ok(prepared(fresh, None)));
        assert_eq!(app.menu.page, Page::Closed);
        assert!(!app.menu.front);
        assert_eq!(app.save_path, quick);
        assert_eq!(std::fs::read(&named).unwrap(), original);
        assert_eq!(
            std::fs::read(&quick).unwrap(),
            b"previous quicksave fixture"
        );
        assert_eq!(
            yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
            saved
        );
        if capture {
            proof(&mut app, "01-fresh-match-committed", &named, &original);
        }
        press_key(&mut app, KeyCode::F5);
        assert!(matches!(app.menu.page, Page::Message { .. }));
        assert_eq!(std::fs::read(&named).unwrap(), original);
        assert_eq!(std::fs::read(&quick).unwrap(), saved);
        assert_eq!(app.save_path, quick);
        if capture {
            proof(&mut app, "02-f5-fresh-save", &named, &original);
        }
        press_key(&mut app, KeyCode::Escape);
        assert_eq!(app.menu.page, Page::Pause);
        press_key(&mut app, KeyCode::Escape);
        assert_eq!(app.menu.page, Page::Closed);
        press_key(&mut app, KeyCode::F9);
        assert_eq!(app.menu.page, Page::Confirm(Intent::Load(quick)));
        if capture {
            proof(&mut app, "03-f9-quicksave-target", &named, &original);
        }
        press_key(&mut app, KeyCode::Escape);
        assert_eq!(app.menu.page, Page::Load);
        assert_eq!(std::fs::read(&named).unwrap(), original);
        assert_eq!(
            yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
            saved
        );
        assert!(app.pending.is_empty());
    }
    println!("Three layouts: successful fresh front commit selects the current folder's quicksave; real F5 preserves the earlier named file and writes exact fresh bytes; real F9 confirms only quicksave.");
}

#[test]
fn new_match_save_target_keeps_loaded_and_in_match_restart_policies() {
    for front in [false, true] {
        let (mut app, _, _) = scene((640, 480));
        let directory = PreferenceDirectory::new();
        let named = directory.0.join("Restored checkpoint.oyrsave");
        let restore = session();
        let bytes = yr_app::snapshot::encode(&restore).unwrap();
        std::fs::write(&named, &bytes).unwrap();
        app.save_path = directory.0.join("quicksave.oyrsave");
        app.menu.front = front;
        complete(&mut app, Ok(prepared(restore, Some(named.clone()))));
        assert_eq!(
            app.save_path, named,
            "saved-result commits choose the supplied slot"
        );
        assert_eq!(std::fs::read(&named).unwrap(), bytes);
        assert!(!app.menu.front);
        let restarted = session();
        let restarted_bytes = yr_app::snapshot::encode(&restarted).unwrap();
        complete(&mut app, Ok(prepared(restarted, None)));
        assert_eq!(
            app.save_path, named,
            "in-match restart keeps its active target"
        );
        assert_eq!(
            std::fs::read(&named).unwrap(),
            bytes,
            "commit does not save implicitly"
        );
        assert_eq!(
            yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
            restarted_bytes
        );
        press_key(&mut app, KeyCode::F9);
        assert_eq!(app.menu.page, Page::Confirm(Intent::Load(named)));
    }
    println!("Front/in-match controlled saved-result commits still choose the named target; in-match restart retains it and does not write files.");
}

#[test]
fn new_match_save_target_waits_for_success_not_preview_failure_or_cancellation() {
    let (mut app, _, _) = scene((640, 480));
    let directory = PreferenceDirectory::new();
    let named = directory.0.join("Protected.oyrsave");
    yr_app::snapshot::save(app.session.as_ref().unwrap(), &named).unwrap();
    let original = std::fs::read(&named).unwrap();
    app.save_path = named.clone();
    app.apply_menu_effect(yr_app::menu::Effect::MainMenu);
    assert_eq!(app.save_path, named);
    complete(
        &mut app,
        Ok(Prepared::Preview(yr_app::frontend::Preview {
            width: 1,
            height: 1,
            rgba: vec![0, 0, 0, 255],
            authored: false,
            setup: Default::default(),
            starts: [None; 8],
            map_rect: (0, 0, 1, 1),
            local_rect: (0, 0, 1, 1),
        })),
    );
    assert_eq!(app.menu.page, Page::Skirmish);
    assert_eq!(app.save_path, named);
    complete(&mut app, Err("controlled preparation failure".into()));
    assert!(matches!(app.menu.page, Page::Message { .. }));
    assert_eq!(app.save_path, named);
    let sender = queued(&mut app, Page::Skirmish);
    app.apply_menu_effect(yr_app::menu::Effect::CancelLoad);
    sender.send(Ok(prepared(session(), None))).ok().unwrap();
    app.poll_menu_load();
    assert_eq!(app.menu.page, Page::Skirmish);
    assert!(app.session.is_none() && app.menu.front);
    assert_eq!(app.save_path, named);
    assert_eq!(std::fs::read(&named).unwrap(), original);
    assert!(!directory.0.join("quicksave.oyrsave").exists());
    assert_eq!(std::fs::read_dir(&directory.0).unwrap().count(), 1);
    println!("Returning to Main, preview success, preparation failure and late cancelled match success preserve target and named-file bytes; no quicksave is created.");
}

fn proof(app: &mut App, name: &str, named: &std::path::Path, original: &[u8]) {
    let Some(directory) = std::env::var_os("OPENYR_NEW_MATCH_SAVE_PROOF") else {
        return;
    };
    let directory = PathBuf::from(directory);
    std::fs::create_dir_all(&directory).unwrap();
    if app.map.as_ref().is_some_and(|m| m.rgba.is_empty()) {
        app.map.as_mut().unwrap().rgba = [30, 40, 30, 255]
            .into_iter()
            .cycle()
            .take(1600 * 1200 * 4)
            .collect();
    }
    let mut frame = vec![0; 640 * 480 * 4];
    app.compose(&mut frame, 640, 480);
    std::fs::write(
        directory.join(format!("{name}.png")),
        yr_render::png::encode_rgba(640, 480, &frame),
    )
    .unwrap();
    let session = app.session.as_ref().unwrap();
    let preserved = std::fs::read(named).unwrap() == original;
    assert!(preserved);
    std::fs::write(directory.join(format!("{name}.json")), serde_json::to_vec_pretty(&serde_json::json!({
        "page":format!("{:?}", app.menu.page), "target":app.save_path.file_name().unwrap().to_string_lossy(),
        "named_file_preserved":preserved, "named_bytes":original.len(),
        "tick":session.sim.tick(), "sim_hash":format!("{:016x}", session.sim.state_hash()),
        "snapshot_bytes":yr_app::snapshot::encode(session).unwrap().len(),
        "retail_art":false, "controlled_worker_result":true,
    })).unwrap()).unwrap();
}
