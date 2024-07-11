use crate::{model, WindowState, ArtifactUniform};

use wgpu::util::DeviceExt;

pub struct Wireframe {
    pub vertices: wgpu::Buffer,
    pub indices: wgpu::Buffer,
}

impl Wireframe {
    pub fn create_pipeline_layout(
        device: &wgpu::Device,
        world_bind_group_layout: &wgpu::BindGroupLayout,
        artifact_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("wireframe::pipeline_layout"),
            bind_group_layouts: &[&world_bind_group_layout, &artifact_bind_group_layout],
            push_constant_ranges: &[],
        })
    }

    pub fn create_pipeline(device: &wgpu::Device, state: &WindowState) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wireframe::shader"),
            source: wgpu::ShaderSource::Wgsl(
                (include_str!("shader/plain_geometry.wsgl").to_owned()).into(),
            ),
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wireframe::render_pipeline"),
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

    pub fn create_uniform_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        let uniform = ArtifactUniform::new([1.0, 0.0, 0.0, 1.0]);
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("wireframe::uniform_buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }

    pub fn render<'rpass>(
        vertices: &'rpass wgpu::Buffer,
        indices: &'rpass wgpu::Buffer,
        render_pass: &mut wgpu::RenderPass<'rpass>,
    ) {
        let num_lines = indices.size() / 8 as u64;
        render_pass.set_vertex_buffer(0, vertices.slice(..));
        render_pass.set_index_buffer(indices.slice(..), wgpu::IndexFormat::Uint32);
        render_pass.draw_indexed(0..num_lines as u32, 0, 0..1);
    }
}
