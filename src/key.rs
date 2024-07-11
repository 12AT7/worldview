use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key {
    pub instance: u32, // Frame number, or tile hash
    pub artifact: String,
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}] {}", self.instance, self.artifact)
    }
}


