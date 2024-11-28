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
    pub fn new(value: usize) -> Self {
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

    pub fn is_valid_vote(&self) -> bool {
        *self != Vote::Null
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
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vote_from_str() {
        assert_eq!(Vote::from("?"), Vote::Unknown);
        assert_eq!(Vote::from("1"), Vote::Option(1));
        assert_eq!(Vote::from("2"), Vote::Option(2));
        assert_eq!(Vote::from("3"), Vote::Option(3));
        assert_eq!(Vote::from("5"), Vote::Option(5));
        assert_eq!(Vote::from("8"), Vote::Option(8));
        assert_eq!(Vote::from("13"), Vote::Option(13));
        assert_eq!(Vote::from("21"), Vote::Null);
        assert_eq!(Vote::from("invalid"), Vote::Null);
    }

    #[test]
    fn test_vote_new() {
        assert_eq!(Vote::new(1), Vote::Option(1));
        assert_eq!(Vote::new(2), Vote::Option(2));
        assert_eq!(Vote::new(3), Vote::Option(3));
        assert_eq!(Vote::new(5), Vote::Option(5));
        assert_eq!(Vote::new(8), Vote::Option(8));
        assert_eq!(Vote::new(13), Vote::Option(13));
        assert_eq!(Vote::new(21), Vote::Null);
    }

    #[test]
    fn test_vote_status() {
        assert_eq!(Vote::Null.status(), "not voted");
        assert_eq!(Vote::Unknown.status(), "voted");
        assert_eq!(Vote::Option(1).status(), "voted");
    }

    #[test]
    fn test_vote_display() {
        assert_eq!(format!("{}", Vote::Null), "not voted");
        assert_eq!(format!("{}", Vote::Unknown), "?");
        assert_eq!(format!("{}", Vote::Option(1)), "1");
        assert_eq!(format!("{}", Vote::Option(8)), "8");
    }

    #[test]
    fn test_vote_is_valid() {
        assert!(!Vote::Null.is_valid_vote());
        assert!(Vote::Unknown.is_valid_vote());
        assert!(Vote::Option(1).is_valid_vote());
    }
}
