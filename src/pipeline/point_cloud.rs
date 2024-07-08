use crate::{model, Playback, RenderArtifact, WindowState};

use std::mem;
use wgpu;

pub struct PointCloud {
    pub vertices: wgpu::Buffer,
}

impl RenderArtifact for PointCloud {
    fn create_pipeline_layout(
        device: &wgpu::Device,
        camera_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("point_cloud::pipeline_layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        })
    }

    fn create_pipeline(
        device: &wgpu::Device,
        state: &WindowState, // layout: &wgpu::PipelineLayout,
                             // surface_capabilities: &wgpu::SurfaceCapabilities,
    ) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("point_cloud::shader"),
            source: wgpu::ShaderSource::Wgsl(
                (include_str!("shader/point_cloud.wsgl").to_owned()).into(),
            ),
        });

        device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("point_cloud::render_pipeline"),
            layout: Some(&state.point_cloud_pipeline_layout),
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
                topology: wgpu::PrimitiveTopology::PointList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        })
    }

    fn render<'rpass>(
        vertices: &'rpass wgpu::Buffer,
        state: &'rpass WindowState,
        render_pass: &mut wgpu::RenderPass<'rpass>,
    ) {
        let num_vertices = vertices.size() / mem::size_of::<model::PlainVertex>() as u64;
        render_pass.set_bind_group(0, &state.camera_bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertices.slice(..));
        render_pass.draw(0..num_vertices as u32, 0..1);
    }
}
