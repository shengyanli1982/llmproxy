use serde::{self, Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct SerializableArcString(pub Arc<String>);

impl Deref for SerializableArcString {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl fmt::Display for SerializableArcString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for SerializableArcString {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SerializableArcString {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Ok(SerializableArcString(Arc::new(String::deserialize(
            deserializer,
        )?)))
    }
}

impl From<String> for SerializableArcString {
    fn from(s: String) -> Self {
        Self(Arc::new(s))
    }
}
