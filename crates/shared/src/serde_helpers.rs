//! Ajudantes de serialização. `Vec2` (glam) só implementa `Serialize` com a feature
//! serde do glam ligada; para não depender disso, serializamos como par `(f32, f32)`
//! via `#[serde(with = "crate::serde_helpers::vec2")]`.

/// serde para `bevy::math::Vec2` como `(f32, f32)`.
pub mod vec2 {
    use bevy::math::Vec2;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(v: &Vec2, serializer: S) -> Result<S::Ok, S::Error> {
        (v.x, v.y).serialize(serializer)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec2, D::Error> {
        let (x, y) = <(f32, f32)>::deserialize(deserializer)?;
        Ok(Vec2::new(x, y))
    }
}
