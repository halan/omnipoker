use serde::de::Deserializer;
use serde::{Deserialize, Serialize, Serializer};

#[derive(Clone, Debug, PartialEq)]
pub enum Vote {
    Null,
    Unknown,
    Option(usize),
}

#[derive(Clone, Debug, PartialEq)]
pub enum VoteStatus {
    NotVoted,
    Voted,
}

impl<'de> Deserialize<'de> for Vote {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Vote::from(s))
    }
}

impl Serialize for Vote {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s: String = self.clone().into();
        serializer.serialize_str(&s)
    }
}

impl<'de> Deserialize<'de> for VoteStatus {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(VoteStatus::from(s))
    }
}

impl Serialize for VoteStatus {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s: String = self.clone().into();
        serializer.serialize_str(&s)
    }
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

impl From<String> for Vote {
    fn from(value: String) -> Self {
        Vote::from(value.as_str())
    }
}

impl From<usize> for Vote {
    fn from(value: usize) -> Self {
        Vote::new(value)
    }
}

impl Into<String> for Vote {
    fn into(self) -> String {
        self.to_string()
    }
}

impl From<&str> for VoteStatus {
    fn from(value: &str) -> Self {
        match value {
            "not voted" => VoteStatus::NotVoted,
            _ => VoteStatus::Voted,
        }
    }
}

impl From<String> for VoteStatus {
    fn from(value: String) -> Self {
        VoteStatus::from(value.as_str())
    }
}

impl Into<String> for VoteStatus {
    fn into(self) -> String {
        self.to_string()
    }
}

impl std::fmt::Display for Vote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Vote::Null => VoteStatus::NotVoted.fmt(f),
            Vote::Unknown => write!(f, "?"),
            Vote::Option(value) => write!(f, "{}", value),
        }
    }
}

impl std::fmt::Display for VoteStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VoteStatus::NotVoted => write!(f, "not voted"),
            VoteStatus::Voted => write!(f, "voted"),
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

    pub fn status(&self) -> VoteStatus {
        match self {
            Vote::Null => VoteStatus::NotVoted,
            _ => VoteStatus::Voted,
        }
    }

    pub fn is_valid_vote(&self) -> bool {
        *self != Vote::Null
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

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
        assert_eq!(Vote::Null.status().to_string(), "not voted");
        assert_eq!(Vote::Unknown.status().to_string(), "voted");
        assert_eq!(Vote::Option(1).status().to_string(), "voted");
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

    #[test]
    fn test_from_json() {
        let vote = Vote::Option(1);
        let json = json!("1");
        let vote_deserialized: Vote = serde_json::from_value(json).unwrap();
        assert_eq!(vote, vote_deserialized);

        let vote = Vote::Unknown;
        let json = json!("?");
        let vote_deserialized: Vote = serde_json::from_value(json).unwrap();
        assert_eq!(vote, vote_deserialized);

        let vote = Vote::Null;
        let json = json!("0");
        let vote_deserialized: Vote = serde_json::from_value(json).unwrap();
        assert_eq!(vote, vote_deserialized);

        let vote = Vote::new(1);
        let json = json!(vote.status());
        let vote_deserialized: VoteStatus = serde_json::from_value(json).unwrap();
        assert_eq!(vote.status(), vote_deserialized);
    }
}
