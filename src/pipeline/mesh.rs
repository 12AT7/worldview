use crate::{model, Playback, RenderArtifact, WindowState};

use std::mem;
use wgpu;

pub struct Mesh {
    pub vertices: wgpu::Buffer,
    pub indices: wgpu::Buffer,
}

impl Mesh {
    pub fn create_pipeline_layout(
        device: &wgpu::Device,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("mesh::pipeline_layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        })
    }

    pub fn create_pipeline(device: &wgpu::Device, state: &WindowState) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("mesh::shader"),
            source: wgpu::ShaderSource::Wgsl(
                (include_str!("shader/mesh.wsgl").to_owned()).into(),
            ),
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("mesh::render_pipeline"),
            layout: Some(&state.mesh_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                compilation_options: Default::default(),
                entry_point: "vs_main",
                buffers: &[model::PlainVertex::desc()],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                compilation_options: Default::default(),
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: state.surface_capabilities.formats[0],
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        })
    }

    pub fn render<'rpass>(
        vertices: &'rpass wgpu::Buffer,
        indices: &'rpass wgpu::Buffer,
        state: &'rpass WindowState,
        render_pass: &mut wgpu::RenderPass<'rpass>,
    ) {
        let num_facets = indices.size() / 8 as u64;
        render_pass.set_bind_group(0, &state.camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertices.slice(..));
        render_pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..num_facets as u32, 0, 0..1);
    }
}
