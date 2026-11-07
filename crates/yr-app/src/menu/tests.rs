use super::*;

#[test]
fn long_save_names_keep_the_edited_suffix_visible() {
    fn check(art: &Art) {
        for (width, height) in [(640, 480), (800, 600), (1280, 800)] {
            let render = |last| {
                let mut state = State::default();
                state.set(Page::Save);
                state.name = format!("{}{last}", "W".repeat(47));
                let mut frame = vec![0; (width * height * 4) as usize];
                draw(&mut frame, width, height, art, &state, (-1.0, -1.0));
                frame
            };
            let a = render('A');
            let b = render('B');
            assert!(
                a != b,
                "the last edited character must remain visible at {width}x{height}"
            );
            let layout = Layout::new(width, height);
            for (i, (a, b)) in a
                .as_chunks::<4>()
                .0
                .iter()
                .zip(b.as_chunks::<4>().0)
                .enumerate()
            {
                if a != b {
                    let (x, y) = layout
                        .local(((i as u32 % width) as f64, (i as u32 / width) as f64))
                        .unwrap();
                    assert!(
                        (48..layout.width as i32 - 168 - 62).contains(&x)
                            && (414..444).contains(&y)
                    );
                }
            }
        }
    }
    check(&Art::default());
    let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if dir.is_dir() {
        let art = Art::load(&yr_assets::AssetFs::open(&dir).unwrap());
        assert!(art.font.is_some());
        check(&art);
    }
}
#[test]
fn modal_navigation_skips_disabled_controls_and_confirms_discard() {
    let mut state = State::default();
    state.set(Page::Pause);
    assert_eq!(state.activate(Button::Delete), Effect::None);
    assert_eq!(state.page, Page::Delete);
    assert_eq!(state.activate(Button::Delete), Effect::None);
    state.escape();
    assert_eq!(state.page, Page::Pause);
    state.focus = 2;
    state.navigate(1);
    assert_eq!(state.focus, 3);
    state.activate(Button::Abort);
    assert_eq!(state.page, Page::Abort);
    assert_eq!(state.activate(Button::Restart), Effect::Restart);
    assert_eq!(state.escape(), Effect::Resume);
    assert!(!state.visible());
    state.set(Page::Load);
    assert_eq!(state.activate(Button::Load), Effect::None);
    state.slots = vec![Slot {
        name: "one".into(),
        path: "one.oyrsave".into(),
    }];
    state.select_slot(0);
    assert_eq!(state.activate(Button::Load), Effect::RequestLoad);
    state.set(Page::Confirm(Intent::Load("one.oyrsave".into())));
    assert_eq!(state.escape(), Effect::None);
    assert_eq!(state.page, Page::Load);
}
#[test]
fn names_geometry_and_small_surface_hits_are_bounded() {
    for name in ["", ".", "..", "../a", "/tmp/a", "a/b", "a\\b", "a:b", "a\n"] {
        assert!(save_name(name).is_none(), "{name:?}");
    }
    assert_eq!(save_name(" mission ").as_deref(), Some("mission.oyrsave"));
    let mut state = State::default();
    state.set(Page::Save);
    state.text(&"界".repeat(1000));
    assert_eq!(state.name.chars().count(), 48);
    for (w, h) in [(1280, 800), (1024, 768), (800, 600), (640, 480), (320, 240)] {
        let layout = Layout::new(w, h);
        let (x, y, _, _) = layout.button(0, Button::Save);
        let point = (
            layout.origin.0 + (f64::from(x) + 1.0) * layout.scale,
            layout.origin.1 + (f64::from(y) + 1.0) * layout.scale,
        );
        assert_eq!(layout.hit(&state, point), Some(Button::Save));
        assert_eq!(layout.hit(&state, (-1.0, 0.0)), None);
        assert!(layout.local((f64::NAN, 1.0)).is_none());
    }
}

#[test]
fn slot_listing_stays_bounded_and_never_follows_symlinks() {
    let path = std::env::temp_dir().join(format!(
        "openyr-menu-slots-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    assert!(slots(&path).unwrap().is_empty());
    std::fs::create_dir(&path).unwrap();
    std::fs::write(path.join("b.oyrsave"), []).unwrap();
    std::fs::write(path.join("a.oyrsave"), []).unwrap();
    std::fs::write(path.join("other.txt"), []).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(path.join("a.oyrsave"), path.join("linked.oyrsave")).unwrap();
    assert_eq!(
        slots(&path)
            .unwrap()
            .iter()
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        ["a", "b"]
    );
    std::fs::remove_dir_all(path).unwrap();
}

#[test]
fn retail_dialog_and_control_pixels_use_the_scoped_source_palettes() {
    use yr_formats::{mix_id, MixArchive, Palette, ShpFile};
    let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        eprintln!("skip: retail install not present");
        return;
    }
    let fs = yr_assets::AssetFs::open(&dir).unwrap();
    let art = Art::load(&fs);
    let archive = MixArchive::parse(fs.read("SIDEC01.MIX").unwrap()).unwrap();
    let ui = Palette::parse(archive.get_by_id(mix_id("UIBKGD.PAL")).unwrap()).unwrap();
    let side = Palette::parse(archive.get_by_id(mix_id("SIDEBAR.PAL")).unwrap()).unwrap();
    assert_eq!(art.backgrounds.len(), 3);
    assert_eq!(art.controls.len(), 3);
    for (image, name) in art
        .backgrounds
        .iter()
        .zip(["BKGDSM.SHP", "BKGDMD.SHP", "BKGDLG.SHP"])
    {
        let shp = ShpFile::parse(archive.get_by_id(mix_id(name)).unwrap()).unwrap();
        let indices = shp.decode_frame_canvas(0).unwrap();
        for (p, &index) in indices.iter().enumerate() {
            assert_eq!(
                &image.pixels[p * 4..p * 4 + 4],
                &if index == 0 { [0; 4] } else { ui.rgba(index) }
            );
        }
    }
    let buttons = ShpFile::parse(archive.get_by_id(0x2f43_f08b).unwrap()).unwrap();
    for i in 0..3 {
        let indices = buttons.decode_frame_canvas(i).unwrap();
        for (p, &index) in indices.iter().enumerate() {
            assert_eq!(
                &art.controls[i].pixels[p * 4..p * 4 + 4],
                &if index == 0 { [0; 4] } else { side.rgba(index) }
            );
        }
    }
    let mut state = State::default();
    state.set(Page::Pause);
    let mut frame = vec![0; 1024 * 768 * 4];
    draw(&mut frame, 1024, 768, &art, &state, (-1.0, -1.0));
    let source = ShpFile::parse(archive.get_by_id(mix_id("BKGDLG.SHP")).unwrap())
        .unwrap()
        .decode_frame_canvas(0)
        .unwrap();
    for y in 80..736 {
        for x in 0..856 {
            let index = source[y * 856 + x];
            if index != 0 {
                assert_eq!(
                    &frame[(y * 1024 + x) * 4..(y * 1024 + x) * 4 + 4],
                    &ui.rgba(index)
                );
            }
        }
    }
    // The authored control's top/bottom bevel is not covered by label glyphs.
    let source = buttons.decode_frame_canvas(0).unwrap();
    for y in [0, 1, 2, 24] {
        for x in 0..125 {
            let index = source[y * 125 + x];
            if index != 0 {
                let at = ((228 + y) * 1024 + 879 + x) * 4;
                assert_eq!(&frame[at..at + 4], &side.rgba(index));
            }
        }
    }
}

#[test]
fn selected_save_name_replacement_filtering_and_navigation_keep_filename_rules() {
    let mut state = State::default();
    state.set(Page::Save);
    state.name = "Old checkpoint".into();
    state.name_selected = true;
    state.text("/\\:\n\t");
    assert_eq!(state.name, "Old checkpoint");
    assert!(
        state.name_selected,
        "invalid text must not consume selection"
    );
    state.text(&"界".repeat(100));
    assert_eq!(state.name.chars().count(), 48);
    assert!(!state.name_selected);
    state.name_selected = true;
    state.backspace_name();
    assert!(state.name.is_empty());
    state.text("new checkpoint");
    assert_eq!(
        save_name(&state.name).as_deref(),
        Some("new checkpoint.oyrsave")
    );
    state.name_selected = true;
    state.navigate(1);
    assert!(!state.name_selected);
    state.slots.push(Slot {
        name: "saved slot".into(),
        path: "saved slot.oyrsave".into(),
    });
    state.name_selected = true;
    state.select_slot(0);
    assert!(!state.name_selected);
    assert_eq!(state.name, "saved slot");
    state.name_selected = true;
    state.set(Page::Pause);
    state.backspace_name();
    state.text("ignored");
    assert_eq!(state.name, "saved slot");
    assert!(!state.name_selected);
}

#[test]
fn save_name_selection_is_visible_only_inside_the_editor_at_three_sizes() {
    let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    let retail = dir
        .is_dir()
        .then(|| Art::load(&yr_assets::AssetFs::open(&dir).unwrap()));
    for art in std::iter::once(Art::default()).chain(retail) {
        let mut state = State::default();
        state.set(Page::Save);
        state.name = format!("{}界", "W".repeat(47));
        for (width, height) in [(640, 480), (800, 600), (1280, 800)] {
            let layout = Layout::for_state(&state, width, height);
            let mut plain = vec![0; (width * height * 4) as usize];
            draw(&mut plain, width, height, &art, &state, (-1.0, -1.0));
            state.name_selected = true;
            let mut selected = vec![0; plain.len()];
            draw(&mut selected, width, height, &art, &state, (-1.0, -1.0));
            let mut changed = 0;
            for (index, (a, b)) in plain
                .as_chunks::<4>()
                .0
                .iter()
                .zip(selected.as_chunks::<4>().0.iter())
                .enumerate()
            {
                if a != b {
                    changed += 1;
                    let p = layout
                        .local(((index as u32 % width) as f64, (index as u32 / width) as f64))
                        .unwrap();
                    assert!(
                        (38..layout.width as i32 - 206).contains(&p.0) && (414..444).contains(&p.1)
                    );
                    assert_eq!(b, [32, 56, 88, 255]);
                }
            }
            assert!(changed > 1000);
            assert_eq!(state.name.chars().count(), 48);
            state.name_selected = false;
        }
    }
}
