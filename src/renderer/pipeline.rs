use crate::{
    entity::Entity,
    mesh::Vertex,
    registry::{Handle, Registry},
    renderer::{
        bindgroup::Instance, depth_texture::DepthTexture, error::RendererError, mesh::Mesh, BindGroup, Camera, Light,
        Renderer,
    },
    world::World,
};
use std::borrow::Cow;

pub struct Pipeline {
    render_pipeline: wgpu::RenderPipeline,
}

impl Pipeline {
    pub async fn new(renderer: &Renderer, bind_group: &BindGroup) -> Result<Self, RendererError> {
        let shader = renderer.device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: None,
            source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(include_str!("shaders/shader.wgsl"))),
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
        world: &World,
        entities: &Registry<Entity>,
        meshes: &mut Registry<Mesh>,
        lights: &Registry<Light>,
        bindgroup: &BindGroup,
        camera: &dyn Camera,
        renderer: &mut Renderer,
        target: &wgpu::TextureView,
        position: [f32; 3],
    ) {
        bindgroup.update_uniforms(&renderer, &lights, camera);
        let mut instance_map = Vec::new();
        let mut start_range = 0;
        let mut transforms = Vec::new();
        for (id, _) in &mut meshes.registry {
            transforms.extend_from_slice(
                entities
                    .registry
                    .iter()
                    .filter_map(|(_, v)| {
                        if v.mesh_handle.id == *id {
                            let m = v.transform.to_matrix();
                            let inv_m = m.inverse();
                            Some(Instance { m, inv_m })
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .as_slice(),
            );
            instance_map.push((Handle::<Mesh>::new(*id), start_range..transforms.len() as u32));
            start_range = transforms.len() as u32;
        }
        for (handle, transform) in world.get_within_view_mesh_transform([position[0], position[2]]) {
            let m = transform.to_matrix();
            let inv_m = m.inverse();
            transforms.push(Instance { m, inv_m });
            instance_map.push((handle, start_range..start_range + 1));
            start_range = transforms.len() as u32;
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
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &renderer.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            for (mesh_handle, instance_range) in instance_map {
                if !instance_range.is_empty() {
                    let mesh = meshes.get(&mesh_handle).unwrap();
                    render_pass.set_pipeline(&self.render_pipeline);
                    render_pass.set_vertex_buffer(0, mesh.vertex_buffer.slice(..));
                    render_pass.set_index_buffer(mesh.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                    render_pass.set_bind_group(0, &bindgroup.bind_group, &[]);
                    render_pass.draw_indexed(0..mesh.len, 0, instance_range);
                }
            }
        }
        renderer.queue.submit(std::iter::once(encoder.finish()));
    }
}
