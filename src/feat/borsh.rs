use borsh::{BorshDeserialize, BorshSerialize};

use crate::phonemes::{Phonehash, PhonehashRepr};

impl<T: BorshDeserialize + PhonehashRepr> BorshDeserialize for Phonehash<T> {
	fn deserialize_reader<R: borsh::io::Read>(reader: &mut R) -> borsh::io::Result<Self> {
		let repr: T = T::deserialize_reader(reader)?;
		if repr != T::default() && !repr.is_finalized() {
			return Err(borsh::io::Error::other("Phonehash is neither blank nor finalized"));
		}
		Ok(Self(repr))
	}
}
impl<T: BorshSerialize + PhonehashRepr> BorshSerialize for Phonehash<T> {
	fn serialize<W: borsh::io::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
		self.0.serialize(writer)
	}
}
