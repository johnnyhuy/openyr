//! Keyboard event routing shared by the native window and input regressions.
use super::*;

impl App {
    /// Repeat events from an earlier input surface need a release or fresh press.
    pub(super) fn suppress_held_keys(&mut self) {
        self.suppressed_keys.extend(self.held_keys.iter().copied());
        self.suppressed_keys.extend(self.keys.drain());
    }

    pub(super) fn menu_input_context(
        &self,
    ) -> (yr_app::menu::Page, Option<yr_app::menu::setup::Field>) {
        (
            self.menu.page.clone(),
            self.menu.setup.popup.map(|popup| popup.field),
        )
    }

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
            self.held_keys.remove(&code);
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
        // A different deliberate key ends a consecutive type-selection pair.
        if code != KeyCode::KeyT && !repeat {
            self.interrupt_type_press();
        }
        self.held_keys.insert(code);
        if self.menu.visible() {
            let context = self.menu_input_context();
            self.menu_key_code(code, text, repeat);
            if context != self.menu_input_context() {
                self.suppress_held_keys();
            }
            return;
        }
        // Menu editors own Command shortcuts; match bindings use their documented modifiers.
        if self.super_key {
            self.keys.remove(&code);
            self.suppressed_keys.insert(code);
            if !repeat {
                self.groups.interrupt();
                self.interrupt_type_press();
            }
            self.view.dirty = true;
            return;
        }
        let now = Instant::now();
        if self.group_key(code, state, repeat, now)
            || self.type_select_key(code, state, repeat, now)
            || self.select_all_key(code, repeat, now)
            || self.base_key(code, repeat, now)
            || self.sidebar_key(code, repeat)
            || self.unit_order_key(code, state, repeat, now)
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
