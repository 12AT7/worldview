use crate::{Element, IntoElement};
use ply_rs::ply;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TriFacet {
    pub vertex_indices: [i32; 3],
}

// Teach worldview how to find the facet in the PLY header
impl IntoElement for TriFacet {
    fn element() -> Element { Element::Facet }
}

// Teach ply_rs how model a facet.
impl ply::PropertyAccess for TriFacet {
    fn new() -> Self {
        TriFacet {
            vertex_indices: [0, 0, 0],
        }
    }

    fn set_property(&mut self, key: String, property: ply::Property) {
        match (key.as_ref(), property) {
            ("vertex_indices", ply::Property::ListInt(vec)) => {
                if vec.len() == 3 {
                    self.vertex_indices = [vec[0], vec[1], vec[2]];
                }
            }
            (_, _) => {}
        }
    }
}
