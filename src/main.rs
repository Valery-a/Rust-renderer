extern crate glad_gl;
extern crate glad_vulkan;
#[cfg(not(target_os = "macos"))]
extern crate openblas_src;

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
    
}

fn main(){
    run();
}