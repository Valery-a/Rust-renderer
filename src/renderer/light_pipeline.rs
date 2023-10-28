use crate::{
    mesh::Vertex,
    registry::{Handle, Registry},
    renderer::{
        depth_texture::DepthTexture, error::RendererError, light_bindgroup::Instance, Camera, Light, LightBindGroup,
        Mesh, Renderer,
    },
};
use glam::{Mat4, Vec3};
use std::borrow::Cow;

pub struct LightPipeline {
    render_pipeline: wgpu::RenderPipeline,
}

impl LightPipeline {
    pub async fn new(renderer: &Renderer, bind_group: &LightBindGroup) -> Result<Self, RendererError> {
        let shader = renderer.device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shaders/light_shader.wgsl"))),
            flags: wgpu::ShaderFlags::EXPERIMENTAL_TRANSLATION,
        });
        let render_pipeline_layout = renderer.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&bind_group.bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = renderer.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc()],
            },
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                clamp_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DepthTexture::DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState {
                    front: wgpu::StencilFaceState::IGNORE,
                    back: wgpu::StencilFaceState::IGNORE,
                    read_mask: 0,
                    write_mask: 0,
                },
                bias: wgpu::DepthBiasState {
                    constant: 0,
                    slope_scale: 0.0,
                    clamp: 0.0,
                },
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[renderer.swap_chain_descriptor.format.into()],
            }),
        });
        Ok(Self { render_pipeline })
    }

    pub fn render(
        &self,
        light_handle: &Handle<Mesh>,
        lights: &Registry<Light>,
        bindgroup: &LightBindGroup,
        camera: &dyn Camera,
        meshes: &Registry<Mesh>,
        renderer: &mut Renderer,
        target: &wgpu::TextureView,
    ) {
        bindgroup.update_uniforms(&renderer, camera);
        let mut transforms = Vec::new();
        for (_, light) in &lights.registry {
            match light {
                Light::Spot(properties) => {
                    let m = Mat4::from_translation(Vec3::new(
                        properties.position[0],
                        properties.position[1],
                        properties.position[2],
                    ));
                    let inv_m = m.inverse();
                    transforms.push(Instance { m, inv_m });
                }
                Light::Point(properties) => {
                    let m = Mat4::from_translation(Vec3::new(
                        properties.position[0],
                        properties.position[1],
                        properties.position[2],
                    ));
                    let inv_m = m.inverse();
                    transforms.push(Instance { m, inv_m });
                }
                _ => (),
            }
        }
        bindgroup.update_instances(&renderer, transforms.as_slice());
        let mut encoder = renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: None,
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &target,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &renderer.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            let vb = meshes.get(light_handle).unwrap();
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffer(0, vb.vertex_buffer.slice(..));
            render_pass.set_index_buffer(vb.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            render_pass.set_bind_group(0, &bindgroup.bind_group, &[]);
            render_pass.draw_indexed(0..vb.len, 0, 0..transforms.len() as u32);
        }
        renderer.queue.submit(std::iter::once(encoder.finish()));
    }
}
