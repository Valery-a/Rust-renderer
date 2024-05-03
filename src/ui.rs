use std::collections::VecDeque;
use std::sync::atomic::{ AtomicBool, Ordering };
use std::sync::{ Arc, Mutex };
use sysinfo::{System};
use egui_glfw_gl::egui::{self, RichText};
use egui_glfw_gl::egui::{
    Color32,
    Frame,
    Rgba,
    SidePanel,
    Style,
    TopBottomPanel,
    Ui,
};
use gfx_maths::Vec3;
use crate::renderer::MutRenderer;
use crate::worldmachine::player::Player;
use crate::ui_defs::chat;
use crate::worldmachine::WorldMachine;
use std::time::{Instant, Duration};
use once_cell::sync::Lazy;

use std::collections::HashMap;

#[derive(Default)]
struct TrieNode {
    children: HashMap<char, TrieNode>,
    is_end_of_word: bool,
}

impl TrieNode {
    fn new() -> Self {
        TrieNode {
            children: HashMap::new(),
            is_end_of_word: false,
        }
    }
}

pub struct Trie {
    root: TrieNode,
}

impl Trie {
    pub fn new() -> Self {
        Trie {
            root: TrieNode::new(),
        }
    }

    pub fn insert(&mut self, word: &str) {
        let mut current = &mut self.root;
        for ch in word.chars() {
            current = current.children.entry(ch).or_insert(TrieNode::new());
        }
        current.is_end_of_word = true;
    }

    pub fn suggest_completions(&self, prefix: &str) -> Vec<String> {
        let mut current = &self.root;
        for ch in prefix.chars() {
            if let Some(node) = current.children.get(&ch) {
                current = node;
            } else {
                return vec![];
            }
        }
        let mut completions = Vec::new();
        self.collect_completions(prefix, current, &mut completions);
        completions
    }

    fn collect_completions(&self, prefix: &str, node: &TrieNode, completions: &mut Vec<String>) {
        if node.is_end_of_word {
            completions.push(prefix.to_owned());
        }
        for (ch, child) in &node.children {
            self.collect_completions(&(prefix.to_owned() + &ch.to_string()), child, completions);
        }
    }
}


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

    pub static ref INTROSND_INFO: Arc<Mutex<introsndInfo>> = Arc::new(
        Mutex::new(introsndInfo {
            powered_by_opacity: 0.0,
            show_copyright: false,
            powered_by: None,
            copyright: None,
        })
    );

    static ref COMMAND_TRIE: Trie = {
        let mut trie = Trie::new();
        trie.insert("increase_speed");
        // Add more commands here as needed
        trie
    };

    pub static ref UNSTABLE_CONNECTION: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    pub static ref DISCONNECTED: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
}

static SYS_INFO: Lazy<Mutex<(System, Instant, f32)>> = Lazy::new(|| {
    let sys = System::new_all();
    Mutex::new((sys, Instant::now(), 0.0))
});

pub struct introsndInfo {
    pub powered_by_opacity: f32,
    pub show_copyright: bool,
    powered_by: Option<egui::TextureHandle>,
    copyright: Option<egui::TextureHandle>,
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

enum CommandResult {
    Success,
    Failure(String),
}

pub async fn render(renderer: &mut MutRenderer, wm: &mut WorldMachine, player: &mut Player) {
    if !SHOW_UI.load(Ordering::Relaxed) {
        return;
    }

    let (mut set_name, mut send_message) = (None, None);

    egui::Window
        ::new("chat")
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

    egui::Window::new("debug")
        .title_bar(true)
        .resizable(true)
        .collapsible(true)
        .anchor(egui::Align2::CENTER_TOP, egui::Vec2::new(0.0, 10.0))
        .default_width(400.0)
        .frame(Frame::none().fill(Color32::from_rgb(25, 25, 25))) 
        .show(&renderer.backend.egui_context.lock().unwrap(), |ui| {
            ui.vertical_centered(|ui| {
                ui.heading("Information");
            });
            ui.separator();
            ui.horizontal(|ui| {
                ui.label("Status:");
                ui.colored_label(Color32::LIGHT_GREEN, "Active");
            });
            render_debug_location(ui);
            render_fps(ui);
            render_memory_usage(ui);
            render_command_panel(ui, wm, player);
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


fn render_fps(ui: &mut Ui) {
    let fps = FPS.lock().unwrap();
    let label_text = format!("FPS: {}", *fps as u32);
    ui.colored_label(Color32::GOLD, label_text);
}

fn render_memory_usage(ui: &mut Ui) {
    let memory_usage = get_memory_usage();
    let label_text = RichText::new(format!("Memory Usage: {:.2} MB", memory_usage)).color(Color32::from_rgb(255, 165, 0)); // Orange color for memory usage
    ui.add(egui::Label::new(label_text));
}

fn get_memory_usage() -> f32 {
    let mut sys_info = SYS_INFO.lock().unwrap();
    let (ref mut sys, ref mut last_updated, ref mut last_value) = *sys_info;

    if last_updated.elapsed() > Duration::from_secs(5) {
        sys.refresh_memory();
        *last_value = sys.used_memory() as f32 / 1024.0;
        *last_updated = Instant::now();
    }

    *last_value
}

fn render_debug_location(ui: &mut Ui) {
    let debug_location = DEBUG_LOCATION.lock().unwrap();
    ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
        ui.label(
            format!("x: {}, y: {}, z: {}", debug_location.x, debug_location.y, debug_location.z)
        );
    });
}

pub fn render_introsnd(renderer: &mut MutRenderer) {
    let mut introsnd_info = INTROSND_INFO.lock().unwrap();

    let window_size = renderer.window_size;
    let poweredby_width = window_size.y / 2.0;
    let poweredby_height = poweredby_width / 2.0;

    if !introsnd_info.show_copyright {
        TopBottomPanel::bottom("powered_by")
            .frame(Frame::none())
            .show_separator_line(false)
            .resizable(false)
            .show(&renderer.backend.egui_context.lock().unwrap(), |ui| {
                if let Some(poweredby) = &introsnd_info.powered_by {
                    let image = egui::Image::new(poweredby, [poweredby_width, poweredby_height]);
                    let tint = Rgba::from_white_alpha(introsnd_info.powered_by_opacity);
                    let image = image.tint(tint);
                    ui.add(image);
                }
            });
    } else {
        TopBottomPanel::bottom("copyright")
            .frame(Frame::none())
            .show_separator_line(false)
            .resizable(false)
            .show(&renderer.backend.egui_context.lock().unwrap(), |ui| {
                if let Some(copyright) = &introsnd_info.copyright {
                    let image = egui::Image::new(copyright, [window_size.x, window_size.y]);
                    ui.add(image);
                }
            });
    }

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
            let copyright_data = crate::textures
                ::load_image("base/textures/ui/developedby.png")
                .expect("failed to load base/textures/ui/developedby.png!");
            let powered_by_image = egui::ColorImage::from_rgba_unmultiplied(
                [powered_by_data.dimensions.0 as _, powered_by_data.dimensions.1 as _],
                &powered_by_data.data
            );
            let copyright_image = egui::ColorImage::from_rgba_unmultiplied(
                [copyright_data.dimensions.0 as _, copyright_data.dimensions.1 as _],
                &copyright_data.data
            );
            introsnd_info.powered_by.replace(
                ui.ctx().load_texture("powered_by", powered_by_image, egui::TextureOptions::NEAREST)
            );
            introsnd_info.copyright.replace(
                ui.ctx().load_texture("copyright", copyright_image, egui::TextureOptions::NEAREST)
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

fn render_command_panel(ui: &mut Ui, wm: &mut WorldMachine, player: &mut Player) {
    let mut command_result = None;
    let mut suggestions = Vec::new(); // Suggestions for auto-completion

    if SHOW_UI.load(Ordering::Relaxed) {
        egui::Window::new("Command Panel")
            .title_bar(true)
            .resizable(true)
            .default_size(egui::Vec2::new(600.0, 300.0))
            .frame(Frame::none().fill(Color32::from_rgb(25, 25, 120))) 
            .hscroll(true) // Enable scrolling if commands overflow
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    ui.label("Command:");
                    ui.add(egui::TextEdit::singleline(&mut wm.command)
                        .hint_text("Type a command here...")
                        .desired_width(200.0)); // Set desired width for better layout

                    if ui.button("Execute").clicked() {
                        command_result = Some(handle_command(&wm.command, player));
                    }
                    if ui.button("Clear").clicked() {
                        wm.command.clear();
                    }
                });

                // Auto-completion logic
                if !wm.command.is_empty() {
                    // Get auto-completion suggestions based on the current command prefix
                    suggestions = COMMAND_TRIE.suggest_completions(&wm.command);
                }

                if !suggestions.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label("Suggestions:");
                        ui.vertical(|ui| {
                            // Display up to 5 auto-completion suggestions
                            for suggestion in suggestions.iter().take(5) {
                                ui.label(suggestion);
                            }
                        });
                    });
                }

                if let Some(result) = command_result {
                    ui.separator(); // Add a separator for better visual separation
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::Vec2::new(10.0, 0.0); // Adjust button spacing
                        ui.label("Command Feedback:"); // Add label for feedback
                        match result {
                            CommandResult::Success => {
                                ui.colored_label(Color32::GREEN, "Success");
                            }
                            CommandResult::Failure(reason) => {
                                ui.colored_label(Color32::RED, &format!("Failed: {}", reason));
                            }
                        }
                    });
                }
            });
    }
}

fn handle_command(command: &str, player: &mut Player) -> CommandResult {
    match command {
        "increase_speed" => {
            player.increase_speed();
            CommandResult::Success
        }
        _ => CommandResult::Failure(format!("Unknown command: {}", command)),
    }
}
