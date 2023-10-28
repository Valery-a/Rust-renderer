use crate::{
    mesh::{MeshData, Vertex},
    renderer::Renderer,
};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Debug)]
pub struct Mesh {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub len: u32,
}

impl Mesh {
    pub fn from_mesh_data(renderer: &Renderer, mesh_data: MeshData) -> Self {
        let vertex_buffer = renderer.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(mesh_data.vertices.as_slice()),
            usage: wgpu::BufferUsage::VERTEX,
        });
        let index_buffer = renderer.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: None,
            contents: bytemuck::cast_slice(mesh_data.indices.as_slice()),
            usage: wgpu::BufferUsage::INDEX,
        });
        Self {
            vertex_buffer,
            index_buffer,
            len: mesh_data.indices.len() as u32,
        }
    }
}

impl Vertex {
    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<Self>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: 2 * mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ],
        }
    }
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}
