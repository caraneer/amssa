use serde::{Deserialize, Serialize};

use crate::phonemes::{Phonehash, PhonehashRepr};

impl<'de, T: Deserialize<'de> + PhonehashRepr> Deserialize<'de> for Phonehash<T> {
	fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
	where
		D: serde::Deserializer<'de>,
	{
		let repr: T = T::deserialize(deserializer)?;
		if repr != T::default() && !repr.is_finalized() {
			return Err(serde::de::Error::custom("Phonehash is neither blank nor finalized"));
		}
		Ok(Self(repr))
	}
}
impl<T: Serialize + PhonehashRepr> Serialize for Phonehash<T> {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: serde::Serializer,
	{
		self.0.serialize(serializer)
	}
}
