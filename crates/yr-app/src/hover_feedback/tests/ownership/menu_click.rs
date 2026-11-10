use super::*;
use yr_app::menu::{Button, Layout};

fn position(app: &mut App, button: Button) {
    let index = app
        .menu
        .buttons()
        .iter()
        .position(|&(id, _)| id == button)
        .unwrap();
    let l = Layout::for_state(&app.menu, 800, 600);
    let (x, y, _, _) = l.button(index, button);
    app.pointer_moved((
        l.origin.0 + f64::from(x + 10) * l.scale,
        l.origin.1 + f64::from(y + 10) * l.scale,
    ));
}
#[test]
fn menu_click_audio_follows_accepted_native_mouse_keyboard_and_mute_routing() {
    let dir = std::env::temp_dir().join(format!(
        "openyr-menu-click-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir(&dir).unwrap();
    let out = dir.join("capture");
    std::fs::create_dir(&out).unwrap();
    std::fs::write(
        dir.join("SOUNDMD.INI"),
        b"[MenuClick]\nSounds=click\nVolume=60\n",
    )
    .unwrap();
    let wave = yr_formats::audio::encode_wav(&yr_formats::audio::Pcm {
        sample_rate: 22050,
        channels: 1,
        samples: (0..2048)
            .map(|i| if i % 32 < 16 { 8000 } else { -8000 })
            .collect(),
    });
    std::fs::write(dir.join("click.wav"), wave).unwrap();
    let (mut app, _, _) = scene((800, 600));
    app.menu.front = true;
    app.menu.set(Page::Main);
    app.audio = Some(
        yr_app::audio::AudioScene::capture(
            dir.to_str().unwrap(),
            "",
            app.session.as_ref().unwrap(),
            out.to_str().unwrap(),
        )
        .unwrap(),
    );
    let before = yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap();
    app.keyboard_input(
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
        None,
        false,
        true,
    );
    assert_eq!(
        app.menu.page,
        Page::Main,
        "synthetic synchronization is not an activation"
    );
    position(&mut app, Button::Internet);
    click(&mut app);
    assert_eq!(app.menu.page, Page::Main);
    position(&mut app, Button::SinglePlayer);
    app.pointer_button(ElementState::Pressed, MouseButton::Left);
    app.pointer_moved((0., 0.));
    app.pointer_button(ElementState::Released, MouseButton::Left);
    assert_eq!(app.menu.page, Page::Main);
    position(&mut app, Button::SinglePlayer);
    click(&mut app);
    assert_eq!(app.menu.page, Page::SinglePlayer);
    app.menu.focus = app
        .menu
        .buttons()
        .iter()
        .position(|&(id, _)| id == Button::MainMenu)
        .unwrap();
    app.keyboard_input(
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
        None,
        false,
        false,
    );
    assert_eq!(app.menu.page, Page::Main);
    app.keyboard_input(
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Pressed,
        None,
        true,
        false,
    );
    app.keyboard_input(
        PhysicalKey::Code(KeyCode::Enter),
        ElementState::Released,
        None,
        false,
        false,
    );
    app.menu.muted = true;
    position(&mut app, Button::SinglePlayer);
    click(&mut app);
    assert_eq!(app.menu.page, Page::SinglePlayer);
    let mut audio = app.audio.take().unwrap();
    audio.update(app.session.as_ref().unwrap(), &[]);
    audio.finish().unwrap();
    let log = std::fs::read_to_string(out.join("audio-cues.tsv")).unwrap();
    assert_eq!(
        log.lines().filter(|l| l.contains("MenuClick")).count(),
        2,
        "{log}"
    );
    let pcm =
        yr_formats::audio::decode_wav(&std::fs::read(out.join("audio.wav")).unwrap()).unwrap();
    assert!(pcm.samples.iter().any(|s| *s != 0));
    assert_eq!(
        yr_app::snapshot::encode(app.session.as_ref().unwrap()).unwrap(),
        before
    );
    app.menu.muted = false;
    app.menu.focus = app
        .menu
        .buttons()
        .iter()
        .position(|&(id, _)| id == Button::MainMenu)
        .unwrap();
    key(&mut app, KeyCode::Enter, ElementState::Pressed);
    assert_eq!(app.menu.page, Page::Main, "missing output remains usable");
    std::fs::remove_file(dir.join("SOUNDMD.INI")).unwrap();
    let missing = dir.join("missing");
    std::fs::create_dir(&missing).unwrap();
    app.audio = Some(
        yr_app::audio::AudioScene::capture(
            dir.to_str().unwrap(),
            "",
            app.session.as_ref().unwrap(),
            missing.to_str().unwrap(),
        )
        .unwrap(),
    );
    position(&mut app, Button::SinglePlayer);
    click(&mut app);
    assert_eq!(
        app.menu.page,
        Page::SinglePlayer,
        "missing optional event stays usable"
    );
    app.audio.take().unwrap().finish().unwrap();
    assert_eq!(
        std::fs::read_to_string(missing.join("audio-cues.tsv"))
            .unwrap()
            .lines()
            .count(),
        1
    );
    if let Some(path) = std::env::var_os("OPENYR_MENU_CLICK_PROOF") {
        let path = PathBuf::from(path);
        std::fs::create_dir_all(&path).unwrap();
        for name in ["audio.wav", "audio-cues.tsv"] {
            std::fs::copy(out.join(name), path.join(name)).unwrap();
        }
    }
    std::fs::remove_dir_all(&dir).unwrap();
    println!("Native mouse and keyboard accepted activations emit exactly two authored MenuClick cues with nonzero mixed PCM; disabled/cancelled/repeated/muted actions remain silent and Session bytes stay exact. No audio device is exercised.");
}
