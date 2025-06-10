// 自定义 Arc<String> 的序列化和反序列化
pub mod arc_string {
    use serde::{self, Deserialize, Deserializer, Serializer};
    use std::sync::Arc;

    pub fn serialize<S>(value: &Arc<String>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(value.as_ref())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Arc<String>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Ok(Arc::new(s))
    }
}
