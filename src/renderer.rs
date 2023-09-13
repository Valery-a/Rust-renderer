use std::vec::Vec as Vector;
use graphics::*;
use math::*;
use std::sync::mpsc::Receiver;
use std::collections::HashMap;
use std::fs;
use graphics_util::create_vert_frag_program;
use std::io::Read;

use matrix::*;
use typenum::U4;

use glfw::{Action, Context, Key};
use time::precise_time_ns;
use std::fmt::Display;
use glad_gl::gl;
use std::ptr::null;

fn load_shaders_vf() -> HashMap<String, Program>{
    let dir : &str = "./assets/shaders/gl/";
    let paths = fs::read_dir(dir).unwrap();
    let mut map : HashMap<String, Program> = HashMap::new();

    for entry in paths{
        let name : String = String::from(entry
            .unwrap()
            .path()
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap());

        if !map.contains_key(&name){
            let mut file_vert = fs::File::open(
                dir.to_string() + &name + ".vert").unwrap();
            let mut source_vert = String::new();
            file_vert.read_to_string(&mut source_vert).unwrap();

            let mut file_frag = fs::File::open(
                dir.to_string() + &name + ".frag").unwrap();
            let mut source_frag = String::new();
            file_frag.read_to_string(&mut source_frag).unwrap();

            let prog = create_vert_frag_program(
                &source_vert,
                &source_frag);


            map.insert(name, Program{id: prog});
        }
    }

    map
}

pub trait RendererVertFrag{
    fn render_mode       (&self) -> u32;
    fn shader_name       (&self) -> String;
    fn set_attrib_ptrs   (&mut self);
    fn construct         (&mut self) -> bool;
    fn deconstruct       (&mut self) -> bool;
    fn draw              (&mut self) -> bool;
    fn reset             (&mut self); //used to clear/reset all data stored in 'self'
    fn pre_render        (&mut self);
}


pub struct RendererVertFragDef<Data>{
    pub vertex_size: u32,
    pub vertex_pool: Vector<f32>,
    pub index_pool: Vector<u32>,
    pub vertex_count: u32,
    pub vbo: u32,
    pub vao: u32,
    pub ebo: u32,
    pub constructed: bool,
    pub render_mode: u32,
    pub shader_name: String,
    pub set_attrib_ptrs: fn(&mut RendererVertFragDef<Data>),
    pub data : Data,
    pub pre_render : Option<fn(&mut RendererVertFragDef<Data>)>,
    pub get_program : Box<dyn Fn() -> Program>,

}

#[derive(Debug, Clone, Copy)]
pub struct Cam{
    pub pos : Vec3<f32>,
    pub look : Vec3<f32>,
    pub up : Vec3<f32>,
}

impl Display for Cam {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        writeln!(f, "Cam{{")?;
        writeln!(f, "pos = {}, ", self.pos)?;
        writeln!(f, "look = {}, ", self.look)?;
        writeln!(f, "up = {}, ", self.up)?;
        writeln!(f, "}}")?;
        Ok(())
    }
}

pub struct Renderer{
    pub render_triangles_pos_color : RendererVertFragDef<()>,
    pub render_lines_pos_color : RendererVertFragDef<()>,
    pub render_triangles_lighting_pos_color_normal : RendererVertFragDef<()>,
    pub render_triangles_texture_screen_pos_tex: RendererVertFragDef<u32>, //one texture for one of these
    pub glfw : Option<glfw::Glfw>,
    pub window : Option<glfw::Window>,
    pub events : Option<Receiver<(f64, glfw::WindowEvent)>>,
    pub frame_buffer_size_callback : Option<Box<dyn Fn(i32, i32)>>,
    pub shaders : HashMap<String, Program>,
    pub camera : Cam,
    last_frame_nt : u64
}

impl Renderer{

    pub fn new(camera : Cam) -> Renderer{
        Renderer{render_triangles_pos_color : RendererVertFragDef::new(VERTEX_SIZE_POS_COLOR, set_attrib_ptrs_pos_color, gl::GL_TRIANGLES, "color", (), None),
                 render_lines_pos_color : RendererVertFragDef::new(VERTEX_SIZE_POS_COLOR, set_attrib_ptrs_pos_color, gl::GL_LINES, "color", (), None),
                 render_triangles_lighting_pos_color_normal : RendererVertFragDef::new(VERTEX_SIZE_POS_COLOR_NORMAL, set_attrib_ptrs_pos_color_normal, gl::GL_TRIANGLES, "lighting", (), None),
                 render_triangles_texture_screen_pos_tex: RendererVertFragDef::new(VERTEX_SIZE_POS_TEX, set_attrib_ptrs_pos_tex, gl::GL_TRIANGLES, "texture", 0, Some(pre_render_pos_tex)),
                 glfw : None, window : None, events : None, frame_buffer_size_callback : None,
                 shaders : HashMap::new(), camera, last_frame_nt : 0}
    }

    pub fn get_window(&mut self) -> &mut glfw::Window{
        self.window.as_mut().unwrap()
    }
    pub fn get_glfw(&mut self) -> &mut glfw::Glfw{
        self.glfw.as_mut().unwrap()
    }

    pub fn get_shaders(&mut self) -> &mut HashMap<String, Program>{
        &mut self.shaders
    }

    pub fn init(&mut self, start_width : u32, start_height : u32, title : &str){
        let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

        glfw.window_hint(glfw::WindowHint::ContextVersionMajor(3));
        glfw.window_hint(glfw::WindowHint::ContextVersionMinor(3));
        glfw.window_hint(glfw::WindowHint::OpenGlProfile(glfw::OpenGlProfileHint::Core));
        if cfg!(target_os = "macos") {
            glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));
        }
        // Create a windowed mode window and its OpenGL context
        let (mut window, events) = glfw.create_window(start_width, start_height, title, glfw::WindowMode::Windowed)
            .expect("Failed to create GLFW window.");


        // Make the window's context current
        window.make_current();
        gl::load(|e| glfw.get_proc_address_raw(e) as *const std::os::raw::c_void);
        glad_vulkan::vk::load(|e| glfw.get_proc_address_raw(e) as *const std::os::raw::c_void);
        window.set_key_polling(true);
        window.set_framebuffer_size_polling(true);

        self.window = Some(window);
        self.glfw = Some(glfw);
        self.events = Some(events);

        self.shaders = load_shaders_vf();

        let prog_lighting = self.shaders.get("lighting").unwrap().clone();
        let prog_color = self.shaders.get("color").unwrap().clone();
        let prog_texture = self.shaders.get("texture").unwrap().clone();

        self.render_triangles_pos_color.get_program = Box::new(move || prog_color);
        self.render_lines_pos_color.get_program = Box::new(move || prog_color);
        self.render_triangles_lighting_pos_color_normal.get_program = Box::new(move || prog_lighting);
        self.render_triangles_texture_screen_pos_tex.get_program = Box::new(move || prog_texture);
    }

    pub fn set_framebuffer_size_callback<F : Fn(i32, i32) >(&mut self, callback : F) where F : 'static{
        self.frame_buffer_size_callback = Some(Box::new(callback));
    }

    pub fn run<F : Fn(&mut Renderer, u64)>(&mut self, pre_render : F) where F : 'static{
        let red = Vec3::new(1.0, 0.0, 0.0);
        let green = Vec3::new(0.0, 1.0, 0.0);
        let blue = Vec3::new(0.0, 0.0, 1.0);
        let _white = Vec3::new(1.0, 1.0, 1.0);

        self.last_frame_nt = precise_time_ns();

        // Loop until the user closes the window
        while !self.get_window().should_close() {
            let t_ns = precise_time_ns();
            let dt_ns = t_ns - self.last_frame_nt;

            pre_render(self, dt_ns);

            let(win_w, win_h) = self.get_window().get_size();
            let aspect =win_w as f32 / win_h as f32;

            let persp = perspective(90.0, aspect, 0.1, 16.0);
            let view = view_dir(self.camera.pos, self.camera.look, self.camera.up);
            let id = Mat::<f32, U4, U4>::identity();
            let ortho_mat = ortho(0.0, win_w as f32, 0.0, win_h as f32, -1.0, 1.0);

            let shader_color = self.get_shaders().get("color").unwrap();
            shader_color.enable();

            shader_color.set_float4x4("P", true, persp.as_slice());
            shader_color.set_float4x4("V", true, view.as_slice());

            if !self.render_lines_pos_color.constructed{
                self.render_lines_pos_color.construct();
            }

            if self.render_lines_pos_color.pre_render.is_some(){
                self.render_lines_pos_color.pre_render();
            }

            self.render_lines_pos_color.draw();

            if !self.render_triangles_pos_color.constructed{
                self.render_triangles_pos_color.construct();
            }

            if self.render_triangles_pos_color.pre_render.is_some(){
                self.render_triangles_pos_color.pre_render();
            }


            self.render_triangles_pos_color.draw();

            let shader_lighting = self.get_shaders().get("lighting").unwrap();
            shader_lighting.enable();


            shader_lighting.set_float4x4("P", true, persp.as_slice());
            shader_lighting.set_float4x4("V", true, view.as_slice());

            shader_lighting.set_vec3f("pointLight.pos" ,Vec3::new(0.0, 8.0,0.0));
            shader_lighting.set_vec3f("pointLight.color" ,(red + green + blue) * 15.0);

            if !self.render_triangles_lighting_pos_color_normal.constructed{
                self.render_triangles_lighting_pos_color_normal.construct();
            }

            if self.render_triangles_lighting_pos_color_normal.pre_render.is_some(){
                self.render_triangles_lighting_pos_color_normal.pre_render();
            }


            self.render_triangles_lighting_pos_color_normal.draw();

            let shader_tex = self.get_shaders().get("texture").unwrap().clone();
            shader_tex.enable();
            gl_bind_texture(gl::GL_TEXTURE_2D, self.render_triangles_texture_screen_pos_tex.data);
            shader_tex.set_float4x4("P", false, ortho_mat.as_slice());
            shader_tex.set_float4x4("V", true, id.as_slice());


            if !self.render_triangles_texture_screen_pos_tex.constructed{
                self.render_triangles_texture_screen_pos_tex.construct();
            }

            if self.render_triangles_texture_screen_pos_tex.pre_render.is_some(){
                self.render_triangles_texture_screen_pos_tex.pre_render();
            }


            self.render_triangles_texture_screen_pos_tex.draw();


            // Swap front and back buffers
            self.get_window().swap_buffers();

            // Poll for and process events
            self.get_glfw().poll_events();
            let window = self.window.as_mut().unwrap();
            for (_, event) in glfw::flush_messages(self.events.as_ref().unwrap()) {
                //println!("{:?}", event);
                match event {
                    glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => {
                        window.set_should_close(true)
                    },
                    glfw::WindowEvent::FramebufferSize(w, h) => {
                        self.frame_buffer_size_callback.iter().for_each(|f| f(w, h));
                    }
                    _ => {},
                }
            }

            self.last_frame_nt = t_ns;
        }
    }

}

pub fn pre_render_pos_tex(r : &mut RendererVertFragDef<u32>){
    gl_active_texture(gl::GL_TEXTURE0);
    (r.get_program)().set_int("textureID", 0);
    (r.get_program)().set_float3("extraColor", 1.0, 1.0, 1.0);
}

pub const VERTEX_SIZE_POS_COLOR : u32 = 6;
pub const VERTEX_SIZE_POS_COLOR_NORMAL : u32 = 9;
pub const VERTEX_SIZE_POS_TEX : u32 = 5;

pub fn set_attrib_ptrs_pos_color<Data>(_:&mut RendererVertFragDef<Data>){
    gl_vertex_attrib_pointer(0, 3, gl::GL_FLOAT, false, VERTEX_SIZE_POS_COLOR * 4,
                             0);
    gl_enable_vertex_attrib_array(0);

    gl_vertex_attrib_pointer(1, 3, gl::GL_FLOAT, false, VERTEX_SIZE_POS_COLOR * 4,
                             3 * 4);
    gl_enable_vertex_attrib_array(1);

}

pub fn set_attrib_ptrs_pos_tex<Data>(_:&mut RendererVertFragDef<Data>){
    gl_vertex_attrib_pointer(0, 3, gl::GL_FLOAT, false, VERTEX_SIZE_POS_TEX * 4,
                             0);
    gl_enable_vertex_attrib_array(0);

    gl_vertex_attrib_pointer(1, 2, gl::GL_FLOAT, false, VERTEX_SIZE_POS_TEX * 4,
                             3 * 4);
    gl_enable_vertex_attrib_array(1);

}

pub fn set_attrib_ptrs_pos_color_normal<Data>(_:&mut RendererVertFragDef<Data>){
    gl_vertex_attrib_pointer(0, 3, gl::GL_FLOAT, false, VERTEX_SIZE_POS_COLOR_NORMAL * 4,
                             0);
    gl_enable_vertex_attrib_array(0);

    gl_vertex_attrib_pointer(1, 3, gl::GL_FLOAT, false, VERTEX_SIZE_POS_COLOR_NORMAL * 4,
                             3 * 4);
    gl_enable_vertex_attrib_array(1);

    gl_vertex_attrib_pointer(2, 3, gl::GL_FLOAT, false, VERTEX_SIZE_POS_COLOR_NORMAL * 4,
                             6 * 4);
    gl_enable_vertex_attrib_array(2);

}

impl<Data> RendererVertFrag for RendererVertFragDef<Data>{
    fn render_mode(&self) -> u32 {
        self.render_mode
    }

    fn shader_name(&self) -> String {
        self.shader_name.clone()
    }

    fn set_attrib_ptrs(&mut self) {
        (self.set_attrib_ptrs)(self)
    }

    fn construct(&mut self) -> bool {
        if self.constructed {return false;};

        self.vao = gl_gen_vertex_arrays();
        self.vbo = gl_gen_buffers();
        self.ebo = gl_gen_buffers();


        gl_bind_vertex_array(self.vao);

        gl_bind_buffer(gl::GL_ARRAY_BUFFER, self.vbo);

        gl_buffer_data(gl::GL_ARRAY_BUFFER,
                       self.vertex_pool.len(),
                       self.vertex_pool.as_slice(),
                       gl::GL_STATIC_DRAW);

        gl_bind_buffer(gl::GL_ELEMENT_ARRAY_BUFFER, self.ebo);
        gl_buffer_data(gl::GL_ELEMENT_ARRAY_BUFFER, self.index_pool.len(),
                       self.index_pool.as_slice(),
                       gl::GL_STATIC_DRAW
        );

        self.set_attrib_ptrs();

        gl_bind_buffer(gl::GL_ARRAY_BUFFER, 0);
        gl_bind_vertex_array(0);

        self.constructed = true;

        true
    }

    fn deconstruct(&mut self) -> bool {
        if !self.constructed {return false;};

        gl_delete_vertex_arrays(self.vao);
        gl_delete_buffers(self.vbo);
        gl_delete_buffers(self.ebo);

        self.constructed = false;

        true
    }

    fn draw(&mut self) -> bool {
        if !self.constructed {return false;};

        gl_bind_vertex_array(self.vao);
        gl_draw_elements(self.render_mode, self.index_pool.len(), gl::GL_UNSIGNED_INT, null());
        gl_bind_vertex_array(0);

        true
    }

    fn reset(&mut self) {
        self.vertex_pool.clear();
        self.index_pool.clear();
        self.vertex_count = 0;
    }

    fn pre_render(&mut self) {
        match self.pre_render{
            Some(x) => x.clone()(self),
            None => ()
        }
    }
}

impl<Data> RendererVertFragDef<Data>{
    pub fn new(vs: u32,
               set_attrib_ptrs : fn (&mut RendererVertFragDef<Data>),
               render_mode: u32,
               shader_name: &str,
               data : Data,
               pre_render : Option<fn(&mut RendererVertFragDef<Data>)>) -> RendererVertFragDef<Data>{
        RendererVertFragDef{
            vertex_size: vs,
            vertex_pool: Vector::new(),
            index_pool: Vector::new(),
            vertex_count: 0,
            vbo: 0,
            vao: 0,
            ebo: 0,
            constructed:false,
            render_mode,
            shader_name : String::from(shader_name.clone()),
            set_attrib_ptrs,
            data,
            pre_render,
            get_program : Box::new(|| Program{id:0})
        }
    }
}

pub fn add_triangle_color<Data>(dat: &mut RendererVertFragDef<Data>, tr: Triangle3<f32>, color: Vec3<f32>){
    dat.vertex_pool.push(tr.p1[0]);
    dat.vertex_pool.push(tr.p1[1]);
    dat.vertex_pool.push(tr.p1[2]);

    dat.vertex_pool.push(color[0]);
    dat.vertex_pool.push(color[1]);
    dat.vertex_pool.push(color[2]);

    dat.vertex_pool.push(tr.p2[0]);
    dat.vertex_pool.push(tr.p2[1]);
    dat.vertex_pool.push(tr.p2[2]);

    dat.vertex_pool.push(color[0]);
    dat.vertex_pool.push(color[1]);
    dat.vertex_pool.push(color[2]);

    dat.vertex_pool.push(tr.p3[0]);
    dat.vertex_pool.push(tr.p3[1]);
    dat.vertex_pool.push(tr.p3[2]);

    dat.vertex_pool.push(color[0]);
    dat.vertex_pool.push(color[1]);
    dat.vertex_pool.push(color[2]);

    dat.index_pool.push(dat.vertex_count + 0);
    dat.index_pool.push(dat.vertex_count + 1);
    dat.index_pool.push(dat.vertex_count + 2);


    dat.vertex_count += 3;
}


pub fn add_triangle_pos_color_normal<Data>(dat: &mut RendererVertFragDef<Data>, tr: Triangle3<f32>, color: Vec3<f32>, normal : Vec3<f32>){
    dat.vertex_pool.push(tr.p1[0]);
    dat.vertex_pool.push(tr.p1[1]);
    dat.vertex_pool.push(tr.p1[2]);

    dat.vertex_pool.push(color[0]);
    dat.vertex_pool.push(color[1]);
    dat.vertex_pool.push(color[2]);

    dat.vertex_pool.push(normal[0]);
    dat.vertex_pool.push(normal[1]);
    dat.vertex_pool.push(normal[2]);

    dat.vertex_pool.push(tr.p2[0]);
    dat.vertex_pool.push(tr.p2[1]);
    dat.vertex_pool.push(tr.p2[2]);

    dat.vertex_pool.push(color[0]);
    dat.vertex_pool.push(color[1]);
    dat.vertex_pool.push(color[2]);

    dat.vertex_pool.push(normal[0]);
    dat.vertex_pool.push(normal[1]);
    dat.vertex_pool.push(normal[2]);

    dat.vertex_pool.push(tr.p3[0]);
    dat.vertex_pool.push(tr.p3[1]);
    dat.vertex_pool.push(tr.p3[2]);

    dat.vertex_pool.push(color[0]);
    dat.vertex_pool.push(color[1]);
    dat.vertex_pool.push(color[2]);

    dat.vertex_pool.push(normal[0]);
    dat.vertex_pool.push(normal[1]);
    dat.vertex_pool.push(normal[2]);

    dat.index_pool.push(dat.vertex_count + 0);
    dat.index_pool.push(dat.vertex_count + 1);
    dat.index_pool.push(dat.vertex_count + 2);


    dat.vertex_count += 3;
}

fn add_vector_to_pool<Data>(dat : &mut RendererVertFragDef<Data>, vec : Vec3<f32>){
    dat.vertex_pool.push(vec.x);
    dat.vertex_pool.push(vec.y);
    dat.vertex_pool.push(vec.z);
}

pub fn add_cube_bounds_pos_color<Data>(dat : &mut RendererVertFragDef<Data>, cube : Cube<f32>, color : Vec3<f32>){
    add_vector_to_pool(dat, Vec3::new(cube.center.x - cube.extent, cube.center.y - cube.extent, cube.center.z - cube.extent));
    add_vector_to_pool(dat, color);
    add_vector_to_pool(dat, Vec3::new(cube.center.x + cube.extent, cube.center.y - cube.extent, cube.center.z - cube.extent));
    add_vector_to_pool(dat, color);
    add_vector_to_pool(dat, Vec3::new(cube.center.x + cube.extent, cube.center.y + cube.extent, cube.center.z - cube.extent));
    add_vector_to_pool(dat, color);
    add_vector_to_pool(dat, Vec3::new(cube.center.x - cube.extent, cube.center.y + cube.extent, cube.center.z - cube.extent));
    add_vector_to_pool(dat, color);
    add_vector_to_pool(dat, Vec3::new(cube.center.x - cube.extent, cube.center.y - cube.extent, cube.center.z + cube.extent));
    add_vector_to_pool(dat, color);
    add_vector_to_pool(dat, Vec3::new(cube.center.x + cube.extent, cube.center.y - cube.extent, cube.center.z + cube.extent));
    add_vector_to_pool(dat, color);
    add_vector_to_pool(dat, Vec3::new(cube.center.x + cube.extent, cube.center.y + cube.extent, cube.center.z + cube.extent));
    add_vector_to_pool(dat, color);
    add_vector_to_pool(dat, Vec3::new(cube.center.x - cube.extent, cube.center.y + cube.extent, cube.center.z + cube.extent));
    add_vector_to_pool(dat, color);

    let indices : [u32;24] = [0,1,1,2,2,3,3,0, 4,5,5,6,6,7,7,4, 0,4, 1,5, 2,6, 3,7];
    for i in indices.iter() {dat.index_pool.push(i.clone() + dat.vertex_count);}
    dat.vertex_count += 8;
}

pub fn add_grid3_pos_color<Data>(dat : &mut RendererVertFragDef<Data>, center : Vec3<f32>, tangent : Vec3<f32>, normal : Vec3<f32>, extent : f32, subdiv_num : u32, color : Vec3<f32>){
    let right = tangent.cross(normal) * extent;
    let along = tangent * extent;
    add_vector_to_pool(dat, center - right - along);
    add_vector_to_pool(dat, color);
    add_vector_to_pool(dat, center + right - along);
    add_vector_to_pool(dat, color);
    add_vector_to_pool(dat, center + right + along);
    add_vector_to_pool(dat, color);
    add_vector_to_pool(dat, center - right + along);
    add_vector_to_pool(dat, color);

    let a = extent / subdiv_num as f32;
    //TODO inefficient loops(could be done in one)
    for i in 1 .. 2 * subdiv_num{
        add_vector_to_pool(dat, center - right * (extent - i as f32 * a) - along);
        add_vector_to_pool(dat, color);
    }

    for i in 1 .. 2 * subdiv_num{
        add_vector_to_pool(dat, center + right - along * (extent - i as f32 * a) );
        add_vector_to_pool(dat, color);
    }

    for i in 1 .. 2 * subdiv_num{
        add_vector_to_pool(dat, center + right * (extent - i as f32 * a) + along);
        add_vector_to_pool(dat, color);
    }

    for i in 1 .. 2 * subdiv_num{
        add_vector_to_pool(dat, center - right + along * (extent - i as f32 * a) );
        add_vector_to_pool(dat, color);
    }

    dat.index_pool.push(0 + dat.vertex_count);
    dat.index_pool.push(1 + dat.vertex_count);
    dat.index_pool.push(1 + dat.vertex_count);
    dat.index_pool.push(2 + dat.vertex_count);
    dat.index_pool.push(2 + dat.vertex_count);
    dat.index_pool.push(3 + dat.vertex_count);
    dat.index_pool.push(3 + dat.vertex_count);
    dat.index_pool.push(0 + dat.vertex_count);

    let off0 : u32 = 4;
    let off1 : u32 = subdiv_num * 2 - 1;

    for i in 0..off1{
        dat.index_pool.push(off0 + off1 + i + dat.vertex_count);
        dat.index_pool.push(off0 + 4*off1 - i - 1 + dat.vertex_count);
    }

    for i in 0..off1{
        dat.index_pool.push(off0 + i + dat.vertex_count);
        dat.index_pool.push(off0 + 3*off1 - i - 1 + dat.vertex_count);
    }

    dat.vertex_count += 4 + 4 * (2 * subdiv_num - 1)
}

