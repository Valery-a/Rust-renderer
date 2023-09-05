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


fn run(){
    
}

fn main(){
    run();
}