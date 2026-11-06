use std::fmt;

use crate::GossipError;

pub const DEFAULT_TOPIC: &str = "locap2p/events/v1";

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Topic(String);

impl Topic {
    pub fn new(value: impl Into<String>) -> Result<Self, GossipError> {
        let value = value.into();
        if value.is_empty() {
            return Err(GossipError::InvalidTopic);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for Topic {
    fn default() -> Self {
        Self(DEFAULT_TOPIC.to_owned())
    }
}

impl fmt::Display for Topic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}
