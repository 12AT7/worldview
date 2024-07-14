use crate::{
    pipeline::{Mesh, PointCloud, Wireframe},
    WindowState,
};

use std::io::BufRead;

use ply_rs::ply;

pub trait RenderArtifact {
    fn update_count(&mut self, header: &ply::Header);
    fn create_pipeline_layout(
        device: &wgpu::Device,
        world_bind_group_layout: &wgpu::BindGroupLayout,
        artifact_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::PipelineLayout;

    fn create_pipeline(device: &wgpu::Device, playback: &WindowState) -> wgpu::RenderPipeline;

    fn create_uniform_buffer(device: &wgpu::Device) -> wgpu::Buffer;
    fn needs_resize(&self, header: &ply::Header) -> bool;
    fn read_ply(&mut self, f: &mut impl BufRead, header: &ply::Header);
    fn write_buffer(&self, queue: &wgpu::Queue);
    fn render<'rpass>(&'rpass self, render_pass: &mut wgpu::RenderPass<'rpass>);
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
    pub fn new(device: &wgpu::Device, header: &ply::Header) -> Option<Artifact> {
        // Detect which artifact type we want to show, given the PLY header.
        if let Some(wireframe) = Wireframe::new(&device, &header) {
            return Some(Artifact::Wireframe(wireframe));
        }

        if let Some(point_cloud) = PointCloud::new(&device, &header) {
            return Some(Artifact::PointCloud(point_cloud));
        }

        None
    }

    pub fn needs_resize(&self, header: &ply::Header) -> bool {
        match self {
            Artifact::PointCloud(point_cloud) => point_cloud.needs_resize(&header),
            Artifact::Mesh(mesh) => mesh.needs_resize(&header),
            Artifact::Wireframe(wireframe) => wireframe.needs_resize(&header),
        }
    }

    pub fn read_ply(&mut self, f: &mut impl BufRead, header: &ply::Header) {
        match self {
            Artifact::PointCloud(point_cloud) => point_cloud.read_ply(f, &header),
            Artifact::Wireframe(wireframe) => wireframe.read_ply(f, &header),
            Artifact::Mesh(mesh) => mesh.read_ply(f, &header),
        }
    }

    pub fn write_buffer(&self, queue: &wgpu::Queue) {
        match self {
            Artifact::PointCloud(point_cloud) => point_cloud.write_buffer(&queue),
            Artifact::Wireframe(wireframe) => wireframe.write_buffer(&queue),
            Artifact::Mesh(mesh) => mesh.write_buffer(&queue),
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
