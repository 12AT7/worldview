use crate::{model, ArtifactUniform, RenderArtifact, WindowState, Element};
use ply_rs::ply;
use wgpu::util::DeviceExt;

pub struct PointCloud {
    pub vertices: wgpu::Buffer,
    pub num_vertices: u32
}

impl PointCloud {
    pub fn render<'rpass>(
        &'rpass self,
        // vertices: &'rpass wgpu::Buffer,
        render_pass: &mut wgpu::RenderPass<'rpass>,
    ) {
        // let num_vertices = vertices.size() / mem::size_of::<model::PlainVertex>() as u64;
        render_pass.set_vertex_buffer(0, self.vertices.slice(..));
        render_pass.draw(0..self.num_vertices, 0..1);
    }
}

impl RenderArtifact for PointCloud {
    fn update_count(&mut self, header: &ply::Header) {
        self.num_vertices = header.elements.get(&Element::Vertex.to_string()).unwrap().count as u32;
    }

    fn create_pipeline_layout(
        device: &wgpu::Device,
        world_bind_group_layout: &wgpu::BindGroupLayout,
        artifact_bind_group_layout: &wgpu::BindGroupLayout,
    ) -> wgpu::PipelineLayout {
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("point_cloud::pipeline_layout"),
            bind_group_layouts: &[&world_bind_group_layout, &artifact_bind_group_layout],
            push_constant_ranges: &[],
        })
    }

    fn create_pipeline(device: &wgpu::Device, state: &WindowState) -> wgpu::RenderPipeline {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("point_cloud::shader"),
            source: wgpu::ShaderSource::Wgsl(
                (include_str!("shader/plain_geometry.wsgl").to_owned()).into(),
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

    fn create_uniform_buffer(device: &wgpu::Device) -> wgpu::Buffer {
        let uniform = ArtifactUniform::new([0.0, 1.0, 0.0, 1.0]);
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("point_cloud::uniform_buffer"),
            contents: bytemuck::cast_slice(&[uniform]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        })
    }

}
