use std::mem;
use ply_rs::ply;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct PlainVertex {
    pub position: [f32; 3],
}

// Teach wgpu how model a vertex.
impl PlainVertex {
    const ATTRIBS: [wgpu::VertexAttribute; 1] = wgpu::vertex_attr_array![0 => Float32x3];

    pub fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<PlainVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &Self::ATTRIBS,
        }
    }
}

// Teach ply_rs how model a vertex.
impl ply::PropertyAccess for PlainVertex {
    fn new() -> Self {
        PlainVertex { position: [0.0, 0.0, 0.0] }
    }

    fn set_property(&mut self, key: String, property: ply::Property) {
        match (key.as_ref(), property) {
            ("x", ply::Property::Float(v)) => self.position[0] = v,
            ("y", ply::Property::Float(v)) => self.position[1] = v,
            ("z", ply::Property::Float(v)) => self.position[2] = v,
            (_, _) => {}
        }
    }
}
