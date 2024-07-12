use crate::{
    model,
    pipeline::{Mesh, PointCloud, Wireframe},
    Element, Key, WindowState,
};

use ply_rs::{parser::Parser, ply};
use std::{
    collections::{HashMap, HashSet},
    io::BufRead,
    mem,
};

pub trait RenderArtifact {
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
            .map(|e| (*e, header.elements.get(&String::from(*e)).unwrap()))
            .collect();

        // if keys == HashSet::from([Element::Vertex]) {
        if keys.len() == 1 {
            let element_size = mem::size_of::<model::PlainVertex>();
            let count = elements.get(&Element::Vertex).unwrap().count;
            let vertices = device.create_buffer(&wgpu::BufferDescriptor {
                mapped_at_creation: false,
                size: (2 * element_size * count) as u64,
                label: Some(&key.artifact),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

            return Some(Artifact::PointCloud(PointCloud { vertices }));
        }

        // We need a discriminant for mesh vs. wireframe somehow.
        // if keys == HashSet::from([Element::Vertex, Element::Face]) {
        if keys.len() == 2 {
            let element_size = mem::size_of::<model::PlainVertex>();
            let count = elements.get(&Element::Vertex).unwrap().count;
            let vertices = device.create_buffer(&wgpu::BufferDescriptor {
                mapped_at_creation: false,
                size: (4 * element_size * count) as u64,
                label: Some(&key.artifact),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });

            let element_size = mem::size_of::<model::TriFacet>();
            let count = elements.get(&Element::Face).unwrap().count;
            let indices = device.create_buffer(&wgpu::BufferDescriptor {
                mapped_at_creation: false,
                size: (4 * element_size * count) as u64,
                label: Some(&key.artifact),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            });

            return Some(Artifact::Wireframe(Wireframe { vertices, indices }));
        }

        None
    }

    pub fn needs_resize(&self, header: &ply::Header) -> bool {
        return true; // This is buggy for now.
        match self {
            Artifact::PointCloud(PointCloud { vertices }) => {
                let count = header
                    .elements
                    .get(&String::from(Element::Vertex))
                    .unwrap()
                    .count;
                let element_size = mem::size_of::<model::PlainVertex>();
                vertices.size() as usize <= element_size * count
            }
            Artifact::Wireframe(Wireframe { vertices, indices }) => {
                let vertex_count = header
                    .elements
                    .get(&String::from(Element::Vertex))
                    .unwrap()
                    .count;
                let vertex_size = mem::size_of::<model::PlainVertex>();

                let index_count = header
                    .elements
                    .get(&String::from(Element::Face))
                    .unwrap()
                    .count;
                let index_size = mem::size_of::<model::Wireframe>();

                (vertices.size() as usize <= vertex_size * vertex_count)
                    || (indices.size() as usize <= index_size * index_count)
            }
            Artifact::Mesh(Mesh { vertices, indices }) => {
                let vertex_count = header
                    .elements
                    .get(&String::from(Element::Vertex))
                    .unwrap()
                    .count;
                let vertex_size = mem::size_of::<model::PlainVertex>();

                let index_count = header
                    .elements
                    .get(&String::from(Element::Face))
                    .unwrap()
                    .count;
                let index_size = 12;

                (vertices.size() as usize <= vertex_size * vertex_count)
                    || (indices.size() as usize <= index_size * index_count)
            }
        }
    }

    pub fn write_buffer(&self, queue: &wgpu::Queue, f: &mut impl BufRead, header: &ply::Header) {
        match self {
            Artifact::PointCloud(PointCloud { vertices }) => {
                let parse = Parser::<model::PlainVertex>::new();
                let element = header.elements.get(&String::from(Element::Vertex)).unwrap();
                let data = parse
                    .read_payload_for_element(f, &element, &header)
                    .unwrap();
                queue.write_buffer(&vertices, 0, bytemuck::cast_slice(&data));
            }
            Artifact::Wireframe(Wireframe { vertices, indices }) => {
                let parse = Parser::<model::PlainVertex>::new();
                let element = header.elements.get(&String::from(Element::Vertex)).unwrap();
                let data = parse
                    .read_payload_for_element(f, &element, &header)
                    .unwrap();
                queue.write_buffer(&vertices, 0, bytemuck::cast_slice(&data));

                let parse = Parser::<model::Wireframe>::new();
                let element = header.elements.get(&String::from(Element::Face)).unwrap();
                let data = parse
                    .read_payload_for_element(f, &element, &header)
                    .unwrap();
                queue.write_buffer(&indices, 0, bytemuck::cast_slice(&data));
            }
            Artifact::Mesh(Mesh { vertices, indices }) => {
                let parse = Parser::<model::PlainVertex>::new();
                let element = header.elements.get(&String::from(Element::Vertex)).unwrap();
                let data = parse
                    .read_payload_for_element(f, &element, &header)
                    .unwrap();
                queue.write_buffer(&vertices, 0, bytemuck::cast_slice(&data));

                let parse = Parser::<model::TriFacet>::new();
                let element = header.elements.get(&String::from(Element::Face)).unwrap();
                let data = parse
                    .read_payload_for_element(f, &element, &header)
                    .unwrap();
                queue.write_buffer(&indices, 0, bytemuck::cast_slice(&data));
            }
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
