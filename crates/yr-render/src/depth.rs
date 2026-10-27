//! Painter order for ground footprints in the isometric projection.

/// Map-space bounds. Units use a point; buildings use their full
/// foundation. Height affects projection, not this ground relation.
#[derive(Clone, Copy)]
pub struct Footprint {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Footprint {
    fn before(self, other: Self) -> bool {
        self.x + self.w <= other.x || self.y + self.h <= other.y
    }

    fn depth(self) -> f32 {
        self.x + self.w + self.y + self.h
    }
}

/// Respect non-overlapping foundations along either map axis. Sorting
/// by x+y alone lets a wide rear building cover its smaller neighbour
/// or a unit beside it. Diagonally separated objects have no dependency.
/// Stable diagonal order resolves ties, overlap and any dependency cycle.
pub fn painter_order(bounds: &[Footprint]) -> Vec<usize> {
    let n = bounds.len();
    let mut edges = vec![Vec::new(); n];
    let mut incoming = vec![0usize; n];
    for a in 0..n {
        for b in a + 1..n {
            let ab = bounds[a].before(bounds[b]);
            let ba = bounds[b].before(bounds[a]);
            let edge = match (ab, ba) {
                (true, false) => Some((a, b)),
                (false, true) => Some((b, a)),
                _ => None,
            };
            if let Some((from, to)) = edge {
                edges[from].push(to);
                incoming[to] += 1;
            }
        }
    }
    let mut pending: Vec<_> = (0..n).collect();
    pending.sort_by(|&a, &b| bounds[a].depth().total_cmp(&bounds[b].depth()));
    let mut order = Vec::with_capacity(n);
    while !pending.is_empty() {
        let index = pending.iter().position(|&i| incoming[i] == 0).unwrap_or(0);
        let from = pending.remove(index);
        order.push(from);
        for &to in &edges[from] {
            incoming[to] -= 1;
        }
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f32, y: f32, w: f32, h: f32) -> Footprint {
        Footprint { x, y, w, h }
    }

    #[test]
    fn wide_building_stays_behind_smaller_east_and_south_neighbours() {
        let yard = rect(10.0, 10.0, 5.0, 5.0);
        for plant in [rect(15.0, 10.0, 2.0, 2.0), rect(10.0, 15.0, 2.0, 2.0)] {
            assert_eq!(painter_order(&[plant, yard]), [1, 0]);
            assert_eq!(painter_order(&[yard, plant]), [0, 1]);
        }
    }

    #[test]
    fn units_on_each_side_of_a_foundation_have_the_right_order() {
        let yard = rect(10.0, 10.0, 4.0, 4.0);
        for (x, y) in [(14.5, 10.5), (10.5, 14.5)] {
            assert_eq!(painter_order(&[rect(x, y, 0.0, 0.0), yard]), [1, 0]);
        }
        for (x, y) in [(9.5, 13.5), (13.5, 9.5)] {
            assert_eq!(painter_order(&[yard, rect(x, y, 0.0, 0.0)]), [1, 0]);
        }
        assert_eq!(painter_order(&[yard, yard]), [0, 1]);
        assert!(painter_order(&[]).is_empty());
    }
}
