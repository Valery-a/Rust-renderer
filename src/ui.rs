use crate::firebase;
use crate::firebase::db_operations::User;
use crate::renderer::MutRenderer;
use crate::ui_defs::chat;
use crate::worldmachine::player::Player;
use crate::worldmachine::WorldMachine;
use egui_glfw_gl::egui::{ self, RichText };
use egui_glfw_gl::egui::{ Color32, Frame, Rgba, SidePanel, Style, TopBottomPanel, Ui };
use gfx_maths::Vec3;
use once_cell::sync::Lazy;
use std::collections::VecDeque;
use std::sync::atomic::{ AtomicBool, Ordering };
use std::sync::{ Arc, Mutex };
use std::time::{ Duration, Instant };
use sysinfo::System;

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
    pub static ref INTROSND_INFO: Arc<Mutex<IntrosndInfo>> = Arc::new(
        Mutex::new(IntrosndInfo {
            introsnd_image_holder_opacity: 0.0,
            show_image_holder: false,
            introsnd_image_holder: None,
            image_holder: None,
        })
    );
    static ref COMMAND_TRIE: Trie = {
        let mut trie = Trie::new();
        trie.insert("increase_speed");
        trie
    };
    pub static ref UNSTABLE_CONNECTION: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
    pub static ref DISCONNECTED: Arc<AtomicBool> = Arc::new(AtomicBool::new(false));
}

static SYS_INFO: Lazy<Mutex<(System, Instant, f32)>> = Lazy::new(|| {
    let sys = System::new_all();
    Mutex::new((sys, Instant::now(), 0.0))
});

pub struct IntrosndInfo {
    pub introsnd_image_holder_opacity: f32,
    pub show_image_holder: bool,
    introsnd_image_holder: Option<egui::TextureHandle>,
    image_holder: Option<egui::TextureHandle>,
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

    egui::Window
        ::new("debug")
        .title_bar(true)
        .resizable(true)
        .collapsible(true)
        .anchor(egui::Align2::CENTER_TOP, egui::Vec2::new(30.0, 10.0))
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
            firebase_admin_panel(ui);
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
    let label_text = RichText::new(format!("Memory Usage: {:.2} MB", memory_usage)).color(
        Color32::from_rgb(255, 165, 0)
    );
    ui.add(egui::Label::new(label_text));
}

fn get_memory_usage() -> f32 {
    let mut sys_info = SYS_INFO.lock().unwrap();
    let (ref mut sys, ref mut last_updated, ref mut last_value) = *sys_info;

    if last_updated.elapsed() > Duration::from_secs(5) {
        sys.refresh_memory();
        *last_value = (sys.used_memory() as f32) / 1024.0;
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
    let introsnd_info = INTROSND_INFO.lock().unwrap();

    let window_size = renderer.window_size;
    let poweredby_width = window_size.y / 2.0;
    let poweredby_height = poweredby_width / 2.0;

    if !introsnd_info.show_image_holder {
        TopBottomPanel::bottom("introsnd_image_holder")
            .frame(Frame::none())
            .show_separator_line(false)
            .resizable(false)
            .show(&renderer.backend.egui_context.lock().unwrap(), |ui| {
                if let Some(poweredby) = &introsnd_info.introsnd_image_holder {
                    let image = egui::Image::new(poweredby, [poweredby_width, poweredby_height]);
                    let tint = Rgba::from_white_alpha(introsnd_info.introsnd_image_holder_opacity);
                    let image = image.tint(tint);
                    ui.add(image);
                }
            });
    } else {
        TopBottomPanel::bottom("image_holder")
            .frame(Frame::none())
            .show_separator_line(false)
            .resizable(false)
            .show(&renderer.backend.egui_context.lock().unwrap(), |ui| {
                if let Some(image_holder) = &introsnd_info.image_holder {
                    let image = egui::Image::new(image_holder, [window_size.x, window_size.y]);
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
            let introsnd_image_holder_data = crate::textures
                ::load_image("base/textures/ui/poweredby.png")
                .expect("failed to load base/textures/ui/poweredby.png!");
            let image_holder_data = crate::textures
                ::load_image("base/textures/ui/developedby.png")
                .expect("failed to load base/textures/ui/developedby.png!");
            let introsnd_image_holder_image = egui::ColorImage::from_rgba_unmultiplied(
                [
                    introsnd_image_holder_data.dimensions.0 as _,
                    introsnd_image_holder_data.dimensions.1 as _,
                ],
                &introsnd_image_holder_data.data
            );
            let image_holder_image = egui::ColorImage::from_rgba_unmultiplied(
                [image_holder_data.dimensions.0 as _, image_holder_data.dimensions.1 as _],
                &image_holder_data.data
            );
            introsnd_info.introsnd_image_holder.replace(
                ui
                    .ctx()
                    .load_texture(
                        "introsnd_image_holder",
                        introsnd_image_holder_image,
                        egui::TextureOptions::NEAREST
                    )
            );
            introsnd_info.image_holder.replace(
                ui
                    .ctx()
                    .load_texture("image_holder", image_holder_image, egui::TextureOptions::NEAREST)
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
    let mut suggestions = Vec::new();

    if SHOW_UI.load(Ordering::Relaxed) {
        egui::Window
            ::new("Command Panel")
            .title_bar(true)
            .resizable(true)
            .default_size(egui::Vec2::new(600.0, 300.0))
            .frame(Frame::none().fill(Color32::from_rgb(25, 25, 120)))
            .hscroll(true)
            .show(ui.ctx(), |ui| {
                ui.horizontal(|ui| {
                    ui.label("Command:");
                    ui.add(
                        egui::TextEdit
                            ::singleline(&mut wm.command)
                            .hint_text("...")
                            .desired_width(200.0)
                    );
                    if ui.button("Execute").clicked() {
                        command_result = Some(handle_command(&wm.command, player));
                    }
                    if ui.button("Clear").clicked() {
                        wm.command.clear();
                    }
                });
                if !wm.command.is_empty() {
                    suggestions = COMMAND_TRIE.suggest_completions(&wm.command);
                }
                if !suggestions.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label("Suggestions:");
                        ui.vertical(|ui| {
                            for suggestion in suggestions.iter().take(5) {
                                if ui.button(suggestion).clicked() {
                                    wm.command = suggestion.clone();
                                }
                            }
                        });
                    });
                }
                if let Some(result) = command_result {
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = egui::Vec2::new(10.0, 0.0);
                        ui.label("Command Feedback:");
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

static USER_INPUT: Lazy<Mutex<(String, u32, String, String)>> = Lazy::new(|| Mutex::new((String::new(), 0, String::new(), String::new())));

fn firebase_admin_panel(ui: &mut Ui) {
    let mut user_input = USER_INPUT.lock().unwrap();
    let (ref mut name, ref mut age, ref mut email, ref mut user_id) = *user_input;
    let firebase = firebase::db_initialize::initialize_firebase();

    if SHOW_UI.load(Ordering::Relaxed) {
        egui::Window::new("Firebase Admin Panel")
            .resizable(true)
            .collapsible(true)
            .default_size(egui::vec2(500.0, 400.0))
            .show(ui.ctx(), |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading("Manage Users");
                    ui.separator();

                    // Input fields for user data
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(name);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Age:");
                        ui.add(egui::DragValue::new(age));
                    });
                    ui.horizontal(|ui| {
                        ui.label("Email:");
                        ui.text_edit_singleline(email);
                    });
                    ui.horizontal(|ui| {
                        ui.label("User ID (for Get, Update, Delete):");
                        ui.text_edit_singleline(user_id);
                    });

                    // Buttons for CRUD operations
                    ui.horizontal(|ui| {
                        if ui.button("Add User").clicked() {
                            let user = User { name: name.clone(), age: *age, email: email.clone() };
                            let firebase_clone = firebase.clone();
                            let user_id_clone = user_id.clone(); // Cloning user_id
                            tokio::spawn(async move {
                                firebase::db_operations::set_user(&firebase_clone, &user).await;
                            });
                        }
                        if ui.button("Get User").clicked() {
                            let firebase_clone = firebase.clone();
                            let user_id_clone = user_id.clone(); // Cloning user_id
                            tokio::spawn(async move {
                                let user = firebase::db_operations::get_user(&firebase_clone, &user_id_clone).await;
                                // Display the user data as JSON
                                println!("User Retrieved: {:?}", serde_json::to_string(&user));
                            });
                        }
                        if ui.button("Update User").clicked() {
                            let user = User { name: name.clone(), age: *age, email: email.clone() };
                            let firebase_clone = firebase.clone();
                            let user_id_clone = user_id.clone(); // Cloning user_id
                            tokio::spawn(async move {
                                firebase::db_operations::update_user(&firebase_clone, &user_id_clone, &user).await;
                            });
                        }
                        if ui.button("Delete User").clicked() {
                            let firebase_clone = firebase.clone();
                            let user_id_clone = user_id.clone(); // Cloning user_id
                            tokio::spawn(async move {
                                firebase::db_operations::delete_user(&firebase_clone, &user_id_clone).await;
                            });
                        }
                    });
                });
            });
    }
}
