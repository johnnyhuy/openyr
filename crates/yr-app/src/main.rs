//! OpenYR window: a native pan/zoom view over a rendered retail map.
//!
//! Usage: `openyr <gamedir> <map> [scale]`
//! Drag with the mouse or use WASD/arrows to pan, wheel or +/- to zoom,
//! Esc to quit. The map is rasterised once by `yr-render` (the same code
//! behind `mix-dump render-map`); this shell only presents a viewport.

use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

const WIN_W: u32 = 1280;
const WIN_H: u32 = 800;

struct View {
    cam_x: f32,
    cam_y: f32,
    zoom: f32,
    dirty: bool,
}

struct App {
    gamedir: String,
    map_name: String,
    scale: u32,
    window: Option<std::sync::Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    map: Option<yr_render::RenderedMap>,
    view: View,
    pressed: bool,
    last_cursor: (f64, f64),
    keys: std::collections::HashSet<KeyCode>,
}

impl App {
    fn blit(&mut self) {
        let (Some(pixels), Some(map)) = (&mut self.pixels, &self.map) else {
            return;
        };
        let (fw, fh) = (pixels.texture().width(), pixels.texture().height());
        let frame = pixels.frame_mut();
        let (mw, mh) = (map.width as i32, map.height as i32);
        for sy in 0..fh as i32 {
            let my = self.view.cam_y + sy as f32 / self.view.zoom;
            for sx in 0..fw as i32 {
                let mx = self.view.cam_x + sx as f32 / self.view.zoom;
                let dst = ((sy * fw as i32 + sx) * 4) as usize;
                if mx >= 0.0 && my >= 0.0 && (mx as i32) < mw && (my as i32) < mh {
                    let src = ((my as i32 * mw + mx as i32) * 4) as usize;
                    frame[dst..dst + 4].copy_from_slice(&map.rgba[src..src + 4]);
                } else {
                    frame[dst..dst + 4].copy_from_slice(&[16, 16, 20, 255]);
                }
            }
        }
        self.view.dirty = false;
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let attrs = Window::default_attributes()
            .with_title(format!("OpenYR — {}", self.map_name))
            .with_inner_size(LogicalSize::new(WIN_W, WIN_H));
        let window = std::sync::Arc::new(el.create_window(attrs).expect("create window"));
        let size = window.inner_size();
        let surface = SurfaceTexture::new(size.width.max(1), size.height.max(1), window.clone());
        let pixels = Pixels::new(WIN_W, WIN_H, surface).expect("pixels surface");
        match yr_render::render_map(&self.gamedir, &self.map_name, self.scale) {
            Ok(map) => {
                println!("OpenYR: rendered {}", map.summary);
                self.view.cam_x = (map.width as f32 - WIN_W as f32) / 2.0;
                self.view.cam_y = (map.height as f32 - WIN_H as f32) / 2.0;
                self.map = Some(map);
            }
            Err(e) => {
                eprintln!("OpenYR: cannot render {}: {e}", self.map_name);
                el.exit();
                return;
            }
        }
        self.window = Some(window);
        self.pixels = Some(pixels);
        self.view.dirty = true;
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(size) => {
                if let Some(p) = &mut self.pixels {
                    let _ = p.resize_surface(size.width.max(1), size.height.max(1));
                    self.view.dirty = true;
                }
            }
            WindowEvent::MouseInput {
                state,
                button: MouseButton::Left,
                ..
            } => {
                self.pressed = state == ElementState::Pressed;
            }
            WindowEvent::CursorMoved { position, .. } => {
                if self.pressed {
                    self.view.cam_x -= (position.x - self.last_cursor.0) as f32 / self.view.zoom;
                    self.view.cam_y -= (position.y - self.last_cursor.1) as f32 / self.view.zoom;
                    self.view.dirty = true;
                }
                self.last_cursor = (position.x, position.y);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let d = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => (p.y / 50.0) as f32,
                };
                self.view.zoom = (self.view.zoom * (1.0 + d * 0.1)).clamp(0.25, 4.0);
                self.view.dirty = true;
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(code) = event.physical_key {
                    match event.state {
                        ElementState::Pressed => {
                            self.keys.insert(code);
                            match code {
                                KeyCode::Escape => el.exit(),
                                KeyCode::Equal => self.view.zoom = (self.view.zoom * 1.2).min(4.0),
                                KeyCode::Minus => self.view.zoom = (self.view.zoom / 1.2).max(0.25),
                                _ => {}
                            }
                            self.view.dirty = true;
                        }
                        ElementState::Released => {
                            self.keys.remove(&code);
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let step = 18.0 / self.view.zoom;
                for (k, dx, dy) in [
                    (KeyCode::KeyA, -step, 0.0),
                    (KeyCode::KeyD, step, 0.0),
                    (KeyCode::KeyW, 0.0, -step),
                    (KeyCode::KeyS, 0.0, step),
                ] {
                    if self.keys.contains(&k) {
                        self.view.cam_x += dx;
                        self.view.cam_y += dy;
                        self.view.dirty = true;
                    }
                }
                if self.view.dirty {
                    self.blit();
                }
                if let Some(p) = &mut self.pixels {
                    let _ = p.render();
                }
                if let Some(w) = &self.window {
                    w.request_redraw();
                }
            }
            _ => {}
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (gamedir, map_name, scale) = match args.as_slice() {
        [g, m] => (g.clone(), m.clone(), 2),
        [g, m, s] => (g.clone(), m.clone(), s.parse().unwrap_or(2)),
        _ => {
            eprintln!("usage: openyr <gamedir> <map> [scale]");
            std::process::exit(2);
        }
    };
    let event_loop = EventLoop::new().expect("event loop");
    let mut app = App {
        gamedir,
        map_name,
        scale,
        window: None,
        pixels: None,
        map: None,
        view: View {
            cam_x: 0.0,
            cam_y: 0.0,
            zoom: 1.0,
            dirty: true,
        },
        pressed: false,
        last_cursor: (0.0, 0.0),
        keys: std::collections::HashSet::new(),
    };
    event_loop.run_app(&mut app).expect("run app");
}
