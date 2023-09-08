#![allow(non_snake_case)]
#![allow(dead_code)]
#![allow(unused)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use libc::*;
use std::ffi::CString;
use std::ffi::CStr;
use std::ptr;
use std::str;
use std;
use matrix::Vec3;
use glad_gl::gl;
use std::os::raw::c_void;
use glad_gl::gl::GLbitfield;

pub struct WindowInfo{
    pub width: usize,
    pub height: usize,
    pub handle: *mut GlfwWindow, //TODO many GL functions take mutable ptr to GlfwWindow, but it is unsafe to leave it as mut in this struct
}

pub enum GlfwWindow{}
pub enum GlfwMonitor{}

#[derive(Clone, Copy, Debug)]
#[repr(C)]
pub struct Program{
    pub id: u32,
    
}

impl Program{
    pub fn get_uniform(&self, name: &str) -> i32{
        gl_get_uniform_location(self.id, name)
    }

    pub fn is_in_use(&self) -> bool {
        let mut cur_id = 0;
        gl_get_integerv(gl::GL_CURRENT_PROGRAM, &mut cur_id);
        self.id == cur_id as u32
    }

    pub fn enable(&self){
        if !self.is_in_use(){
            gl_use_program(self.id);
        }
    }

    pub fn disable(&self){
        if self.is_in_use(){
            gl_use_program(0);
        }
    }

    pub fn set_bool(&self, name: &str, val: bool){
        self.enable();
        gl_uniform1i(self.get_uniform(name), if val {1} else {0});
    }


    pub fn set_int(&self, name: &str, val: i32){
        self.enable();
        gl_uniform1i(self.get_uniform(name), val);
    }

    pub fn set_float(&self, name: &str, val: f32){
        self.enable();
        gl_uniform1f(self.get_uniform(name), val);
    }

    pub fn set_float2(&self, name: &str, val1: f32, val2: f32){
        self.enable();
        gl_uniform2f(self.get_uniform(name), val1, val2);
    }

    pub fn set_float3(&self, name: &str, val1: f32, val2: f32, val3: f32){
        self.enable();
        gl_uniform3f(self.get_uniform(name), val1, val2, val3);
    }

    pub fn set_vec3f(&self, name: &str, vec : Vec3<f32>){
        self.enable();
        gl_uniform3f(self.get_uniform(name), vec.x, vec.y, vec.z);
    }

    pub fn set_float4(&self, name: &str, val1: f32, val2: f32, val3: f32, val4: f32){
        self.enable();
        gl_uniform4f(self.get_uniform(name), val1, val2, val3, val4);
    }


    //mat is assumed to be in column major order
    pub fn set_float4x4(&self, name: &str, transpose: bool, mat: &[f32]){
        self.enable();
        gl_uniform_matrix4fv(self.get_uniform(name), transpose, mat)
    }
    
}
