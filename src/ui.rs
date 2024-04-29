use std::collections::VecDeque;
use std::sync::atomic::{ AtomicBool, Ordering };
use std::sync::{ Arc, Mutex };
use egui_glfw_gl::egui;
use egui_glfw_gl::egui::{ Color32, Frame, Rgba, SidePanel, Style, TopBottomPanel, Ui };
use gfx_maths::Vec3;
use crate::renderer::MutRenderer;
use crate::ui_defs::chat;
use crate::worldmachine::WorldMachine;

lazy_static! {
    pub static ref SHOW_UI: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    pub static ref SHOW_DEBUG_LOCATION: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
    pub static ref SHOW_FPS: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
    pub static ref SHOW_DEBUG_LOG: Arc<AtomicBool> = Arc::new(AtomicBool::new(true));
    pub static ref DEBUG_LOCATION: Arc<Mutex<Vec3>> = Arc::new(
        Mutex::new(Vec3::new(0.0, 0.0, 0.0))
    );
    pub static ref BOB_T: Arc<Mutex<f32>> = Arc::new(Mutex::new(0.0));
    pub static ref FPS: Arc<Mutex<f32>> = Arc::new(Mutex::new(0.0));
    pub static ref DEBUG_LOG: Arc<Mutex<OnScreenDebugLog>> = Arc::new(
        Mutex::new(OnScreenDebugLog {
            buffer: VecDeque::new(),
        })
    );

    pub static ref INTROSND_INFO: Arc<Mutex<IntrosndInfo>> = Arc::new(
        Mutex::new(IntrosndInfo {
            powered_by_opacity: 0.0,
            powered_by: None,
        })
    );

    pub static ref UNSTABLE_CONNECTION: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    pub static ref DISCONNECTED: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
}

pub struct IntrosndInfo {
    pub powered_by_opacity: f32,
    powered_by: Option<egui::TextureHandle>,
}

pub struct OnScreenDebugLog {
    buffer: VecDeque<String>,
}

impl OnScreenDebugLog {
    const MAX_LOG_SIZE: usize = 10;

    pub fn log(&mut self, message: String) {
        self.buffer.push_back(message);
        if self.buffer.len() > Self::MAX_LOG_SIZE {
            self.buffer.pop_front();
        }
    }

    pub fn get(&mut self) -> Vec<String> {
        self.buffer.iter().cloned().collect()
    }
}

pub fn debug_log(message: impl ToString) {
    DEBUG_LOG.lock().unwrap().log(message.to_string());
}

pub async fn render(renderer: &mut MutRenderer, wm: &mut WorldMachine) {
    if !SHOW_UI.load(Ordering::Relaxed) {
        return;
    }

    let (mut set_name, mut send_message) = (None, None);

    egui::Window
        ::new("chatW")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::LEFT_BOTTOM, egui::Vec2::new(30.0, -100.0))
        .fixed_size(egui::Vec2::new(400.0, 400.0))
        .frame(Frame::dark_canvas(&Style::default()))
        .show(&renderer.backend.egui_context.lock().unwrap(), |ui| {
            let (name, message) = chat::chat(ui, wm);
            set_name = name;
            send_message = message;
        });
    if let Some(name) = set_name {
        wm.set_name(name).await;
    }
    if let Some(message) = send_message {
        wm.send_chat_message(message).await;
    }

    egui::Window
        ::new("right")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::RIGHT_TOP, egui::Vec2::new(-30.0, 10.0))
        .fixed_size(egui::Vec2::new(400.0, 400.0))
        .frame(Frame::dark_canvas(&Style::default()))
        .show(&renderer.backend.egui_context.lock().unwrap(), |ui| {
            if SHOW_DEBUG_LOCATION.load(Ordering::Relaxed) {
                render_debug_location(ui);
            }
            if SHOW_FPS.load(Ordering::Relaxed) {
                render_fps(ui);
            }

            if UNSTABLE_CONNECTION.load(Ordering::Relaxed) && !DISCONNECTED.load(Ordering::Relaxed) {
                let style = ui.style().visuals.widgets.noninteractive.bg_fill.clone();
                ui.style_mut().visuals.widgets.noninteractive.bg_fill = Color32::from(
                    Rgba::from_rgb(0.8, 0.0, 0.0)
                );
                ui.label("BAD connection!");
                ui.style_mut().visuals.widgets.noninteractive.bg_fill = style;
            }

            if DISCONNECTED.load(Ordering::Relaxed) {
                let style = ui.style().visuals.widgets.noninteractive.bg_fill.clone();
                ui.style_mut().visuals.widgets.noninteractive.bg_fill = Color32::from(
                    Rgba::from_rgb(0.8, 0.0, 0.0)
                );
                ui.label("disconnected from the server!");
                ui.style_mut().visuals.widgets.noninteractive.bg_fill = style;
            }
        });

    let egui::FullOutput {
        platform_output,
        repaint_after: _,
        textures_delta,
        shapes,
    } = renderer.backend.egui_context.lock().unwrap().end_frame();

    if !platform_output.copied_text.is_empty() {
        egui_glfw_gl::copy_to_clipboard(
            &mut renderer.backend.input_state.lock().unwrap(),
            platform_output.copied_text
        );
    }

    let clipped_shapes = renderer.backend.egui_context.lock().unwrap().tessellate(shapes);
    renderer.backend.painter
        .lock()
        .unwrap()
        .paint_and_update_textures(1.0, &clipped_shapes, &textures_delta);
}

pub fn init_introsnd(renderer: &mut MutRenderer) {
    SidePanel::left("loading_ctx")
        .frame(Frame::none())
        .show_separator_line(false)
        .resizable(false)
        .show(&renderer.backend.egui_context.lock().unwrap(), |ui| {
            let mut introsnd_info = INTROSND_INFO.lock().unwrap();
            let powered_by_data = crate::textures
                ::load_image("base/textures/ui/poweredby.png")
                .expect("failed to load base/textures/ui/poweredby.png!");
            let powered_by_image = egui::ColorImage::from_rgba_unmultiplied(
                [powered_by_data.dimensions.0 as _, powered_by_data.dimensions.1 as _],
                &powered_by_data.data
            );
            introsnd_info.powered_by.replace(
                ui.ctx().load_texture("powered_by", powered_by_image, egui::TextureOptions::NEAREST)
            );
        });

    let egui::FullOutput {
        platform_output,
        repaint_after: _,
        textures_delta,
        shapes,
    } = renderer.backend.egui_context.lock().unwrap().end_frame();

    if !platform_output.copied_text.is_empty() {
        egui_glfw_gl::copy_to_clipboard(
            &mut renderer.backend.input_state.lock().unwrap(),
            platform_output.copied_text
        );
    }

    let clipped_shapes = renderer.backend.egui_context.lock().unwrap().tessellate(shapes);
    renderer.backend.painter
        .lock()
        .unwrap()
        .paint_and_update_textures(1.0, &clipped_shapes, &textures_delta);
}

pub fn render_introsnd(renderer: &mut MutRenderer) {
    let introsnd_info = INTROSND_INFO.lock().unwrap();

    let window_size = renderer.window_size;
    let poweredby_width = window_size.y / 2.0;
    let poweredby_height = poweredby_width / 2.0;

    let egui::FullOutput {
        platform_output,
        repaint_after: _,
        textures_delta,
        shapes,
    } = renderer.backend.egui_context.lock().unwrap().end_frame();

    if !platform_output.copied_text.is_empty() {
        egui_glfw_gl::copy_to_clipboard(
            &mut renderer.backend.input_state.lock().unwrap(),
            platform_output.copied_text
        );
    }

    let clipped_shapes = renderer.backend.egui_context.lock().unwrap().tessellate(shapes);
    renderer.backend.painter
        .lock()
        .unwrap()
        .paint_and_update_textures(1.0, &clipped_shapes, &textures_delta);
}

fn render_debug_location(ui: &mut Ui) {
    let debug_location = DEBUG_LOCATION.lock().unwrap();
    ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
        ui.label(
            format!("X- {}, Y- {}, Z- {}", debug_location.x, debug_location.y, debug_location.z)
        );
    });
}

fn render_fps(ui: &mut Ui) {
    let fps = FPS.lock().unwrap();
    let bob_t = BOB_T.lock().unwrap();
    ui.with_layout(egui::Layout::top_down(egui::Align::RIGHT), |ui| {
        ui.label(format!("FRAMES: {}", *fps as u32));
        ui.label(format!("BOB_T: {}", *bob_t));
    });
}
