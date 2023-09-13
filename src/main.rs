extern crate generic_array;
extern crate typenum;
extern crate alga;
extern crate libc;
extern crate ansi_term;
extern crate time;
extern crate rand;
extern crate noise;
extern crate glfw;
extern crate glad_gl;
extern crate glad_vulkan;
#[cfg(target_os = "macos")]
extern crate accelerate_src;
#[cfg(not(target_os = "macos"))]
extern crate openblas_src;
extern crate lapack_sys;

#[cfg(feature = "vulkan")]
extern crate vulkano;
#[cfg(feature = "vulkan")]
extern crate vulkano_shaders;
#[cfg(feature = "vulkan")]
mod vulkan_raytracer;

use glad_gl::gl::GL_TEXTURE_2D;
use std::vec::Vec as Vector;

mod graphics;
mod graphics_util;
mod renderer;

mod math;
#[macro_use]
mod matrix;

//#[macro_use]
//mod matrix_const;

mod extraction;

use extraction::uniform_manifold_dc::*;
//use extraction::adaptive_manifold_dc;

use noise::Perlin;
use graphics::*;
use renderer::*;
use math::*;
use matrix::*;
use glad_gl::gl;


fn handle_input(_glfw: &mut glfw::Glfw, win: &mut glfw::Window, dt_ns: u64, camera: &mut Cam) {
    if win.get_key(glfw::Key::Escape) == glfw::Action::Press{
        win.set_should_close(true);
    }else{
        if win.get_key(glfw::Key::Tab) == glfw::Action::Press{
            //debug

            let (w, h) = win.get_size();

            println!("[debug] window size: ({}, {})", w, h);

        }
    }
    let dt_s : f32 = dt_ns as f32 / 1000000000.0;

    if win.get_key(glfw::Key::W) == glfw::Action::Press{
        camera.pos += camera.look * dt_s as f32;

    }

    if win.get_key(glfw::Key::S) == glfw::Action::Press{
        camera.pos -= camera.look * dt_s as f32;
    }

    if win.get_key(glfw::Key::A) == glfw::Action::Press{
        let right = camera.look.cross(camera.up);

        camera.pos -= right * dt_s as f32;
    }

    if win.get_key(glfw::Key::D) == glfw::Action::Press{
        let right = camera.look.cross(camera.up);

        camera.pos += right * dt_s as f32;
    }

    if win.get_key(glfw::Key::Space) == glfw::Action::Press{

        camera.pos += camera.up * dt_s as f32;
    }

    if win.get_key(glfw::Key::LeftShift) == glfw::Action::Press{

        camera.pos -= camera.up * dt_s as f32;
    }

    if win.get_key(glfw::Key::Left) == glfw::Action::Press{

        let mat = rot_mat3(camera.up, std::f32::consts::PI * dt_s / 2.0);
        camera.look = (mat * camera.look).normalize();
    }
    if win.get_key(glfw::Key::Right) == glfw::Action::Press{

        let mat = rot_mat3(camera.up, -std::f32::consts::PI * dt_s / 2.0);
        camera.look = (mat * camera.look).normalize();
    }
    if win.get_key(glfw::Key::Kp0) == glfw::Action::Press{

        let mat = rot_mat3(camera.look, std::f32::consts::PI * dt_s / 2.0);
        camera.up = (mat * camera.up).normalize();
    }
    if win.get_key(glfw::Key::KpDecimal) == glfw::Action::Press{

        let mat = rot_mat3(camera.look, -std::f32::consts::PI * dt_s / 2.0);
        camera.up = (mat * camera.up).normalize();
    }
    if win.get_key(glfw::Key::Up) == glfw::Action::Press{
        let right = camera.look.cross(camera.up);
        let mat = rot_mat3(right, std::f32::consts::PI * dt_s / 2.0);
        camera.look = (mat * camera.look).normalize();
        camera.up = (mat * camera.up).normalize();
    }
    if win.get_key(glfw::Key::Down) == glfw::Action::Press{
        let right = camera.look.cross(camera.up);
        let mat = rot_mat3(right, -std::f32::consts::PI * dt_s / 2.0);
        camera.look = (mat * camera.look).normalize();
        camera.up = (mat * camera.up).normalize();
    }

}


fn run(){
    let def_width: u32 = 800;
    let def_height: u32 = 600;
    let title = "bomba";
    let cam = renderer::Cam{look : Vec3::new(0.0, 0.0, -1.0), up : Vec3::new(0.0, 1.0, 0.0), pos : Vec3::new(0.0, 0.0, 0.0)};
    let mut renderer = Renderer::new(cam);
    renderer.init(def_width, def_height, title);
    renderer.set_framebuffer_size_callback(|w, h| println!("new dims {} {}", w, h));


    let _block_size : f32 = 0.125;//2.0;
    let _chunk_size : usize = 128;//*2;



    //UNIFORM MANIFOLD DC
    let perlin = Perlin::new();
    let noise = noise_f32(perlin, Cube{center : Vec3::new(1.0,-1.0,1.0), extent : 3.5} );//perlin.get([p.x,p.y,p.z])  ;
    //let den4 = union3(den3, mk_half_space_pos(Plane{point : Vec3::new(0.0, 2.0, -4.0), normal : Vec3::new(1.0, 1.0, 0.0).normalize()}));
    //let den = f;
    //construct_grid(&den4, Vec3::new(-3.0, -3.0, -8.0), BLOCK_SIZE, CHUNK_SIZE, 8, &mut renderer_tr_light, &mut renderer_lines);

    let test_sphere = Sphere{center : Vec3::new(2.7, -1.0, 0.0), rad : 0.1};
    let test_sphere2 = Sphere{center : Vec3::new(2.7, 3.0, 0.0), rad : 0.1};
    let test_sphere3 = Sphere{center : Vec3::new(2.7, 1.0, 2.7), rad : 0.1};
    let ts1 = mk_sphere(test_sphere);
    let ts2 = mk_sphere(test_sphere2);
    let ts22 = mk_sphere(test_sphere3);
    let ts3 = difference3(ts1, ts2);
    let _ts4 = difference3(ts3, ts22);
    let a1 = union3(difference3(noise, ts1), mk_obb(Vec3::new(-1.0, -1.0, 0.0), Vec3::new(1.0, -1.0, 0.0).normalize(), Vec3::new(1.0, 1.0, 0.5).normalize(), Vec3::new(1.0, 0.5, 0.2)));
    //add_sphere_color(&mut renderer_tr_light, &test_sphere, 100, 100, Vec3::new(1.0, 1.0, 1.0));
    //construct_grid(&ts4, Vec3::new(-0.5, -2.5, -2.5), 1.0/8.0, 2*8*8, 32, &mut renderer.render_triangles_lighting_pos_color_normal, &mut renderer.render_lines_pos_color);
    let mut triangles_for_rt = Vector::with_capacity(1000);
    construct_grid(a1, Vec3::new(-4.0, -2.5, -4.5), 0.125/2.0, 128, 16, &mut renderer.render_triangles_lighting_pos_color_normal, &mut renderer.render_lines_pos_color, &mut triangles_for_rt);
    //unsafe { uni_manifold_dc::sample_grid(a1, Vec3::new(-4.0, -2.5, -4.5), 0.125 / 2.0, 128, 16, 0.001, &mut renderer.render_lines_pos_color); }

    //triangle / mesh
    //add_triangle_color(&mut renderer.render_triangles_pos_color, Triangle3{p1 : vec3![-0.2, 0.0, -1.0], p2 : vec3![0.2, 0.0, -1.0], p3 : vec3![0.0, 0.3, -1.0]}, red);
    //add_grid3_pos_color(&mut renderer.render_lines_pos_color, Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), Vec3::new(0.0, 1.0, 0.0), 1.0, 8, white);

    //println!("{:?}", img);
    /*let mut tex = [0u32];
    gl_active_texture(gl::GL_TEXTURE0);
    gl_gen_textures(1, &mut tex);
    gl_bind_texture(gl::GL_TEXTURE_2D, tex[0]);
    gl_tex_parameteri(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_MAG_FILTER, gl::GL_LINEAR);
    gl_tex_parameteri(gl::GL_TEXTURE_2D, gl::GL_TEXTURE_MIN_FILTER, gl::GL_LINEAR);
    renderer.render_triangles_texture_screen_pos_tex.data = tex[0];*/
    /*gl_use_program(rs_prog);*/
    //gl_dispatch_compute(def_width, def_height, 1);
    //gl_memory_barrier(gl::GL_SHADER_IMAGE_ACCESS_BARRIER_BIT);

    //vulkan_raytracer::setup(def_width, def_height, &cam, &triangles_for_rt);
    gl_generate_mipmap(GL_TEXTURE_2D);



    //add_quad_pos_tex(&mut renderer.render_triangles_texture_screen_pos_tex, [vec3![0.0, 0.0, 0.0], vec3![800.0, 0.0, 0.0], vec3![800.0, 600.0, 0.0], vec3![0.0, 600.0, 0.0]], [vec2![0.0, 1.0], vec2![1.0, 1.0], vec2![1.0, 0.0], vec2![0.0, 0.0]]);


    renderer.run(move |renderer, dt_ns| {

        handle_input(renderer.glfw.as_mut().unwrap(), renderer.window.as_mut().unwrap(), dt_ns, &mut renderer.camera);

        //let img = vulkan_raytracer::setup(def_width, def_height, &renderer.camera, &triangles_for_rt);
        //gl_tex_image_2d(gl::GL_TEXTURE_2D, 0, gl::GL_RGBA, def_width, def_height, 0, gl::GL_RGBA, gl::GL_UNSIGNED_BYTE, img.as_slice());

        gl_enable(gl::GL_DEPTH_TEST);
        gl_clear(gl::GL_COLOR_BUFFER_BIT | gl::GL_DEPTH_BUFFER_BIT);
        gl_clear_color(0.3, 0.2, 0.6, 1.0);

        
    });
}

fn main(){
    run();
}