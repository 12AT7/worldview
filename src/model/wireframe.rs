use crate::{Element, IntoElement};
use ply_rs::ply;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Wireframe {
    pub vertex_indices: [i32; 6],
}
//
// Teach worldview how to find the vertex in the PLY header
impl IntoElement for Wireframe {
    fn element() -> Element { Element::Facet }
}

// Teach ply_rs how model a wireframe facet.
impl ply::PropertyAccess for Wireframe {
    fn new() -> Self {
        Wireframe {
            vertex_indices: [0, 0, 0, 0, 0, 0],
        }
    }

    fn set_property(&mut self, key: String, property: ply::Property) {
        match (key.as_ref(), property) {
            ("vertex_indices", ply::Property::ListInt(vec)) => {
                if vec.len() == 3 {
                    self.vertex_indices = [vec[0], vec[1], vec[1], vec[2], vec[2], vec[0]];
                } else {
                    panic!("Wrong number of indices");
                }
            }
            (_, _) => {}
        }
    }
}
