//! A* pathfinding over the terrain grid.
//!
//! Deterministic by construction: fixed neighbour order, integer costs,
//! and heap ties broken by (f, h, y, x) — never by hash or insertion
//! order. Diagonal steps cost 141% and may not cut corners past
//! impassable orthogonal neighbours.

use crate::terrain::{Loco, TerrainGrid};
use std::cmp::Reverse;
use std::collections::BinaryHeap;

/// Fixed neighbour order: N, NE, E, SE, S, SW, W, NW (map cells, y down).
const DIRS: [(i32, i32); 8] = [
    (0, -1),
    (1, -1),
    (1, 0),
    (1, 1),
    (0, 1),
    (-1, 1),
    (-1, 0),
    (-1, -1),
];

fn enter_cost(grid: &TerrainGrid, loco: Loco, x: u16, y: u16, diag: bool) -> Option<i64> {
    let f = grid.speed_factor(loco, x, y)?;
    // Cost inversely proportional to speed: 100% ground = 25600.
    let base = 6_553_600i64 / i64::from(f.raw()).max(1);
    Some(if diag { base * 141 / 100 } else { base })
}

/// Path from `from` to `to` (both map cells), excluding `from`,
/// including `to`. `None` when unreachable or an endpoint is impassable.
pub fn find_path(
    grid: &TerrainGrid,
    loco: Loco,
    from: (u16, u16),
    to: (u16, u16),
) -> Option<Vec<(u16, u16)>> {
    find_path_avoiding(grid, loco, from, to, &[])
}

/// Nearest cell to `to` that `loco` may stand on, searching square rings
/// out to `max_r` in fixed (y, x) order; `to` itself when passable.
pub fn nearest_passable(grid: &TerrainGrid, loco: Loco, to: (u16, u16), max_r: i32) -> Option<(u16, u16)> {
    for r in 0..=max_r {
        for dy in -r..=r {
            for dx in -r..=r {
                if dx.abs() != r && dy.abs() != r {
                    continue;
                }
                let (x, y) = (i32::from(to.0) + dx, i32::from(to.1) + dy);
                let (Ok(x), Ok(y)) = (u16::try_from(x), u16::try_from(y)) else { continue };
                if grid.passable(loco, x, y) {
                    return Some((x, y));
                }
            }
        }
    }
    None
}

/// As [`find_path`], additionally treating `avoid` cells (e.g. cells
/// other units stand on) as blocked, except the goal itself.
pub fn find_path_avoiding(
    grid: &TerrainGrid,
    loco: Loco,
    from: (u16, u16),
    to: (u16, u16),
    avoid: &[(u16, u16)],
) -> Option<Vec<(u16, u16)>> {
    if from == to {
        return Some(Vec::new());
    }
    if !grid.passable(loco, to.0, to.1) {
        return None;
    }
    let (w, h) = (grid.width(), grid.height());
    let n = w * h;
    let mut g_score = vec![i64::MAX; n];
    let mut came: Vec<Option<usize>> = vec![None; n];
    let mut closed = vec![false; n];
    let start = grid.local_index(from.0, from.1)?;
    let goal = grid.local_index(to.0, to.1)?;
    let mut blocked = vec![false; n];
    for &(x, y) in avoid {
        if let Some(i) = grid.local_index(x, y) {
            blocked[i] = i != goal;
        }
    }
    let heuristic = |i: usize| -> i64 {
        let (x, y) = ((i % w) as i64, (i / w) as i64);
        let (gx, gy) = ((goal % w) as i64, (goal / w) as i64);
        let (dx, dy) = ((x - gx).abs(), (y - gy).abs());
        // Octile distance at the minimum cell cost scale.
        25_600 * (dx + dy) + (36_200 - 2 * 25_600) * dx.min(dy)
    };
    // (f, h, y, x) packed so Reverse gives a total, deterministic order.
    let key = |f: i64, hh: i64, i: usize| (Reverse(f), Reverse(hh), Reverse(i / w), Reverse(i % w));
    type HeapKey = (Reverse<i64>, Reverse<i64>, Reverse<usize>, Reverse<usize>, usize);
    let mut open: BinaryHeap<HeapKey> = BinaryHeap::new();
    g_score[start] = 0;
    let h0 = heuristic(start);
    let (a, b, c, d) = key(h0, h0, start);
    open.push((a, b, c, d, start));
    while let Some((_, _, _, _, cur)) = open.pop() {
        if closed[cur] {
            continue;
        }
        closed[cur] = true;
        if cur == goal {
            let mut path = Vec::new();
            let mut at = goal;
            while at != start {
                path.push(grid.cell_at_local(at % w, at / w));
                at = came[at].expect("path chain leads to start");
            }
            path.reverse();
            return Some(path);
        }
        let (cx, cy) = ((cur % w) as i32, (cur / w) as i32);
        for (dx, dy) in DIRS {
            let (nx, ny) = (cx + dx, cy + dy);
            if nx < 0 || ny < 0 || nx as usize >= w || ny as usize >= h {
                continue;
            }
            let (mx, my) = grid.cell_at_local(nx as usize, ny as usize);
            let diag = dx != 0 && dy != 0;
            let (fx, fy) = grid.cell_at_local(cx as usize, cy as usize);
            if !grid.can_step(loco, fx, fy, mx, my) {
                continue;
            }
            if diag {
                // No corner cutting.
                let (ax, ay) = grid.cell_at_local((cx + dx) as usize, cy as usize);
                let (bx, by) = grid.cell_at_local(cx as usize, (cy + dy) as usize);
                if !grid.passable(loco, ax, ay) || !grid.passable(loco, bx, by) {
                    continue;
                }
            }
            let Some(cost) = enter_cost(grid, loco, mx, my, diag) else { continue };
            let ni = ny as usize * w + nx as usize;
            if blocked[ni] {
                continue;
            }
            let ng = g_score[cur] + cost;
            if ng < g_score[ni] {
                g_score[ni] = ng;
                came[ni] = Some(cur);
                let hh = heuristic(ni);
                let (a, b, c, d) = key(ng + hh, hh, ni);
                open.push((a, b, c, d, ni));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::terrain::TerrainGrid;

    fn grid_with_wall() -> TerrainGrid {
        // 5x5 clear, wall column x=2 except a gap at y=4 (raw 15 = cliff).
        let mut tiles = Vec::new();
        for y in 0..5u16 {
            for x in 0..5u16 {
                let raw = if x == 2 && y != 4 { 15 } else { 13 };
                tiles.push((x, y, 0, raw, 0));
            }
        }
        TerrainGrid::from_tiles(0, 0, 5, 5, tiles.into_iter())
    }

    #[test]
    fn path_detours_through_the_gap() {
        let g = grid_with_wall();
        let path = find_path(&g, Loco::Track, (0, 0), (4, 0)).expect("path exists");
        assert!(path.contains(&(2, 4)), "must use the gap: {path:?}");
        assert_eq!(path.last(), Some(&(4, 0)));
        // Same request is deterministic.
        assert_eq!(path, find_path(&g, Loco::Track, (0, 0), (4, 0)).unwrap());
    }

    #[test]
    fn unreachable_returns_none() {
        let g = grid_with_wall();
        // Cliff cell itself is not a legal target.
        assert_eq!(find_path(&g, Loco::Foot, (0, 0), (2, 0)), None);
        // Ships go nowhere on land.
        assert_eq!(find_path(&g, Loco::Ship, (0, 0), (4, 0)), None);
    }

    #[test]
    fn avoiding_routes_around_units_and_nearest_passable_skips_cliffs() {
        let g = grid_with_wall();
        // Block the direct row; the path must step off y=0.
        let p = find_path_avoiding(&g, Loco::Track, (0, 0), (1, 0), &[(1, 0)]).expect("goal itself never avoided");
        assert_eq!(p, vec![(1, 0)]);
        let p = find_path_avoiding(&g, Loco::Track, (0, 4), (4, 4), &[(2, 4)]);
        assert_eq!(p, None, "the only gap is held");
        // Cliff (2,0): nearest standable cell is a neighbour in ring order.
        assert_eq!(nearest_passable(&g, Loco::Track, (2, 0), 2), Some((1, 0)));
        assert_eq!(nearest_passable(&g, Loco::Track, (0, 0), 2), Some((0, 0)));
    }
}
