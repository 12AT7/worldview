use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key {
    pub frame: u32,
    pub artifact: String,
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}] {}", self.frame, self.artifact)
    }
}


