use crate::{
    model,
    pipeline::{Mesh, PointCloud, Wireframe},
    Element, IntoElement, Key, WindowState,
};

use ply_rs::{parser::Parser, ply};
use std::{
    collections::{HashMap, HashSet},
    io::BufRead,
    mem,
};

pub trait RenderArtifact {
    fn update_count(&mut self, header: &ply::Header);
    fn create_pipeline_layout(
        device: &wgpu::Device,
        world_bind_group_layout: &wgpu::BindGroupLayout,
        artifact_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::PipelineLayout;

    fn create_pipeline(device: &wgpu::Device, playback: &WindowState) -> wgpu::RenderPipeline;

    fn create_uniform_buffer(device: &wgpu::Device) -> wgpu::Buffer;
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ArtifactUniform {
    color: [f32; 4],
}

impl ArtifactUniform {
    pub fn new(color: [f32; 4]) -> Self {
        Self { color }
    }
}

pub enum Artifact {
    PointCloud(PointCloud),
    Wireframe(Wireframe),
    Mesh(Mesh),
}

impl Artifact {
    pub fn new(device: &wgpu::Device, key: &Key, header: &ply::Header) -> Option<Artifact> {
        // Interrogate the header to figure out if we have a point cloud,
        // mesh, or something else.
        let keys: HashSet<Element> = header
            .elements
            .keys()
            .filter_map(|key| Element::from(key))
            .collect();

        let elements: HashMap<Element, &ply::ElementDef> = keys
            .iter()
            .map(|e| (*e, header.elements.get(&e.to_string()).unwrap()))
            .collect();

        if keys == HashSet::from([Element::Vertex]) {
            let element_size = mem::size_of::<model::PlainVertex>();
            let count = elements.get(&Element::Vertex).unwrap().count;
            let vertices = device.create_buffer(&wgpu::BufferDescriptor {
                mapped_at_creation: false,
                size: (2 * element_size * count) as u64,
                label: Some(&key.artifact),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

            return Some(Artifact::PointCloud(PointCloud {
                vertices,
                num_vertices: count as u32,
            }));
        }

        // We need a discriminant for mesh vs. wireframe somehow.
        if keys == HashSet::from([Element::Vertex, Element::Facet]) {
            let element_size = mem::size_of::<model::PlainVertex>();
            let count = elements.get(&Element::Vertex).unwrap().count;
            let vertices = device.create_buffer(&wgpu::BufferDescriptor {
                mapped_at_creation: false,
                size: (2 * element_size * count) as u64,
                label: Some(&key.artifact),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

            let element_size = mem::size_of::<model::TriFacet>();
            let count = elements.get(&Element::Facet).unwrap().count;
            let indices = device.create_buffer(&wgpu::BufferDescriptor {
                mapped_at_creation: false,
                size: (4 * element_size * count) as u64,
                label: Some(&key.artifact),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            });

            return Some(Artifact::Wireframe(Wireframe {
                vertices,
                indices,
                num_lines: count as u32 / 2,
            }));
        }

        None
    }

    pub fn needs_resize(&self, header: &ply::Header) -> bool {
        match self {
            Artifact::PointCloud(PointCloud { vertices, .. }) => {
                model::PlainVertex::buffer_too_small(&header, vertices)
            }
            Artifact::Wireframe(Wireframe {
                vertices, indices, ..
            }) => {
                model::PlainVertex::buffer_too_small(&header, vertices)
                    || model::Wireframe::buffer_too_small(&header, indices)
            }
            Artifact::Mesh(Mesh {
                vertices, indices, ..
            }) => {
                model::PlainVertex::buffer_too_small(&header, vertices)
                    || model::Wireframe::buffer_too_small(&header, indices)
            }
        }
    }

    pub fn write_buffer(&self, queue: &wgpu::Queue, f: &mut impl BufRead, header: &ply::Header) {
        match self {
            Artifact::PointCloud(PointCloud { vertices, .. }) => {
                let parse = Parser::<model::PlainVertex>::new();
                let element = header.elements.get(&Element::Vertex.to_string()).unwrap();
                let data = parse
                    .read_payload_for_element(f, &element, &header)
                    .unwrap();
                queue.write_buffer(&vertices, 0, bytemuck::cast_slice(&data));
            }
            Artifact::Wireframe(Wireframe {
                vertices, indices, ..
            }) => {
                let vertex_element = match header.elements.get(&Element::Vertex.to_string()) {
                    Some(e) => e,
                    None => return,
                };
                let index_element = match header.elements.get(&Element::Facet.to_string()) {
                    Some(e) => e,
                    None => return,
                };

                let parse = Parser::<model::PlainVertex>::new();
                let data = parse
                    .read_payload_for_element(f, &vertex_element, &header)
                    .unwrap();
                queue.write_buffer(&vertices, 0, bytemuck::cast_slice(&data));

                let parse = Parser::<model::Wireframe>::new();
                let data = parse
                    .read_payload_for_element(f, &index_element, &header)
                    .unwrap();
                queue.write_buffer(&indices, 0, bytemuck::cast_slice(&data));
            }
            Artifact::Mesh(Mesh {
                vertices, indices, ..
            }) => {
                let parse = Parser::<model::PlainVertex>::new();
                let element = header.elements.get(&Element::Vertex.to_string()).unwrap();
                let data = parse
                    .read_payload_for_element(f, &element, &header)
                    .unwrap();
                queue.write_buffer(&vertices, 0, bytemuck::cast_slice(&data));

                let parse = Parser::<model::TriFacet>::new();
                let element = header.elements.get(&Element::Facet.to_string()).unwrap();
                let data = parse
                    .read_payload_for_element(f, &element, &header)
                    .unwrap();
                queue.write_buffer(&indices, 0, bytemuck::cast_slice(&data));
            }
        }
    }

    pub fn update_count(&mut self, header: &ply::Header) {
        match self {
            Artifact::PointCloud(point_cloud) => point_cloud.update_count(header),
            Artifact::Wireframe(wireframe) => wireframe.update_count(header),
            Artifact::Mesh(mesh) => mesh.update_count(header),
        }
    }

    pub fn create_uniform_buffer(&self, device: &wgpu::Device) -> wgpu::Buffer {
        match self {
            Artifact::PointCloud(_) => PointCloud::create_uniform_buffer(&device),
            Artifact::Wireframe(_) => Wireframe::create_uniform_buffer(&device),
            Artifact::Mesh(_) => Mesh::create_uniform_buffer(&device),
        }
    }

    pub fn create_pipeline(
        &self,
        device: &wgpu::Device,
        state: &WindowState,
    ) -> wgpu::RenderPipeline {
        match self {
            Artifact::PointCloud(_) => PointCloud::create_pipeline(&device, &state),
            Artifact::Wireframe(_) => Wireframe::create_pipeline(&device, &state),
            Artifact::Mesh(_) => Mesh::create_pipeline(&device, &state),
        }
    }
}
