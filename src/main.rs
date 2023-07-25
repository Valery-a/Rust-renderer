extern crate sdl2;
use std::any::Any;
use std::io;
use sdl2::keyboard::{Keycode};
use sdl2::pixels::{Color};
use sdl2::event::Event;
use sdl2::rect::Point;
use core::f32::{consts::PI, INFINITY};
const RESOLUTION: (u32, u32) = (800, 600);
use std::time::Instant;
mod line_intersection;
mod face;
mod vector;
mod matrix;
mod camera;

use line_intersection::line_intersection; // Import from line_intersection.rs
use face::Face; // Import from face.rs
use vector::Vector; // Import from vector.rs
use matrix::Matrix; // Import from matrix.rs
use camera::Camera; // Import from camera.rs

mod objects {
    pub mod cube;    // Import from objects/cube.rs
    pub mod pyramid; // Import from objects/pyramid.rs
    pub mod sphere;  // Import from objects/sphere.rs
}

use crate::objects::cube::Object1;
use crate::objects::pyramid::Object2;
use crate::objects::sphere::Object3;


fn screen(v: &Vector) -> Point {
    let x = (v.x+1.) / 2. * (RESOLUTION.0 as f32);
    let y = (v.y+1.) / 2. * (RESOLUTION.1 as f32);
    Point::new(x as i32, y as i32)
}

fn main() {
    let mut input: String = String::new();
    println!("Please enter the object you want to render:");
    io::stdin().read_line(&mut input).expect("Failed to read input.");
    let object = match input.trim() {
        "cube" => Box::new(Object1::cube()) as Box<dyn Any>,
        "pyramid" => Box::new(Object2::pyramid()) as Box<dyn Any>,
        "sphere" => Box::new(Object3::sphere()) as Box<dyn Any>,
        _ => {
            eprintln!("Invalid input. Please enter a valid object");
            return;
        }
    };

    let mut input = String::new();
    println!("Please enter the number of objects you want to generate:");
    io::stdin().read_line(&mut input).expect("Failed to read input.");
    let num_objects = match input.trim().parse::<usize>() {
        Ok(n) => n,
        Err(_) => {
            eprintln!("Invalid input. Please enter a valid number");
            return;
        }
    };

    let face = Face::new(Point::new(10, 2), Point::new(200, 200), Point::new(4, 200));
    dbg!(face.barycentric(&Point::new(50, 50)));

    let sdl_context = sdl2::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let window = video_subsystem.window("renderer", RESOLUTION.0, RESOLUTION.1)
        .position_centered()
        .build()
        .unwrap();
    let mut canvas = window.into_canvas().build().unwrap();

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut i = 0_f32;
let mut last_frame_time = Instant::now();
'running: loop {
    canvas.set_draw_color(Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.set_draw_color(Color::RGB(244, 0, 0));

    let mut camera = Camera::new(Vector::from_xyz(i, -6., 0.), Vector::from_xyz(0., 0., 0.), Vector::from_xyz(0., 0., -1.));
    let view = camera.view_matrix();
    let proj = Matrix::perspective(1., PI / 2., 5., INFINITY);
    let view_proj = &proj.dot(&view);

    for event in event_pump.poll_iter() {
        match event {
            Event::Quit { .. } => {
                break 'running;
            }
            Event::KeyDown { keycode: Some(keycode), .. } => {
                match keycode {
                    Keycode::W => camera.move_forward(0.9),
                    Keycode::S => camera.move_backward(0.9),
                    Keycode::A => camera.move_left(0.9),
                    Keycode::D => camera.move_right(0.9),
                    Keycode::Up => i += 0.1,
                    Keycode::Down => i -= 0.1,
                    _ => {}
                }
            }
            _ => {}
        }
    }

    for _ in 0..num_objects {
        if let Some(object3) = object.downcast_ref::<Object3>() {
            object3.render(&mut canvas, &view_proj);
        } else if let Some(object2) = object.downcast_ref::<Object2>() {
            object2.render(&mut canvas, &view_proj);
        } else if let Some(object1) = object.downcast_ref::<Object1>() {
            object1.render(&mut canvas, &view_proj);
        }
    }

    canvas.present();

    // FPS counter
    let current_time = Instant::now();
    let elapsed_time = current_time.duration_since(last_frame_time);
    let fps = 1.0 / elapsed_time.as_secs_f32();
    last_frame_time = current_time;
    println!("FPS: {:.2}", fps);
}
}