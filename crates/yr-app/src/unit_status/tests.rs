use super::*;

#[test]
fn health_pips_have_exact_empty_full_and_damage_boundaries() {
    let size = (640, 480);
    for (fraction, color, filled) in [
        (1.0, [0, 252, 0, 255], 9),
        (0.51, [0, 252, 0, 255], 5),
        (0.5, [252, 252, 0, 255], 5),
        (0.25, [252, 0, 0, 255], 3),
        (0.01, [252, 0, 0, 255], 1),
        (0.0, [252, 0, 0, 255], 0),
        (-1.0, [252, 0, 0, 255], 0),
        (2.0, [0, 252, 0, 255], 9),
    ] {
        let mut frame = vec![0; size.0 as usize * size.1 as usize * 4];
        draw_health(&mut frame, size, (100.0, 100.0), 28.0, fraction);
        assert_eq!(
            frame
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|p| **p == color)
                .count(),
            filled * 2
        );
        // A full 28 by 4 frame, including separators and empty pip slots.
        assert_eq!(
            frame
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|p| p[3] == 255)
                .count(),
            112
        );
    }
}

#[test]
fn health_clips_world_edges_and_rejects_nonfinite_or_malformed_inputs() {
    let size = (640, 480);
    for center in [(-1.0, -1.0), (471.0, 479.0), (480.0, 200.0)] {
        let mut frame = vec![0; size.0 as usize * size.1 as usize * 4];
        draw_health(&mut frame, size, center, f32::MAX, 1.0);
        for row in frame.chunks_exact(size.0 as usize * 4) {
            assert!(row[(size.0 - SIDEBAR_W) as usize * 4..]
                .iter()
                .all(|b| *b == 0));
        }
    }
    for (center, width, fraction) in [
        ((f32::NAN, 0.0), 28.0, 1.0),
        ((0.0, f32::INFINITY), 28.0, 1.0),
        ((f32::MAX, 0.0), 28.0, 1.0),
        ((0.0, 0.0), f32::NAN, 1.0),
        ((0.0, 0.0), 28.0, f32::NAN),
        ((0.0, 0.0), -1.0, 1.0),
    ] {
        let mut frame = vec![0; size.0 as usize * size.1 as usize * 4];
        draw_health(&mut frame, size, center, width, fraction);
        assert!(frame.iter().all(|b| *b == 0));
    }
    let mut invalid = [0; 9];
    draw_health(&mut invalid, size, (0.0, 0.0), 28.0, 1.0);
    assert_eq!(invalid, [0; 9]);
}
