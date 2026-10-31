use super::*;
use crate::menu::{Button, Effect, Layout, Page, State};
#[test]
fn front_navigation_geometry_and_disabled_modes_are_consistent() {
    let mut s = State {
        front: true,
        ..State::default()
    };
    s.set(Page::Main);
    for b in [Button::Internet, Button::Network, Button::Movies] {
        assert_eq!(s.activate(b), Effect::None);
        assert_eq!(s.page, Page::Main);
    }
    s.activate(Button::SinglePlayer);
    assert_eq!(s.page, Page::SinglePlayer);
    assert_eq!(s.activate(Button::Campaign), Effect::None);
    s.activate(Button::Load);
    s.escape();
    assert_eq!(s.page, Page::SinglePlayer);
    s.activate(Button::MainMenu);
    s.activate(Button::Controls);
    s.escape();
    assert_eq!(s.page, Page::Main);
    s.escape();
    assert_eq!(s.page, Page::Exit);
    s.escape();
    assert_eq!(s.page, Page::Main);
    for (w, h) in [(1280, 800), (800, 600), (640, 480), (400, 300)] {
        let l = Layout::for_state(&s, w, h);
        let (x, y, _, _) = l.button(0, Button::SinglePlayer);
        assert_eq!(
            l.hit(
                &s,
                (
                    l.origin.0 + f64::from(x + 4) * l.scale,
                    l.origin.1 + f64::from(y + 4) * l.scale
                )
            ),
            Some(Button::SinglePlayer)
        );
        assert_eq!(l.hit(&s, (-1.0, 0.0)), None);
    }
}
#[test]
fn retail_shell_uses_its_own_button_palette_and_preserves_source_pixels() {
    use yr_formats::{Palette, ShpFile};
    let dir = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join("Games/RA2YR/prefix/drive_c/Westwood/RA2");
    if !dir.is_dir() {
        return;
    }
    let fs = yr_assets::AssetFs::open(&dir).unwrap();
    let art = Art::load(&fs);
    for (name, palette, images) in [
        ("SDTP.SHP", "SHELL.PAL", &art.top),
        ("SDWRNANM.SHP", "SHELL2.PAL", &art.warning),
        ("SDBTNANM.SHP", "SDBTNANM.PAL", &art.buttons),
    ] {
        let s = ShpFile::parse(fs.read(name).unwrap()).unwrap();
        let p = Palette::parse(fs.read(palette).unwrap()).unwrap();
        assert_eq!(images.len(), s.frames.len());
        for (i, image) in images.iter().enumerate() {
            for (index, value) in s.decode_frame_canvas(i).unwrap().into_iter().enumerate() {
                assert_eq!(
                    &image.pixels[index * 4..index * 4 + 4],
                    &if value == 0 { [0; 4] } else { p.rgba(value) },
                    "{name} frame {i}"
                );
            }
        }
    }
    let s = State {
        front: true,
        page: Page::Main,
        ..State::default()
    };
    let mut frame = vec![0; 800 * 600 * 4];
    draw(
        &mut frame,
        800,
        600,
        &art,
        Scene {
            state: &s,
            movie: None,
            preview: None,
            phase: 0,
            cursor: (-1.0, -1.0),
        },
    );
    for y in [0, 1, 2, 40, 41] {
        for x in 0..156 {
            let source = (y * 156 + x) * 4;
            let pixel = &art.buttons[2].pixels[source..source + 4];
            if pixel[3] != 0 {
                let dest = ((199 + y) * 800 + 638 + x) * 4;
                assert_eq!(&frame[dest..dest + 4], pixel);
            }
        }
    }
}
