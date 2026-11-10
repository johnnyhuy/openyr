use super::*;
use crate::menu::{Button, Effect, Layout, Page, State};
use crate::sidebar::art::Image;

fn art() -> Art {
    Art {
        buttons: (0..17)
            .map(|n| Image {
                width: 156,
                height: 42,
                pixels: [n * 8, 32, 80, 255]
                    .into_iter()
                    .cycle()
                    .take(156 * 42 * 4)
                    .collect(),
            })
            .collect(),
        ..Art::default()
    }
}
fn frame(state: &State, phase: u64, art: &Art) -> Vec<u8> {
    let mut frame = vec![0; 800 * 600 * 4];
    draw(
        &mut frame,
        800,
        600,
        art,
        Scene {
            state,
            movie: None,
            preview: None,
            phase,
            cursor: (-1., -1.),
        },
    );
    frame
}
fn sample(state: &State, button: Button, pixels: &[u8]) -> [u8; 4] {
    let index = state
        .buttons()
        .iter()
        .position(|&(id, _)| id == button)
        .unwrap();
    let (x, y, _, _) = Layout::for_state(state, 800, 600).button(index, button);
    let at = (((y + 1) * 800 + x + 1) * 4) as usize;
    pixels[at..at + 4].try_into().unwrap()
}
#[test]
fn menu_click_animation_advances_caps_and_releases_without_changing_actions() {
    let mut state = State {
        front: true,
        ..State::default()
    };
    state.set(Page::Main);
    let art = art();
    state.press_button(Some(Button::SinglePlayer), 10);
    for (phase, expected) in [(10, 4), (11, 5), (22, 16), (u64::MAX, 16)] {
        assert_eq!(
            sample(&state, Button::SinglePlayer, &frame(&state, phase, &art)),
            [expected * 8, 32, 80, 255]
        );
    }
    assert_eq!(
        state.activate_feedback(Button::SinglePlayer, 23),
        Some(Effect::None)
    );
    assert_eq!(state.page, Page::SinglePlayer);
    assert!(
        state.activation.is_none(),
        "departing control must not paint over another button"
    );
    state.set(Page::Controls);
    for (phase, expected) in [(30, 4), (34, 8), (42, 16), (43, 2)] {
        if phase == 30 {
            assert_eq!(
                state.activate_feedback(Button::EdgeScroll, phase),
                Some(Effect::EdgeScroll)
            );
        }
        let image = frame(&state, phase, &art);
        assert_eq!(
            sample(&state, Button::EdgeScroll, &image),
            [expected * 8, 32, 80, 255]
        );
        if let Some(path) = std::env::var_os("OPENYR_MENU_CLICK_PROOF") {
            let path = std::path::PathBuf::from(path);
            std::fs::create_dir_all(&path).unwrap();
            std::fs::write(
                path.join(format!("menu-click-phase-{phase}.png")),
                yr_render::png::encode_rgba(800, 600, &image),
            )
            .unwrap();
        }
    }
    state.set(Page::Main);
    assert_eq!(state.activate_feedback(Button::Internet, 50), None);
    assert!(state.activation.is_none() && state.press_phase.is_none());
    println!("Authored front shell: held frames 4-16 cap safely; accepted persistent control animates then returns to normal; navigation and disabled controls keep their semantics.");
}
#[test]
fn menu_click_animation_tolerates_missing_or_short_optional_sheet() {
    let mut state = State {
        front: true,
        ..State::default()
    };
    state.set(Page::Main);
    state.press_button(Some(Button::SinglePlayer), 0);
    for count in [0, 3, 5, 17] {
        let mut art = art();
        art.buttons.truncate(count);
        let image = frame(&state, u64::MAX, &art);
        assert_eq!(image.len(), 800 * 600 * 4);
        if count > 0 {
            let n = if count == 3 {
                2
            } else if count == 5 {
                4
            } else {
                16
            };
            assert_eq!(
                sample(&state, Button::SinglePlayer, &image),
                [n * 8, 32, 80, 255]
            );
        }
    }
}
