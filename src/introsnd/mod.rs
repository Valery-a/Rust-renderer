use std::ffi::CString;
use std::process;
use std::ptr::null;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant, SystemTime};

use fyrox_sound::buffer::{DataSource, SoundBufferResource};
use fyrox_sound::context::SoundContext;
use fyrox_sound::futures::executor::block_on;
use fyrox_sound::source::{SoundSourceBuilder, Status};
use gfx_maths::{Quaternion, Vec3};
use kira::manager::backend::cpal::CpalBackend;
use kira::manager::AudioManager;
use glad_gl::gl::*;

use crate::animation::Animation;
use crate::helpers::{gen_rainbow, set_shader_if_not_already};
use crate::light::Light;
use crate::renderer::{ht_renderer, RGBA};
use crate::textures::Texture;

pub fn animate(renderer: &mut ht_renderer, sss: &SoundContext) {
    // Set the clear color for the renderer
    renderer.backend.clear_colour.store(RGBA { r: 0, g: 0, b: 0, a: 255 }, Ordering::SeqCst);

    // Load me19-mesh logo model and texture
    if let Err(err) = renderer.load_texture_if_not_already_loaded_synch("me19") {
        eprintln!("Failed to load me19-mesh texture: {}", err);
        process::exit(1);
    }

    if let Err(err) = renderer.load_mesh_if_not_already_loaded_synch("me19") {
        eprintln!("Failed to load me19 mesh: {}", err);
        process::exit(1);
    }

    // Retrieve mesh and texture
    let mut mesh = renderer.meshes.get("me19").expect("Failed to get me19 mesh").clone();
    let mut texture = renderer.textures.get("me19").expect("Failed to get me19-mesh texture").clone();
    let rainbow_shader = renderer.shaders.get("rainbow").unwrap().clone();

    // Set up lighting shader
    unsafe {
        let lighting_shader = *renderer.shaders.get("lighting").unwrap();
        set_shader_if_not_already(renderer, lighting_shader);

        let lighting_shader = renderer.backend.shaders.as_ref().unwrap().get(lighting_shader).unwrap();
        static USE_SHADOWS_C: &'static str = "use_shadows\0";
        let use_shadows_loc = GetUniformLocation(lighting_shader.program, USE_SHADOWS_C.as_ptr() as *const GLchar);
        Uniform1i(use_shadows_loc, 0);
    }

    // Load textures and initialize UI
    let start_time = Instant::now();
    renderer.backend.input_state.lock().unwrap().input.time = Some(start_time.elapsed().as_secs_f64());
    renderer.backend.egui_context.lock().unwrap().begin_frame(renderer.backend.input_state.lock().unwrap().input.take());
    crate::ui::init_introsnd(renderer);

    // Set up lights and sound effects
    let mut light_a = Light {
        position: Vec3::new(0.5, 0.0, 1.6),
        color: Vec3::new(1.0, 1.0, 1.0),
        intensity: 1000.0,
        radius: 10.0,
        casts_shadow: true,
    };

    let mut light_b = Light {
        position: Vec3::new(-0.5, 0.0, 1.6),
        color: Vec3::new(1.0, 1.0, 1.0),
        intensity: 1000.0,
        radius: 10.0,
        casts_shadow: true,
    };

    let introsnd_sfx = match block_on(DataSource::from_file("base/snd/introsnd.wav")) {
        Ok(source) => SoundBufferResource::new_generic(source).unwrap(),
        Err(err) => {
            eprintln!("Failed to load introsnd.wav: {}", err);
            process::exit(1);
        }
    };

    // Set up sound source
    let source = SoundSourceBuilder::new()
        .with_buffer(introsnd_sfx)
        .with_looping(false)
        .with_status(Status::Playing)
        .build()
        .unwrap();

    let source_handle = sss.state().add_source(source);
    debug!("Playing introsnd.wav");
    let time_of_start = SystemTime::now(); // when the animation started
    let mut current_time = SystemTime::now(); // for later
    let rainbow_time = 1032.0; // in milliseconds
    let rainbow_anim = Animation::new(Vec3::new(0.0, 0.0, -10.0), Vec3::new(0.0, 0.25, 2.0), rainbow_time);

    let mut last_time = SystemTime::now();
    loop {
        // Check how long it's been
        current_time = SystemTime::now();
        let time_since_start = current_time.duration_since(time_of_start).expect("Failed to get time since start");
        let time_since_start = time_since_start.as_millis() as f32;

        // Has it been long enough?
        if time_since_start > rainbow_time {
            break;
        }

        // Poll events and manage window
        if current_time.duration_since(last_time).unwrap().as_secs_f32() <= 0.01 {
            poll_and_manage_window(&mut renderer);
            continue;
        } else {
            poll_and_manage_window(&mut renderer);
        }

        // Set color of mesh
        #[cfg(feature = "glfw")]
        unsafe {
            set_shader_if_not_already(renderer, rainbow_shader.clone());
            let colour = gen_rainbow(time_since_start as f64);

            // Get uniform location
            let colour_c = CString::new("i_colour").unwrap();
            let colour_loc = GetUniformLocation(
                renderer.backend.shaders.as_mut().unwrap()[rainbow_shader.clone()].program,
                colour_c.as_ptr(),
            );
            Uniform4f(colour_loc, colour.r as f32 / 255.0, colour.g as f32 / 255.0, colour.b as f32 / 255.0, 1.0);

            Viewport(0, 0, renderer.render_size.x as i32, renderer.render_size.y as i32);
        }

        // Get the point at the current time
        let point = rainbow_anim.get_point_at_time(time_since_start as f64);

        // Set the position of the mesh
        mesh.position = point;

        // Draw the mesh
        mesh.render_basic_lines(renderer, rainbow_shader.clone());

        // Swap buffers
        renderer.introsnd_swap_buffers();
        last_time = current_time;
    }

    // Set up normal animation
    let normal_time = 10000.0 - rainbow_time; // in milliseconds
    let normal_anim = Animation::new(Vec3::new(0.0, 0.25, 2.0), Vec3::new(0.0, 0.35, 1.7), normal_time);

    let opacity_delay = 1000.0; // in milliseconds
    let mut opacity_timer = 0.0;
    let mut intensity_timer = 0.0;
    let mut intensity_downtimer = 0.0;
    let mut dutch = 0.0; // Dutch angle

    let mut last_time = SystemTime::now();
    let start_time = Instant::now();

    loop {
        // Check how long it's been
        current_time = SystemTime::now();
        let time_since_start = current_time.duration_since(time_of_start).expect("Failed to get time since start");
        let time_since_start = time_since_start.as_millis() as f32;

        // Has it been long enough?
        if time_since_start > normal_time {
            break;
        }

        // Poll events and manage window
        if current_time.duration_since(last_time).unwrap().as_secs_f32() <= 0.01 {
            poll_and_manage_window(&mut renderer);
            continue;
        } else {
            poll_and_manage_window(&mut renderer);
        }

        // Update input state and Egui context
        renderer.backend.input_state.lock().unwrap().input.time = Some(start_time.elapsed().as_secs_f64());
        renderer.backend.egui_context.lock().unwrap().begin_frame(renderer.backend.input_state.lock().unwrap().input.take());
        let time_since_start = time_since_start + rainbow_time;

        // Get the point at the current time
        let point = normal_anim.get_point_at_time(time_since_start as f64);

        // Set the position of the mesh
        mesh.position = point;

        // Set the rotation of the mesh
        mesh.rotation = Quaternion::from_euler_angles_zyx(&Vec3::new(0.0, 0.0, dutch));
        dutch += 0.01 * current_time.duration_since(last_time).unwrap().as_secs_f32();

        unsafe {
            Viewport(0, 0, renderer.render_size.x as i32, renderer.render_size.y as i32);
        }

        // Send the lights to the renderer
        renderer.set_lights(vec![light_a, light_b]);

        // Draw the mesh
        mesh.render(renderer, Some(&texture), None, None);
        renderer.clear_all_shadow_buffers();
        mesh.render(renderer, Some(&texture), None, Some((1, 0)));
        mesh.render(renderer, Some(&texture), None, Some((2, 0)));
        renderer.next_light();
        mesh.render(renderer, Some(&texture), None, Some((1, 1)));
        mesh.render(renderer, Some(&texture), None, Some((2, 1)));
        renderer.next_light();

        // Handle opacity and light intensity changes
        handle_opacity_and_intensity(&mut renderer, &mut opacity_timer, &mut intensity_timer, &mut intensity_downtimer, &mut light_a, &mut light_b, &current_time);

        // Update light positions
        light_a.position = mesh.position + Vec3::new(-0.5, 0.0, -1.2);
        light_b.position = mesh.position + Vec3::new(0.5, 0.0, -1.2);

        // Swap buffers
        renderer.introsnd_swap_buffers();
        last_time = current_time;
    }

    // Set up copyright display
    let copyright_time = 2000.0 + normal_time; // in milliseconds
    let mut last_time = SystemTime::now();

    loop {
        // Check how long it's been
        current_time = SystemTime::now();
        let time_since_start = current_time.duration_since(time_of_start).expect("Failed to get time since start");
        let time_since_start = time_since_start.as_millis() as f32;

        // Has it been long enough?
        if time_since_start > copyright_time {
            break;
        }

        // Poll events and manage window
        if current_time.duration_since(last_time).unwrap().as_secs_f32() <= 0.01 {
            poll_and_manage_window(&mut renderer);
            continue;
        } else {
            poll_and_manage_window(&mut renderer);
        }

        // Update input state and Egui context
        renderer.backend.input_state.lock().unwrap().input.time = Some(start_time.elapsed().as_secs_f64());
        renderer.backend.egui_context.lock().unwrap().begin_frame(renderer.backend.input_state.lock().unwrap().input.take());

        unsafe {
            Viewport(0, 0, renderer.render_size.x as i32, renderer.render_size.y as i32);
        }

        // Show copyright information
        crate::ui::introsnd_INFO.lock().unwrap().show_copyright = true;

        // Swap buffers
        renderer.introsnd_swap_buffers();

        last_time = current_time;
    }

    // Remove the sound source
    sss.state().remove_source(source_handle);
}

// Helper function to poll events and manage window
fn poll_and_manage_window(renderer: &mut ht_renderer) {
    renderer.backend.window.lock().unwrap().glfw.poll_events();
    if renderer.manage_window() {
        process::exit(0);
    }
}

// Helper function to handle opacity and light intensity changes
fn handle_opacity_and_intensity(
    renderer: &mut ht_renderer,
    opacity_timer: &mut f32,
    intensity_timer: &mut f32,
    intensity_downtimer: &mut f32,
    light_a: &mut Light,
    light_b: &mut Light,
    current_time: &SystemTime,
) {
    // Increase opacity
    if *opacity_timer < 1000.0 {
        *opacity_timer += current_time.duration_since(renderer.last_frame_time.unwrap()).expect("Failed to get time since last frame").as_millis() as f32;
    } else if crate::ui::introsnd_INFO.lock().unwrap().powered_by_opacity < 1.0 {
        crate::ui::introsnd_INFO.lock().unwrap().powered_by_opacity += current_time.duration_since(renderer.last_frame_time.unwrap()).unwrap().as_secs_f32() / 10.0;
    }

    // Increase light intensity
    if *intensity_downtimer < 100.0 {
        *intensity_downtimer += current_time.duration_since(renderer.last_frame_time.unwrap()).expect("Failed to get time since last frame").as_millis() as f32;
        light_a.intensity = (-*intensity_downtimer / 100.0) * 777.0;
        light_b.intensity = (-*intensity_downtimer / 100.0) * 777.0;
    } else if *intensity_timer < 1000.0 {
        *intensity_timer += current_time.duration_since(renderer.last_frame_time.unwrap()).expect("Failed to get time since last frame").as_millis() as f32;
        light_a.intensity = (*intensity_timer / 1000.0) * 0.2;
        light_b.intensity = (*intensity_timer / 1000.0) * 0.2;
        light_a.color.y = (-*intensity_timer / 1000.0) * 0.01;
        light_b.color.x = (-*intensity_timer / 1000.0) * 0.01;
    }
}
