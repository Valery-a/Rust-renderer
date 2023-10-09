use cgmath::{Deg, InnerSpace, Matrix4, One, PerspectiveFov, Vector2, Vector3, Zero};
use rend::render::{ChunkMeshBuilder, Face, Vertex};
use parking_lot::Mutex;
use std::time::{Duration, Instant};
use voxel_render::camera::Camera;
use winit::event::DeviceEvent;

use voxel_render::mesh::InstanceTransformData;
use voxel_space::{translation, Sided, Walker};
use wgpu::{include_wgsl, util::DeviceExt, Features, TextureUsages};
use winit::{
    event::{Event, KeyboardInput, StartCause, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::Window,
};

pub mod rend;

// The RenderState struct represents the state of the rendering system.
// It holds references to the wgpu device, queue, bind group layouts, pipeline layout, surface, window, configuration, and swapchain format.
pub struct RenderState {
    device: wgpu::Device,
    queue: wgpu::Queue,

    camera_layout: wgpu::BindGroupLayout,
    texture_layout: wgpu::BindGroupLayout,
    pipeline_layout: wgpu::PipelineLayout,

    surface: wgpu::Surface,
    pub window: Window,
    config: Mutex<wgpu::SurfaceConfiguration>,
    pub swapchain_format: wgpu::TextureFormat,
}
impl RenderState {
    pub fn resize(&self, width: u32, height: u32) {
        let mut config = self.config.lock();
        config.width = width;
        config.height = height;
        self.surface.configure(&self.device, &config);
    }
}

// This struct represents the camera as a bind group used for passing camera data to the GPU during rendering.
pub struct CameraBindGroup(wgpu::BindGroup);
impl CameraBindGroup {
    pub fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[Camera::layout_entry(0)],
        })
    }

    pub fn new(device: &wgpu::Device, layout: &wgpu::BindGroupLayout, camera: &Camera) -> Self {
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout,
            entries: &[camera.entry(0)],
        });
        CameraBindGroup(bind_group)
    }

    pub fn set_bind_group<'a>(&'a self, rpass: &mut wgpu::RenderPass<'a>, index: u32) {
        rpass.set_bind_group(index, &self.0, &[]);
    }
}

#[derive(Debug, Default, Clone, Copy, Eq, PartialEq)]
pub struct PlayerInput {
    pub forward: bool,
    pub backward: bool,
    pub leftward: bool,
    pub rightward: bool,
    pub upward: bool,
    pub downward: bool,
}
impl PlayerInput {
    pub fn update(&mut self, scancode: u32, pressed: bool) {
        *(match scancode {
            17 => &mut self.forward,
            30 => &mut self.leftward,
            31 => &mut self.backward,
            32 => &mut self.rightward,
            42 => &mut self.downward,
            57 => &mut self.upward,
            _ => return,
        }) = pressed;
    }

    pub fn delta(&self) -> Option<Vector3<f64>> {
        let mut delta = Vector3::<i32>::zero();
        if self.leftward {
            delta.x -= 1;
        }
        if self.rightward {
            delta.x += 1;
        }
        if self.downward {
            delta.y -= 1;
        }
        if self.upward {
            delta.y += 1;
        }
        if self.backward {
            delta.z += 1;
        }
        if self.forward {
            delta.z -= 1;
        }
        if !delta.is_zero() {
            delta.cast()
        } else {
            None
        }
    }
}

fn main() {
    // Initializes the wgpu graphics backend and sets up the rendering state, including creating the device, queue, and bind group layouts.
    env_logger::init();
    // Creates an event loop for handling window events and user input.
    let event_loop = EventLoop::new();
    // Creates a window for rendering.
    let window = Window::new(&event_loop).unwrap();
    // Polls the block_on function to set up the rendering state asynchronously.
    let state = pollster::block_on(async {
        // Creates a new wgpu instance.
        let instance = wgpu::Instance::new(Default::default());
        // Creates a surface on which rendering will occur.
        let surface = unsafe { instance.create_surface(&window).unwrap() };
        // Requests a compatible adapter for rendering.
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            })
            .await
            .unwrap();
        // Creates a wgpu device and queue for rendering.
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: None,
                    features: Features::empty(),
                    limits: wgpu::Limits::downlevel_defaults(),
                },
                None,
            )
            .await
            .unwrap();
        // Creates a bind group layout for the camera data.
        let camera_layout = CameraBindGroup::layout(&device);
        // Creates a bind group layout for the texture data.
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: None,
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        // Creates a pipeline layout with the camera and texture bind group layouts.
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&camera_layout, &texture_layout],
            push_constant_ranges: &[],
        });
        // Gets the surface capabilities to configure the surface.
        let caps = surface.get_capabilities(&adapter);
        // Creates a surface configuration with initial values for width, height, format, etc.
        let config = Mutex::new(wgpu::SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format: caps.formats[0],
            width: 0,
            height: 0,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: caps.alpha_modes[0],
            view_formats: Vec::new(),
        });
        // Constructs the RenderState struct with all the gathered data.
        RenderState {
            device,
            queue,
            camera_layout,
            texture_layout,
            pipeline_layout,
            surface,
            window,
            config,
            swapchain_format: caps.formats[0],
        }
    });
    // Loads the shaders (vertex and fragment) from the provided file using the include_wgsl! macro.
    let shader = state
        .device
        .create_shader_module(include_wgsl!("./resources/shader.wgsl"));
    // Creates a render pipeline with the defined vertex and fragment shaders, and pipeline layout.
    let pipeline = state
        .device
        .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&state.pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::LAYOUT, InstanceTransformData::LAYOUT],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(state.swapchain_format.into())],
            }),
            primitive: wgpu::PrimitiveState {
                cull_mode: Some(wgpu::Face::Back),
                ..Default::default()
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: wgpu::TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });
    // Creates a depth texture for depth testing during rendering.
    let mut depth = state.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Depth32Float,
        usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let mut depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());
    // Creates a new Camera instance with a default configuration.
    let mut camera = Camera::new(
        &state.device,
        PerspectiveFov {
            fovy: Deg(45.0).into(),
            aspect: 1.0,
            near: 0.1,
            far: 100.0,
        },
        Matrix4::one(),
    );
    // Creates a bind group for the camera using the CameraBindGroup struct.
    let camera_bind_group = CameraBindGroup::new(&state.device, &state.camera_layout, &camera);
    // Creates a PlayerInput struct to handle player input.
    let mut input = PlayerInput::default();
    // Variable to track if mouse tracking is enabled.
    let mut tracking = false;
    // Constants for defining the voxel space and step size.
    const SIDE: f32 = 0.485_868_28;
    // Constants for translating voxel positions.
    const STEP: f64 = 1.272_019_649_514_069;
    // The faces data representing the voxel-based 3D scene.
    let faces = {
        // Create a 3D array called 'chunk' with dimensions [16][16][16] and initialize all elements to 'false'.
        let mut chunk = [[[false; 16]; 16]; 16];
        // Set specific elements in 'chunk' to 'true' in a pattern to create a hollow cube-like shape at the center.
        for i in 0..16 {
            for j in 0..2 {
                for k in 0..2 {
                    chunk[i][7 + j][7 + k] = true;
                    chunk[7 + j][i][7 + k] = true;
                    chunk[7 + j][7 + k][i] = true;
                }
            }
        }
        // Create another 3D array called 'faces' with dimensions [16][16][16], where each element is initialized to a default value.
        let mut faces = [[[Default::default(); 16]; 16]; 16];
        // Loop through each coordinate (x, y, z) in 'faces'.
        for x in 0..16 {
            for y in 0..16 {
                for z in 0..16 {
                    // Get the corresponding boolean value from 'chunk' at the current coordinate (x, y, z).
                    let s = chunk[x][y][z];
                    // Use the boolean value and neighboring elements from 'chunk' to create faces for each direction (neg_x, pos_x, neg_y, etc.).
                    // The Face struct is used to represent each face, and it contains a boolean value indicating whether the face should be visible or not.
                    faces[x][y][z] = Sided {
                        neg_x: Face(s && (x == 0 || !chunk[x - 1][y][z])),
                        pos_x: Face(s && (x == 15 || !chunk[x + 1][y][z])),
                        neg_y: Face(s && (y == 0 || !chunk[x][y - 1][z])),
                        pos_y: Face(s && (y == 15 || !chunk[x][y + 1][z])),
                        neg_z: Face(s && (z == 0 || !chunk[x][y][z - 1])),
                        pos_z: Face(s && (z == 15 || !chunk[x][y][z + 1])),
                    };
                }
            }
        }
    // Return the 'faces' data, which represents the visibility of faces in a 3D space
        faces
    };

    // Creates a ChunkMeshBuilder and generates mesh data to represent the voxel-based 3D scene.
    let mut mesh_builder = ChunkMeshBuilder::new(SIDE);
    mesh_builder.add_chunk(Matrix4::one(), &faces);
    let (vertex, index) = mesh_builder.data();
    // Creates vertex and index buffers to store mesh data for rendering.
    let vertex = state
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            usage: wgpu::BufferUsages::VERTEX,
            contents: bytemuck::cast_slice(vertex),
        });
    let index_len = index.len();
    let index = state
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            usage: wgpu::BufferUsages::INDEX,
            contents: bytemuck::cast_slice(index),
        });
    // Initializes a Walker to generate data for voxel space.
    let walker = Walker::new();

    let trs = Sided {
        neg_x: translation(Vector3::new(-STEP, 0.0, 0.0)),
        pos_x: translation(Vector3::new(STEP, 0.0, 0.0)),
        neg_y: translation(Vector3::new(0.0, -STEP, 0.0)),
        pos_y: translation(Vector3::new(0.0, STEP, 0.0)),
        neg_z: translation(Vector3::new(0.0, 0.0, -STEP)),
        pos_z: translation(Vector3::new(0.0, 0.0, STEP)),
    };
    // Generates chunk data using the Walker to represent the voxel space.
    let mut chunk_data = Vec::new();

    walker.generate(
        (Matrix4::one(), 6),
        |_cell, &(tr, radius)| {
            chunk_data.push(InstanceTransformData::new(tr));
            radius > 0
        },
        |side, &(tr, radius)| (tr * trs[side], radius - 1),
    );

    let chunk_len = chunk_data.len();
    let chunk_data = state
        .device
        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            usage: wgpu::BufferUsages::VERTEX,
            contents: bytemuck::cast_slice(&chunk_data),
        });

    println!("{}", chunk_len);

    let gragas = include_bytes!("./resources/Gragas_Render.png");
    let gragas = image::load_from_memory(gragas).unwrap().into_rgba8();
    // Creates a texture with the image data.
    let extent = gragas.dimensions();
    let extent = wgpu::Extent3d {
        width: extent.0,
        height: extent.1,
        depth_or_array_layers: 1,
    };
    let texture = state.device.create_texture(&wgpu::TextureDescriptor {
        label: None,
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8UnormSrgb,
        usage: TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_DST,
        view_formats: &[],
    });
    state.queue.write_texture(
        wgpu::ImageCopyTexture {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &gragas,
        wgpu::ImageDataLayout {
            offset: 0,
            bytes_per_row: Some(4 * extent.width),
            rows_per_image: Some(extent.height),
        },
        wgpu::Extent3d {
            depth_or_array_layers: 1,
            ..extent
        },
    );

    let texture_view = texture.create_view(&wgpu::TextureViewDescriptor {
        label: None,
        dimension: Some(wgpu::TextureViewDimension::D2Array),
        ..Default::default()
    });
    let texture_sampler = state.device.create_sampler(&wgpu::SamplerDescriptor {
        label: None,
        address_mode_u: wgpu::AddressMode::Repeat,
        address_mode_v: wgpu::AddressMode::Repeat,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Nearest,
        min_filter: wgpu::FilterMode::Nearest,
        mipmap_filter: wgpu::FilterMode::Nearest,
        ..Default::default()
    });

    let texture_bind_group = state.device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: None,
        layout: &state.texture_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&texture_sampler),
            },
        ],
    });

    let mut time = Instant::now();
    // step is defining the time step for updating the simulation and rendering loop.
    // In the event loop, you'll notice that the application waits for a specific amount of time before updating the simulation and rendering again. This is done to achieve a consistent frame rate and avoid running the simulation and rendering loop as fast as possible, which could result in the application consuming excessive resources and generating high frame rates, leading to unnecessary GPU usage.
    // The time step is defined in nanoseconds (ns) and represents the target duration between frames. In this case, the time step is set to 16,666,666 ns, which is approximately 16.67 milliseconds (ms), equivalent to a frame rate of about 60 frames per second (FPS). This is a common target frame rate for many real-time applications, including games and simulations.
    // By using a fixed time step, the application can ensure that the simulation and rendering update at a consistent rate, even if the rendering takes more or less time for complex scenes or slow hardware. This can help achieve smooth and consistent frame rates, providing a better user experience.
    // During the event loop, the application calculates the elapsed time since the last frame and updates the simulation accordingly, so the movement and animation remain consistent regardless of the actual frame rate.
    let step = Duration::from_nanos(16_666_666);

    //The event loop listens for various events, such as mouse motion, keyboard input, window resizing, and redraw requests.
    //When a redraw request is received, the rendering process begins, and the scene is rendered using the defined render pipeline and the provided mesh data.
    event_loop.run(move |event, _, ctrl| {
        let _ = (&depth,);

        match event {
                        // When the program starts, this sets the control flow to 'Wait' to continue running.

            Event::DeviceEvent {
                            // When the main event loop becomes idle, this sets the control flow to 'Wait' to continue running.

                event: DeviceEvent::MouseMotion { delta },
                ..
            } => {
                if tracking {
                    let dx = delta.0 as f64;
                    let dy = delta.1 as f64;
                    // Adjust sensitivity and rotation speed as needed
                    camera.apply_rotation(Vector2::new(dy * 0.005, -dx * 0.005));
                    camera.commit(&state.queue);
                }
            }
            Event::WindowEvent {
                event: WindowEvent::CursorEntered { .. },
                ..
            } => {
                tracking = true;
                state.window.set_cursor_visible(false);
            }
            Event::WindowEvent {
                event: WindowEvent::CursorLeft { .. },
                ..
            } => {
                tracking = false;
                state.window.set_cursor_visible(true);
            }
            Event::NewEvents(StartCause::Init) => {
                time = Instant::now();
                *ctrl = ControlFlow::WaitUntil(time + step);
                                // Redraw the window when idle.

                state.window.request_redraw();
            }
                        // When the window is redrawn, this triggers the rendering process.

            Event::NewEvents(StartCause::ResumeTimeReached { .. }) => {
                let delta_time = {
                    let prev = std::mem::replace(&mut time, Instant::now());
                    *ctrl = ControlFlow::WaitUntil(time + step);
                    (time - prev).as_secs_f64()
                };

                if let Some(delta) = input.delta() {
                    let delta = delta.normalize() * delta_time * STEP / 16.0 * 5.0;
                    camera.apply_transform(Matrix4::from_translation(delta));
                    camera.commit(&state.queue);
                }

                state.window.request_redraw();
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(size),
                ..
            } => {
                state.resize(size.width, size.height);
                depth = state.device.create_texture(&wgpu::TextureDescriptor {
                    label: None,
                    size: wgpu::Extent3d {
                        width: size.width,
                        height: size.height,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Depth32Float,
                    usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                });
                depth_view = depth.create_view(&wgpu::TextureViewDescriptor::default());

                camera.viewport.aspect = size.width as f64 / size.height as f64;
                camera.commit(&state.queue);
            }
            Event::WindowEvent {
                event:
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state, scancode, ..
                            },
                        ..
                    },
                ..
            } => input.update(
                scancode,
                matches!(state, winit::event::ElementState::Pressed),
            ),
            Event::RedrawRequested(_) => {
                let frame = state.surface.get_current_texture().unwrap();
                let view = frame.texture.create_view(&Default::default());
                let mut encoder = state
                    .device
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: None,
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                            view: &depth_view,
                            depth_ops: Some(wgpu::Operations {
                                load: wgpu::LoadOp::Clear(1.0),
                                store: true,
                            }),
                            stencil_ops: None,
                        }),
                    });
                    rpass.set_pipeline(&pipeline);
                    camera_bind_group.set_bind_group(&mut rpass, 0);
                    rpass.set_bind_group(1, &texture_bind_group, &[]);
                    rpass.set_vertex_buffer(0, vertex.slice(..));
                    rpass.set_vertex_buffer(1, chunk_data.slice(..));
                    rpass.set_index_buffer(index.slice(..), wgpu::IndexFormat::Uint32);
                    rpass.draw_indexed(0..index_len as _, 0, 0..chunk_len as _);
                }
                state.queue.submit(Some(encoder.finish()));
                frame.present();
            }
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *ctrl = ControlFlow::Exit,
            _ => {}
        }
    });
}
