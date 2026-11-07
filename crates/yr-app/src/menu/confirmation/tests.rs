use super::*;

#[test]
fn filename_labels_quote_the_basename_without_exposing_directories() {
    for action in [Intent::Save, Intent::Load, Intent::Delete] {
        assert_eq!(
            name(&action("/private/parent/checkpoint.oyrsave".into())),
            "\"checkpoint.oyrsave\""
        );
        assert_eq!(
            name(&action("/private/parent/one\n\t\"\\two.oyrsave".into())),
            "\"one\\n\\t\\\"\\\\two.oyrsave\""
        );
    }
}

#[test]
fn filename_wrapping_preserves_unicode_spaces_and_every_reachable_line() {
    let text = name(&Intent::Load(
        format!("/private/{}.oyrsave", "W  界".repeat(200)).into(),
    ));
    for front in [false, true] {
        let mut state = State {
            front,
            ..Default::default()
        };
        state.set(Page::Confirm(Intent::Load(
            format!("/private/{}.oyrsave", "W  界".repeat(200)).into(),
        )));
        for size in [(640, 480), (800, 600), (1280, 800)] {
            let layout = Layout::for_state(&state, size.0, size.1);
            let face = Typeface(None);
            let wrapped: Vec<_> = lines(face, &text, region(layout).width).collect();
            assert_eq!(wrapped.concat(), text);
            assert!(wrapped
                .iter()
                .all(|s| face.width(s) <= region(layout).width));
            let mut reached = 0;
            loop {
                let (first, rows, count) = viewport(face, &text, layout, state.message_scroll);
                reached = reached.max((first + rows).min(count));
                let previous = state.message_scroll;
                assert!(scroll(None, &mut state, size, 10));
                if state.message_scroll == previous {
                    break;
                }
            }
            assert_eq!(reached, wrapped.len());
            scroll(None, &mut state, size, i32::MIN);
            assert_eq!(state.message_scroll, 0);
        }
        state.set(Page::Pause);
        assert!(!scroll(None, &mut state, (640, 480), 1));
    }
}

#[test]
fn confirmation_targets_wrap_inside_both_skins_with_fallback_and_retail_fonts() {
    fn check(pause: &crate::menu::Art, front_art: &crate::frontend::Art) {
        for front in [false, true] {
            for (width, height) in [(640, 480), (800, 600), (1280, 800)] {
                for action in [Intent::Save, Intent::Load, Intent::Delete] {
                    for prefix in ["W".repeat(47), "W".repeat(200), "界  file ".repeat(12)] {
                        let render = |last| {
                            let mut state = State {
                                front,
                                ..Default::default()
                            };
                            state.set(Page::Confirm(action(
                                format!("/private/{prefix}{last}.oyrsave").into(),
                            )));
                            let mut frame = vec![0; (width * height * 4) as usize];
                            if front {
                                crate::frontend::draw(
                                    &mut frame,
                                    width,
                                    height,
                                    front_art,
                                    crate::frontend::Scene {
                                        state: &state,
                                        movie: None,
                                        preview: None,
                                        phase: 0,
                                        cursor: (-1.0, -1.0),
                                    },
                                );
                            } else {
                                crate::menu::draw(
                                    &mut frame,
                                    width,
                                    height,
                                    pause,
                                    &state,
                                    (-1.0, -1.0),
                                );
                            }
                            frame
                        };
                        let a = render('A');
                        let b = render('B');
                        assert!(a!=b,"complete target suffix must remain visible: front={front} {width}x{height}");
                        let state = State {
                            front,
                            ..Default::default()
                        };
                        let layout = Layout::for_state(&state, width, height);
                        let r = region(layout);
                        for (at, (a, b)) in a
                            .as_chunks::<4>()
                            .0
                            .iter()
                            .zip(b.as_chunks::<4>().0)
                            .enumerate()
                        {
                            if a != b {
                                let (x, y) = layout
                                    .local(((at as u32 % width) as f64, (at as u32 / width) as f64))
                                    .unwrap();
                                assert!(
                                    (r.left..r.left + r.width).contains(&x)
                                        && (r.top..r.top + r.height).contains(&y),
                                    "target text escaped its panel at {x},{y}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    check(
        &crate::menu::Art::default(),
        &crate::frontend::Art::default(),
    );
    let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if dir.is_dir() {
        let fs = yr_assets::AssetFs::open(&dir).unwrap();
        let pause = crate::menu::Art::load(&fs);
        let front = crate::frontend::Art::load(&fs);
        assert!(pause.font.is_some());
        check(&pause, &front);
    }
}
