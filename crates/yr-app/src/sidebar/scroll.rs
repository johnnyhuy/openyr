use super::*;
use std::time::{Duration, Instant};

const HOLD_DELAY: Duration = Duration::from_millis(350);
const HOLD_INTERVAL: Duration = Duration::from_millis(80);

#[derive(Debug, Clone, Copy)]
struct ScrollPress {
    direction: i32,
    size: (u32, u32),
    next: Instant,
    inside: bool,
}

/// Presentation-only row offsets. Each category keeps its own position.
#[derive(Debug, Clone, Copy, Default)]
pub struct View {
    pub category: Category,
    rows: [usize; 4],
    wheel: f64,
    pressed: Option<(Category, bool)>,
    press_inside: bool,
    scroll_press: Option<ScrollPress>,
}

impl From<Category> for View {
    fn from(category: Category) -> Self {
        Self {
            category,
            ..Self::default()
        }
    }
}

fn visible_rows(height: u32) -> usize {
    (height.saturating_sub(TOP as u32 + 26) + GAP as u32) as usize / (SLOT_H + GAP) as usize
}

impl View {
    fn index(self) -> usize {
        Category::ALL
            .iter()
            .position(|&c| c == self.category)
            .unwrap()
    }

    fn max_row(self, session: &Session, height: u32) -> usize {
        let visible = visible_rows(height);
        if visible == 0 {
            return 0;
        }
        slots(session, self.category)
            .count()
            .div_ceil(2)
            .saturating_sub(visible)
    }

    pub fn first_row(self, session: &Session, height: u32) -> usize {
        self.rows[self.index()].min(self.max_row(session, height))
    }

    pub fn select(&mut self, category: Category) {
        self.clear_press();
        self.category = category;
        self.clear_wheel();
    }

    /// Keep the pre-press selection for the sheet's two pressed variants.
    pub fn press(&mut self, category: Category) {
        let selected = self.category == category;
        self.select(category);
        self.pressed = Some((category, selected));
        self.press_inside = true;
    }

    pub fn clear_press(&mut self) {
        self.pressed = None;
        self.press_inside = false;
        self.scroll_press = None;
    }

    /// The origin tab owns the gesture even while its held art is not visible.
    pub fn tab_pressed(self) -> bool {
        self.pressed.is_some()
    }

    /// A held sidebar control keeps ownership while the pointer is outside it.
    pub fn pointer_pressed(self) -> bool {
        self.tab_pressed() || self.scroll_press.is_some()
    }

    pub fn press_scroll(
        &mut self,
        session: &Session,
        size: (u32, u32),
        pos: (f64, f64),
        now: Instant,
    ) -> bool {
        let Some(direction) = self.scroll_at(session, size.0, size.1, pos) else {
            return false;
        };
        self.clear_press();
        self.scroll(session, size.1, direction);
        self.scroll_press = Some(ScrollPress {
            direction,
            size,
            next: now + HOLD_DELAY,
            inside: true,
        });
        true
    }

    pub fn track_scroll(
        &mut self,
        session: &Session,
        size: (u32, u32),
        pos: (f64, f64),
        now: Instant,
    ) {
        let Some(press) = self.scroll_press else {
            return;
        };
        if size != press.size {
            self.scroll_press = None;
            return;
        }
        let inside = self.scroll_at(session, size.0, size.1, pos) == Some(press.direction);
        let press = self.scroll_press.as_mut().unwrap();
        if !inside || !press.inside {
            press.next = now + HOLD_DELAY;
        }
        press.inside = inside;
    }

    /// Delayed redraws advance at most one row, without a catch-up burst.
    pub fn repeat_scroll(
        &mut self,
        session: &Session,
        size: (u32, u32),
        pos: (f64, f64),
        now: Instant,
    ) -> bool {
        self.track_scroll(session, size, pos, now);
        let Some(press) = self.scroll_press.as_mut() else {
            return false;
        };
        if !press.inside || now < press.next {
            return false;
        }
        press.next = now + HOLD_INTERVAL;
        let direction = press.direction;
        self.scroll(session, size.1, direction)
    }

    pub fn track_press(&mut self, hovered: Option<Category>) {
        self.press_inside = self.pressed.is_some_and(|(tab, _)| Some(tab) == hovered);
    }

    pub(super) fn tab_frame(self, category: Category) -> usize {
        if let Some((tab, selected)) = self.pressed.filter(|_| self.press_inside) {
            if tab == category {
                return 3 + usize::from(selected);
            }
        }
        usize::from(category == self.category)
    }

    pub fn clear_wheel(&mut self) {
        self.wheel = 0.0;
    }

    pub fn scroll(&mut self, session: &Session, height: u32, rows: i32) -> bool {
        self.wheel = 0.0;
        let first = self.first_row(session, height);
        let next = first
            .saturating_add_signed(rows as isize)
            .min(self.max_row(session, height));
        self.rows[self.index()] = next;
        first != next
    }

    /// Accumulate high-resolution wheel input in rows, positive downward.
    pub fn wheel(&mut self, session: &Session, height: u32, rows: f64) -> bool {
        if !rows.is_finite() {
            return false;
        }
        let sum = self.wheel + rows.clamp(-128.0, 128.0);
        let whole = sum.trunc() as i32;
        let changed = self.scroll(session, height, whole);
        let first = self.first_row(session, height);
        // Discard outward motion at either end so reversing responds at once.
        if (sum > 0.0 && first < self.max_row(session, height)) || (sum < 0.0 && first > 0) {
            self.wheel = sum.fract();
        }
        changed
    }

    pub(super) fn visible_slots(
        self,
        session: &Session,
        height: u32,
    ) -> impl Iterator<Item = (usize, usize)> + '_ {
        slots(session, self.category)
            .skip(self.first_row(session, height) * 2)
            .take(visible_rows(height) * 2)
            .enumerate()
    }

    fn buttons(
        self,
        session: &Session,
        width: u32,
        height: u32,
    ) -> impl Iterator<Item = (i32, (i32, i32), bool)> {
        let max = self.max_row(session, height);
        let first = self.first_row(session, height);
        [-1, 1]
            .into_iter()
            .enumerate()
            .filter_map(move |(i, direction)| {
                (max > 0).then_some((
                    direction,
                    (slot_origin(width, i).0, height as i32 - 24),
                    if direction < 0 {
                        first > 0
                    } else {
                        first < max
                    },
                ))
            })
    }

    /// Only enabled scroll buttons return an action. Cameo gutters stay inert.
    pub fn scroll_at(
        self,
        session: &Session,
        width: u32,
        height: u32,
        pos: (f64, f64),
    ) -> Option<i32> {
        if !contains((0, 0), (width as i32, height as i32), pos) {
            return None;
        }
        self.buttons(session, width, height)
            .find_map(|(direction, origin, enabled)| {
                (enabled && contains(origin, (SLOT_W, 20), pos)).then_some(direction)
            })
    }

    pub(super) fn draw_buttons(self, canvas: &mut Canvas<'_>, session: &Session) {
        for (direction, (x, y), enabled) in self.buttons(session, canvas.width, canvas.height) {
            let color = if enabled { GOLD } else { [96, 100, 104, 255] };
            let pressed = enabled
                && self.scroll_press.is_some_and(|press| {
                    press.direction == direction
                        && press.inside
                        && press.size == (canvas.width, canvas.height)
                });
            canvas.rect((x, y), (SLOT_W, 20), color);
            canvas.rect(
                (x + 1, y + 1),
                (SLOT_W - 2, 18),
                if pressed { [68, 58, 30, 255] } else { PANEL },
            );
            for row in 0..5 {
                let dy = if direction < 0 { row } else { 4 - row };
                canvas.rect((x + SLOT_W / 2 - row, y + 7 + dy), (row * 2 + 1, 1), color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_complete_rows_fit_above_the_bottom_cap() {
        for (height, rows) in [(0, 0), (301, 0), (302, 1), (351, 1), (352, 2), (800, 10)] {
            assert_eq!(visible_rows(height), rows, "height {height}");
        }
        assert!(visible_rows(u32::MAX) > 0);
    }
}
