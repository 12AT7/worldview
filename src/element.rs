#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Element {
    Vertex,
    Face,
}

impl Element {
    pub fn from(e: &String) -> Option<Element> {
        match e.as_ref() {
            "vertex" => Some(Element::Vertex),
            "face" => Some(Element::Face),
            _ => None
        }
    }
}

impl From<Element> for String {
    fn from(e: Element) -> String {
        match e {
            Element::Vertex => "vertex".to_string(),
            Element::Face => "face".to_string(),
        }
    }
}


