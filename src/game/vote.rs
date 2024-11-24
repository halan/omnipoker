#[derive(Clone, Debug, PartialEq)]
pub enum Vote {
    Null,
    Unknown,
    Option(usize),
}

impl From<&str> for Vote {
    fn from(value: &str) -> Self {
        match value {
            "?" => Vote::Unknown,
            text => match text.trim().parse() {
                Ok(value) => Vote::new(value),
                Err(_) => Vote::Null,
            },
        }
    }
}

impl Vote {
    fn new(value: usize) -> Self {
        match value {
            // Fibonacci sequence
            1 | 2 | 3 | 5 | 8 | 13 => Vote::Option(value),
            _ => Vote::Null,
        }
    }

    pub fn status(&self) -> &str {
        match self {
            Vote::Null => "not voted",
            _ => "voted",
        }
    }
}

impl std::fmt::Display for Vote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Vote::Null => write!(f, "{}", self.status()),
            Vote::Unknown => write!(f, "?"),
            Vote::Option(value) => write!(f, "{}", value),
        }
    }
}
