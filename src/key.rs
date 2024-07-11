use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Key {
    pub instance: Option<u32>, // Frame number, or tile hash
    pub artifact: String,
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self.instance {
            Some(u) => write!(f, "[{}] {}", u, self.artifact),
            None => write!(f, "{}", self.artifact)
        }
    }
}


