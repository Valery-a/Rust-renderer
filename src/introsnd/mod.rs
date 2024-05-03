use std::ffi::CString;
use std::process;
use std::sync::atomic::Ordering;
use std::time::{ Instant, SystemTime };
use fyrox_sound::buffer::{ DataSource, SoundBufferResource };
use fyrox_sound::context::SoundContext;
use fyrox_sound::futures::executor::block_on;
use fyrox_sound::source::{ SoundSourceBuilder, Status };
use gfx_maths::{ Quaternion, Vec3 };
use glad_gl::gl::*;
use crate::animation::Animation;
use crate::helpers::{ gen_rainbow, set_shader_if_not_set };
use crate::light::Light;
use crate::renderer::{ RGBA, MutRenderer };

pub fn animate(renderer: &mut MutRenderer, sss: &SoundContext) {
    renderer.backend.clear_colour.store(RGBA { r: 0, g: 0, b: 0, a: 255 }, Ordering::SeqCst);
    renderer.load_texture_if_not_loaded_synch("mut19").expect("failed to load me19-mesh texture");
    renderer.load_mesh_if_not_loaded_synch("mut19").expect("failed to load me19 mesh");

    let mut mesh = renderer.meshes.get("mut19").expect("failed to get me19 mesh").clone();
    let texture = renderer.textures
        .get("mut19")
        .expect("failed to get me19-mesh texture")
        .clone();
    let rainbow_shader = renderer.shaders.get("rainbow").unwrap().clone();

    unsafe {
        let lighting_shader = *renderer.shaders.get("lighting").unwrap();
        set_shader_if_not_set(renderer, lighting_shader);
        let lighting_shader = renderer.backend.shaders
            .as_ref()
            .unwrap()
            .get(lighting_shader)
            .unwrap();
        static USE_SHADOWS_C: &'static str = "use_shadows\0";
        let use_shadows_loc = GetUniformLocation(
            lighting_shader.program,
            USE_SHADOWS_C.as_ptr() as *const GLchar
        );
        Uniform1i(use_shadows_loc, 0);
    }

    let start_time = Instant::now();
    renderer.backend.input_state.lock().unwrap().input.time = Some(
        start_time.elapsed().as_secs_f64()
    );
    renderer.backend.egui_context
        .lock()
        .unwrap()
        .begin_frame(renderer.backend.input_state.lock().unwrap().input.take());
    crate::ui::init_introsnd(renderer);

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

    let introsnd_sfx = SoundBufferResource::new_generic(
        block_on(DataSource::from_file("base/snd/introsnd.wav")).unwrap()
    ).unwrap();

    let source = SoundSourceBuilder::new()
        .with_buffer(introsnd_sfx)
        .with_looping(false)
        .with_status(Status::Playing)
        .build()
        .unwrap();

    let source_handle = sss.state().add_source(source);
    debug!("playing introsnd.wav");
    let time_of_start = SystemTime::now();
    let mut current_time = SystemTime::now();
    let rainbow_time = 1032.0;
    let rainbow_anim = Animation::new(
        Vec3::new(0.0, 0.0, -10.0),
        Vec3::new(0.0, 0.25, 2.0),
        rainbow_time
    );

    let mut last_time = SystemTime::now();
    loop {
        current_time = SystemTime::now();
        let time_since_start = current_time
            .duration_since(time_of_start)
            .expect("failed to get time since start");
        let time_since_start = time_since_start.as_millis() as f32;
        if time_since_start > rainbow_time {
            break;
        }

        if current_time.duration_since(last_time).unwrap().as_secs_f32() <= 0.01 {
            renderer.backend.window.lock().unwrap().glfw.poll_events();
            if renderer.manage_window() {
                process::exit(0);
            }
            continue;
        } else {
            renderer.backend.window.lock().unwrap().glfw.poll_events();
            if renderer.manage_window() {
                process::exit(0);
            }
        }

        #[cfg(feature = "glfw")]
        unsafe {
            set_shader_if_not_set(renderer, rainbow_shader.clone());
            let colour = gen_rainbow(time_since_start as f64);

            let colour_c = CString::new("i_colour").unwrap();
            let colour_loc = GetUniformLocation(
                renderer.backend.shaders.as_mut().unwrap()[rainbow_shader.clone()].program,
                colour_c.as_ptr()
            );
            Uniform4f(
                colour_loc,
                (colour.r as f32) / 255.0,
                (colour.g as f32) / 255.0,
                (colour.b as f32) / 255.0,
                1.0
            );

            Viewport(0, 0, renderer.render_size.x as i32, renderer.render_size.y as i32);
        }

        let point = rainbow_anim.get_point_at_time(time_since_start as f64);
        mesh.position = point;
        mesh.render_basic_lines(renderer, rainbow_shader.clone());
        renderer.introsnd_swap_buffers();

        last_time = current_time;
    }

    let normal_time = 10000.0 - rainbow_time;
    let normal_anim = Animation::new(
        Vec3::new(0.0, 0.25, 2.0),
        Vec3::new(0.0, 0.35, 1.7),
        normal_time
    );

    let opacity_delay = 1000.0;
    let mut opacity_timer = 0.0;
    let mut intensity_timer = 0.0;
    let mut intensity_downtimer = 0.0;

    let mut dutch = 0.0;

    let mut last_time = SystemTime::now();
    let start_time = Instant::now();
    loop {
        current_time = SystemTime::now();
        let time_since_start = current_time
            .duration_since(time_of_start)
            .expect("failed to get time since start");
        let time_since_start = time_since_start.as_millis() as f32;
        if time_since_start > normal_time {
            break;
        }

        if current_time.duration_since(last_time).unwrap().as_secs_f32() <= 0.01 {
            renderer.backend.window.lock().unwrap().glfw.poll_events();
            if renderer.manage_window() {
                process::exit(0);
            }
            continue;
        } else {
            renderer.backend.window.lock().unwrap().glfw.poll_events();
            if renderer.manage_window() {
                process::exit(0);
            }
        }
        renderer.backend.input_state.lock().unwrap().input.time = Some(
            start_time.elapsed().as_secs_f64()
        );
        renderer.backend.egui_context
            .lock()
            .unwrap()
            .begin_frame(renderer.backend.input_state.lock().unwrap().input.take());
        let time_since_start = time_since_start + rainbow_time;

        let point = normal_anim.get_point_at_time(time_since_start as f64);

        mesh.position = point;

        mesh.rotation = Quaternion::from_euler_angles_zyx(&Vec3::new(0.0, 0.0, dutch));
        dutch += 0.01 * current_time.duration_since(last_time).unwrap().as_secs_f32();

        unsafe {
            Viewport(0, 0, renderer.render_size.x as i32, renderer.render_size.y as i32);
        }

        renderer.set_lights(vec![light_a, light_b]);

        mesh.render(renderer, Some(&texture), None, None);
        renderer.clear_every_shadow_buffer();
        mesh.render(renderer, Some(&texture), None, Some((1, 0)));
        mesh.render(renderer, Some(&texture), None, Some((2, 0)));
        renderer.next_light();
        mesh.render(renderer, Some(&texture), None, Some((1, 1)));
        mesh.render(renderer, Some(&texture), None, Some((2, 1)));
        renderer.next_light();

        if opacity_timer < opacity_delay {
            opacity_timer += current_time
                .duration_since(last_time)
                .expect("failed to get time since last frame")
                .as_millis() as f32;
        } else if crate::ui::INTROSND_INFO.lock().unwrap().introsnd_image_holder_opacity < 1.0 {
            crate::ui::INTROSND_INFO.lock().unwrap().introsnd_image_holder_opacity +=
                current_time.duration_since(last_time).unwrap().as_secs_f32() / 10.0;
        }

        if intensity_downtimer < 100.0 {
            intensity_downtimer += current_time
                .duration_since(last_time)
                .expect("failed to get time since last frame")
                .as_millis() as f32;
            light_a.intensity = (-intensity_downtimer / 100.0) * 777.0;
            light_b.intensity = (-intensity_downtimer / 100.0) * 777.0;
        } else if intensity_timer < 1000.0 {
            intensity_timer += current_time
                .duration_since(last_time)
                .expect("failed to get time since last frame")
                .as_millis() as f32;
            light_a.intensity = (intensity_timer / 1000.0) * 0.2;
            light_b.intensity = (intensity_timer / 1000.0) * 0.2;
            light_a.color.y = (-intensity_timer / 1000.0) * 0.01;
            light_b.color.x = (-intensity_timer / 1000.0) * 0.01;
        }

        light_a.position = mesh.position + Vec3::new(-0.5, 0.0, -1.2);
        light_b.position = mesh.position + Vec3::new(0.5, 0.0, -1.2);

        renderer.introsnd_swap_buffers();

        last_time = current_time;
    }
    let display_time = 2000.0 + normal_time;
    let mut last_time = SystemTime::now();

    loop {
        current_time = SystemTime::now();
        let time_since_start = current_time
            .duration_since(time_of_start)
            .expect("failed to get time since start");
        let time_since_start = time_since_start.as_millis() as f32;
        if time_since_start > display_time {
            break;
        }

        if current_time.duration_since(last_time).unwrap().as_secs_f32() <= 0.01 {
            renderer.backend.window.lock().unwrap().glfw.poll_events();
            if renderer.manage_window() {
                process::exit(0);
            }
            continue;
        } else {
            renderer.backend.window.lock().unwrap().glfw.poll_events();
            if renderer.manage_window() {
                process::exit(0);
            }
        }
        renderer.backend.input_state.lock().unwrap().input.time = Some(
            start_time.elapsed().as_secs_f64()
        );
        renderer.backend.egui_context
            .lock()
            .unwrap()
            .begin_frame(renderer.backend.input_state.lock().unwrap().input.take());

        unsafe {
            Viewport(0, 0, renderer.render_size.x as i32, renderer.render_size.y as i32);
        }

        crate::ui::INTROSND_INFO.lock().unwrap().show_image_holder = true;
        renderer.introsnd_swap_buffers();

        last_time = current_time;
    }

    sss.state().remove_source(source_handle);
}
