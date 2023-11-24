// array of 2d points showing the path that the cool looking rainbow line will take to draw the logo
use std::ffi::CString;
use std::process;
use std::ptr::null;
use std::sync::atomic::Ordering;
use std::time::{Instant, SystemTime};

use fyrox_sound::buffer::{DataSource, SoundBufferResource};
use fyrox_sound::context::SoundContext;
use fyrox_sound::futures::executor::block_on;
use fyrox_sound::source::{SoundSourceBuilder, Status};
use gfx_maths::{Quaternion, Vec2, Vec3};
use kira::manager::AudioManager;
use kira::manager::backend::cpal::CpalBackend;
use kira::sound::static_sound::{StaticSoundData, StaticSoundSettings};
use glad_gl::gl::*;
use crate::animation::Animation;
use crate::helpers::{gen_rainbow, set_shader_if_not_already};
use crate::light::Light;
use crate::renderer::{RGBA, ht_renderer};
use crate::textures::Texture;

pub fn animate(renderer: &mut ht_renderer, sss: &SoundContext) {
    // Set the clear color for the renderer
    renderer.backend.clear_colour.store(RGBA { r: 0, g: 0, b: 0, a: 255 }, Ordering::SeqCst);

    // Load me19-mesh logo model and texture
    renderer.load_texture_if_not_already_loaded_synch("me19").expect("failed to load me19-mesh texture");
    renderer.load_mesh_if_not_already_loaded_synch("me19").expect("failed to load me19 mesh");

    // Retrieve mesh and texture
    let mut mesh = renderer.meshes.get("me19").expect("failed to get me19 mesh").clone();
    let mut texture = renderer.textures.get("me19").expect("failed to get me19-mesh texture").clone();
    let rainbow_shader = renderer.shaders.get("rainbow").unwrap().clone();

    // Set up lighting shader
    unsafe {
        let lighting_shader = *renderer.shaders.get("lighting").unwrap();
        set_shader_if_not_already(renderer, lighting_shader);
        let lighting_shader = renderer.backend.shaders.as_ref().unwrap().get(lighting_shader).unwrap();
        static use_shadows_c: &'static str = "use_shadows\0";
        let use_shadows_loc = GetUniformLocation(lighting_shader.program, use_shadows_c.as_ptr() as *const GLchar);
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

    let mut introsnd_sfx = SoundBufferResource::new_generic(block_on(DataSource::from_file("base/snd/introsnd.wav")).unwrap()).unwrap();

    // Set up sound source
    let source = SoundSourceBuilder::new()
        .with_buffer(introsnd_sfx)
        .with_looping(false)
        .with_status(Status::Playing)
        .build().unwrap();

    let source_handle = sss.state().add_source(source);
    debug!("playing introsnd.wav");
    let time_of_start = SystemTime::now(); // when the animation started
    let mut current_time = SystemTime::now(); // for later
    let rainbow_time = 1032.0; // in milliseconds
    let rainbow_anim = Animation::new(Vec3::new(0.0, 0.0, -10.0), Vec3::new(0.0, 0.25, 2.0), rainbow_time);

    let mut last_time = SystemTime::now();
    loop {
        // check how long it's been
        current_time = SystemTime::now();
        let time_since_start = current_time.duration_since(time_of_start).expect("failed to get time since start");
        let time_since_start = time_since_start.as_millis() as f32;
        // has it been long enough?
        if time_since_start > rainbow_time {
            break;
        }

        if current_time.duration_since(last_time).unwrap().as_secs_f32() <= 0.01 {
            // poll events
            renderer.backend.window.lock().unwrap().glfw.poll_events();
            if renderer.manage_window() {
                process::exit(0);
            }
            continue;
        } else {
            // poll events
            renderer.backend.window.lock().unwrap().glfw.poll_events();
            if renderer.manage_window() {
                process::exit(0);
            }
        }

        // set colour of mesh
        #[cfg(feature = "glfw")]
        unsafe {
            set_shader_if_not_already(renderer, rainbow_shader.clone());
            let colour = gen_rainbow(time_since_start as f64);
            // get uniform location

            let colour_c = CString::new("i_colour").unwrap();
            let colour_loc = GetUniformLocation(renderer.backend.shaders.as_mut().unwrap()[rainbow_shader.clone()].program, colour_c.as_ptr());
            Uniform4f(colour_loc, colour.r as f32 / 255.0, colour.g as f32 / 255.0, colour.b as f32 / 255.0, 1.0);

            Viewport(0, 0, renderer.render_size.x as i32, renderer.render_size.y as i32);
        }

        // get the point at the current time
        let point = rainbow_anim.get_point_at_time(time_since_start as f64);
        // set the position of the mesh
        mesh.position = point;
        // draw the mesh
        mesh.render_basic_lines(renderer, rainbow_shader.clone());
        // swap buffers
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

    let mut dutch = 0.0; // dutch angle or whatever this probably isn't the correct usage of that word

    let mut last_time = SystemTime::now();
    let start_time = Instant::now();
    loop {
        // check how long it's been
        current_time = SystemTime::now();
        let time_since_start = current_time.duration_since(time_of_start).expect("failed to get time since start");
        let time_since_start = time_since_start.as_millis() as f32;
        // has it been long enough?
        if time_since_start > normal_time {
            break;
        }

        if current_time.duration_since(last_time).unwrap().as_secs_f32() <= 0.01 {
            // poll events
            renderer.backend.window.lock().unwrap().glfw.poll_events();
            if renderer.manage_window() {
                process::exit(0);
            }
            continue;
        } else {
            // poll events
            renderer.backend.window.lock().unwrap().glfw.poll_events();
            if renderer.manage_window() {
                process::exit(0);
            }
        }
        renderer.backend.input_state.lock().unwrap().input.time = Some(start_time.elapsed().as_secs_f64());
        renderer.backend.egui_context.lock().unwrap().begin_frame(renderer.backend.input_state.lock().unwrap().input.take());
        let time_since_start =  time_since_start + rainbow_time;
        // get the point at the current time
        let point = normal_anim.get_point_at_time(time_since_start as f64);
        // set the position of the mesh
        mesh.position = point;
        // set the rotation of the mesh
        mesh.rotation = Quaternion::from_euler_angles_zyx(&Vec3::new(0.0, 0.0, dutch));
        dutch += 0.01 * current_time.duration_since(last_time).unwrap().as_secs_f32();

        unsafe {
            Viewport(0, 0, renderer.render_size.x as i32, renderer.render_size.y as i32);
        }

        // send the lights to the renderer
        renderer.set_lights(vec![light_a, light_b]);

        // draw the mesh
        mesh.render(renderer, Some(&texture), None, None);
        renderer.clear_all_shadow_buffers();
        mesh.render(renderer, Some(&texture), None, Some((1, 0)));
        mesh.render(renderer, Some(&texture), None, Some((2, 0)));
        renderer.next_light();
        mesh.render(renderer, Some(&texture), None, Some((1, 1)));
        mesh.render(renderer, Some(&texture), None, Some((2, 1)));
        renderer.next_light();

        if opacity_timer < opacity_delay {
            opacity_timer += current_time.duration_since(last_time).expect("failed to get time since last frame").as_millis() as f32;
        } else if crate::ui::introsnd_INFO.lock().unwrap().powered_by_opacity < 1.0 {
            crate::ui::introsnd_INFO.lock().unwrap().powered_by_opacity += current_time.duration_since(last_time).unwrap().as_secs_f32() / 10.0;
        }

        // increase light intensity
        if intensity_downtimer < 100.0 {
            intensity_downtimer += current_time.duration_since(last_time).expect("failed to get time since last frame").as_millis() as f32;
            light_a.intensity = (-intensity_downtimer / 100.0) * 777.0;
            light_b.intensity = (-intensity_downtimer / 100.0) * 777.0;
        } else if intensity_timer < 1000.0 {
            intensity_timer += current_time.duration_since(last_time).expect("failed to get time since last frame").as_millis() as f32;
            light_a.intensity = (intensity_timer / 1000.0) * 0.2;
            light_b.intensity = (intensity_timer / 1000.0) * 0.2;
            light_a.color.y = (-intensity_timer / 1000.0) * 0.01;
            light_b.color.x = (-intensity_timer / 1000.0) * 0.01;
        }

        light_a.position = mesh.position + Vec3::new(-0.5, 0.0, -1.2);
        light_b.position = mesh.position + Vec3::new(0.5, 0.0, -1.2);

        // swap buffers
        renderer.introsnd_swap_buffers();

        last_time = current_time;
    }

    // Set up copyright display
    let copyright_time = 2000.0 + normal_time; // in milliseconds
    let mut last_time = SystemTime::now();

    loop {
        // check how long it's been
        current_time = SystemTime::now();
        let time_since_start = current_time.duration_since(time_of_start).expect("failed to get time since start");
        let time_since_start = time_since_start.as_millis() as f32;
        if time_since_start > copyright_time {
            break;
        }

        if current_time.duration_since(last_time).unwrap().as_secs_f32() <= 0.01 {
            // poll events
            renderer.backend.window.lock().unwrap().glfw.poll_events();
            if renderer.manage_window() {
                process::exit(0);
            }
            continue;
        } else {
            // poll events
            renderer.backend.window.lock().unwrap().glfw.poll_events();
            if renderer.manage_window() {
                process::exit(0);
            }
        }
        renderer.backend.input_state.lock().unwrap().input.time = Some(start_time.elapsed().as_secs_f64());
        renderer.backend.egui_context.lock().unwrap().begin_frame(renderer.backend.input_state.lock().unwrap().input.take());

        unsafe {
            Viewport(0, 0, renderer.render_size.x as i32, renderer.render_size.y as i32);
        }

        crate::ui::introsnd_INFO.lock().unwrap().show_copyright = true;
        // swap buffers
        renderer.introsnd_swap_buffers();

        last_time = current_time;
    }

    // Remove the sound source
    sss.state().remove_source(source_handle);
}
