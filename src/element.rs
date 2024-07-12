// PLY files support "elements" with arbitrary names.  However, we need
// to know what the element actually is and what it can do, without the
// ambiguity of the name which is not even consistent across PLY utilities.
// Element is a enum that fixes specifically what elements we support,
// and how they appear in PLY files.

use ply_rs::ply;
use std::mem;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Element {
    Vertex,
    Facet,
}

pub trait IntoElement {
    fn element() -> Element;
    fn buffer_too_small(header: &ply::Header, buffer: &wgpu::Buffer) -> bool
    where
        Self: Sized,
    {
        let element_size = mem::size_of::<Self>();
        let element_name = Self::element().to_string();
        let element_count = match header.elements.get(&element_name) {
            Some(element) => element.count,
            None => return false, // Cannot allocate buffer anyway
        };
        let buffer_size = buffer.size() as usize;
        buffer_size < element_size * element_count
    }
}

impl Element {
    pub fn from(e: &String) -> Option<Element> {
        match e.as_ref() {
            "vertex" => Some(Element::Vertex),
            "face" => Some(Element::Facet),
            _ => None,
        }
    }
}

impl std::string::ToString for Element {
    fn to_string(&self) -> String {
        match self {
            Element::Vertex => "vertex".to_string(),
            Element::Facet => "face".to_string(),
        }
    }
}
