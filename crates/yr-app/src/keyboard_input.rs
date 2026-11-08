//! Keyboard event routing shared by the native window and input regressions.
use super::*;

impl App {
    pub(super) fn keyboard_input(
        &mut self,
        physical_key: PhysicalKey,
        state: ElementState,
        text: Option<&str>,
        repeat: bool,
        is_synthetic: bool,
    ) {
        let PhysicalKey::Code(code) = physical_key else {
            return;
        };
        // Focus synchronization must never become a new menu action or order.
        // Releases still clear held keys after focus loss or a modal transition.
        if state == ElementState::Released {
            self.keys.remove(&code);
            self.suppressed_keys.remove(&code);
            return;
        }
        if is_synthetic || !self.pointer.focused {
            self.suppressed_keys.insert(code);
            return;
        }
        if repeat && self.suppressed_keys.contains(&code) {
            return;
        }
        self.suppressed_keys.remove(&code);
        if self.menu.visible() {
            self.menu_key_code(code, text, repeat);
            return;
        }
        let now = Instant::now();
        if self.group_key(code, state, repeat, now) || self.unit_order_key(code, state, repeat, now)
        {
            return;
        }
        self.groups.interrupt();
        self.keys.insert(code);
        match code {
            KeyCode::ArrowLeft | KeyCode::ArrowRight | KeyCode::ArrowUp | KeyCode::ArrowDown => {
                // Releases can arrive before the redraw that pans the camera.
                self.cancel_pointer_action();
            }
            KeyCode::Escape if !repeat => {
                self.escape_command_or_pause();
            }
            KeyCode::Equal => self.zoom_camera((self.view.zoom * 1.2).min(4.0)),
            KeyCode::Minus => self.zoom_camera((self.view.zoom / 1.2).max(0.25)),
            KeyCode::KeyA if !repeat => {
                self.order_feedback.clear();
                self.cancel_pointer_action();
                self.placing = None;
                self.service_mode = ServiceMode::None;
                self.attack_move = true;
            }
            KeyCode::KeyK if !repeat => self.toggle_service(ServiceMode::Repair),
            KeyCode::KeyL if !repeat => self.toggle_service(ServiceMode::Sell),
            KeyCode::KeyM if !repeat => {
                self.apply_menu_effect(yr_app::menu::Effect::Mute);
            }
            KeyCode::F5 if !repeat => self.save_menu_path(self.save_path.clone()),
            KeyCode::F9 if !repeat => {
                self.open_menu(yr_app::menu::Page::Confirm(yr_app::menu::Intent::Load(
                    self.save_path.clone(),
                )));
            }
            _ => {}
        }
        self.view.dirty = true;
    }
}

#[cfg(test)]
mod tests;
