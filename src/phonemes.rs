use core::fmt::{self, Write as _};
use std::{convert::Infallible, str::FromStr};

use deunicode::AsciiChars;
use itertools::peek_nth;

pub trait PhonehashRepr: Default + Copy + Clone + Eq + Ord {
	fn stray_bits() -> u32;
	fn max_phonemes() -> u32;
	fn is_finalized(&self) -> bool;
	fn finalize(&mut self, remaining: u32);
	fn append(&mut self, elem: PhonehashElem) -> bool;
	fn phoneme_at(&self, index: u32) -> Option<PhonehashElem>;
	fn starts_with(&self, other: Self) -> bool;
}

macro_rules! impl_phonehash_repr_uint {
	($ty:ty) => {
		impl PhonehashRepr for $ty {
			#[inline]
			fn stray_bits() -> u32 {
				Self::BITS % 3
			}
			#[inline]
			fn max_phonemes() -> u32 {
				Self::BITS / 3
			}
			#[inline]
			fn is_finalized(&self) -> bool {
				// top 3 bits set => mask
				const FINAL_MASK: $ty = !0 << (<$ty>::BITS - 3);
				(*self & FINAL_MASK) != 0
			}

			fn finalize(&mut self, remaining: u32) {
				*self <<= 3 * remaining;
				*self <<= Self::stray_bits();
			}
			fn append(&mut self, elem: PhonehashElem) -> bool {
				let elem = (elem as u8) as Self;
				// ignore spaces and don't have consecutive duplicate elements. We're also going to ignore vowels as that makes
				// things compress better
				if elem <= 1 || *self & 7 == elem || self.is_finalized() {
					return false;
				}
				*self <<= 3;
				*self |= elem as Self;
				return true;
			}
			fn phoneme_at(&self, index: u32) -> Option<PhonehashElem> {
				if index >= Self::max_phonemes() {
					return None;
				}
				let result = self
					.overflowing_shr(Self::stray_bits() + 3 * (Self::max_phonemes() - index - 1))
					.0;
				match result & 7 {
					0 => Some(PhonehashElem::Space),
					1 => Some(PhonehashElem::A),
					2 => Some(PhonehashElem::B),
					3 => Some(PhonehashElem::F),
					4 => Some(PhonehashElem::S),
					5 => Some(PhonehashElem::G),
					6 => Some(PhonehashElem::M),
					7 => Some(PhonehashElem::W),
					_ => None,
				}
			}
			fn starts_with(&self, other: Self) -> bool {
				let mut max_phoneme_bits: Self = !Self::default();
				while (other & max_phoneme_bits) != 0 {
					max_phoneme_bits = max_phoneme_bits.overflowing_shr(3).0;
				}
				*self >= other && *self <= (other | max_phoneme_bits)
			}
		}
	};
}

// Implement for the unsigned primitives you care about:
impl_phonehash_repr_uint!(u8);
impl_phonehash_repr_uint!(u16);
impl_phonehash_repr_uint!(u32);
impl_phonehash_repr_uint!(u64);
impl_phonehash_repr_uint!(u128);
impl_phonehash_repr_uint!(usize);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Phonehash<T: PhonehashRepr>(pub(crate) T);
impl<T: PhonehashRepr> Phonehash<T> {
	/// Calculates the new phoneme hash of the string
	pub fn new(s: &str) -> Self {
		phonehash_elements(s).collect()
	}
	/// it's like `str::starts_with` but more fuzzy and based on the phoneme hash
	pub fn starts_with(&self, other: Self) -> bool {
		self.0.starts_with(other.0)
	}
}
impl<T: PhonehashRepr> FromStr for Phonehash<T> {
	type Err = Infallible;
	fn from_str(s: &str) -> Result<Self, Self::Err> {
		Ok(Self::new(s))
	}
}
impl<T: PhonehashRepr> FromIterator<PhonehashElem> for Phonehash<T> {
	fn from_iter<I: IntoIterator<Item = PhonehashElem>>(iter: I) -> Self {
		let mut repr: T = T::default();

		let mut remaining_max = T::max_phonemes();
		for item in iter {
			if repr.append(item) {
				remaining_max -= 1;
				if remaining_max == 0 {
					break;
				}
			}
		}
		repr.finalize(remaining_max);
		Self(repr)
	}
}
impl<T: PhonehashRepr> fmt::Display for Phonehash<T> {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		for i in 0..T::max_phonemes() {
			self.0
				.phoneme_at(i)
				.map(|pelem| pelem.fmt(f))
				.unwrap_or_else(|| f.write_char('?'))?;
		}

		Ok(())
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PhonehashElem {
	Space = 0,
	A = 1, // A, E, I, O, U, Y
	B = 2, // B, D, T, P
	F = 3, // F, V
	S = 4, // C, S, X, K, Q, Z
	G = 5, // G, J,
	M = 6, // M, N
	W = 7, // L, R, W
	       // H is always treated as silent
}
impl PhonehashElem {
	pub fn is_space(&self) -> bool {
		*self == Self::Space
	}
}
impl fmt::Display for PhonehashElem {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			PhonehashElem::Space => f.write_char('_'),
			PhonehashElem::A => f.write_char('A'),
			PhonehashElem::B => f.write_char('B'),
			PhonehashElem::F => f.write_char('F'),
			PhonehashElem::S => f.write_char('S'),
			PhonehashElem::G => f.write_char('G'),
			PhonehashElem::M => f.write_char('M'),
			PhonehashElem::W => f.write_char('W'),
		}
	}
}

pub fn phonehash_elements(s: &str) -> impl Iterator<Item = PhonehashElem> {
	// Normalize string to lower case ascii alphabet
	let mut ascii_alphas = peek_nth(
		s.ascii_chars()
			.flat_map(|s| s.unwrap_or_default().chars())
			.map(|c| (c as u8).to_ascii_lowercase())
			.flat_map(|c| -> Box<dyn Iterator<Item = u8>> {
				match c {
					b'0' => Box::new(b" zero ".iter().copied()),
					b'1' => Box::new(b" one ".iter().copied()),
					b'2' => Box::new(b" two ".iter().copied()),
					b'3' => Box::new(b" three ".iter().copied()),
					b'4' => Box::new(b" four ".iter().copied()),
					b'5' => Box::new(b" five ".iter().copied()),
					b'6' => Box::new(b" six ".iter().copied()),
					b'7' => Box::new(b" seven ".iter().copied()),
					b'8' => Box::new(b" eight ".iter().copied()),
					b'9' => Box::new(b" nine ".iter().copied()),
					b'$' => Box::new(b" dollar ".iter().copied()),
					b'%' => Box::new(b" percent ".iter().copied()),
					b'&' => Box::new(b" and ".iter().copied()),
					b'+' => Box::new(b" plus ".iter().copied()),
					// lowkey annoying that I can only erase the iter type by doing a dyn
					b'a'..=b'z' => Box::new(std::iter::once(c)),
					_ => Box::new(std::iter::once(b' ')),
				}
			}),
	);

	// operations that require 1 char lookahead
	let mut check_silent_first_letter = true;
	let ascii_alphas = std::iter::from_fn(move || -> Option<u8> {
		loop {
			match (
				std::mem::take(&mut check_silent_first_letter),
				ascii_alphas.next(),
				ascii_alphas.peek_nth(0).copied(),
				ascii_alphas.peek_nth(1).copied(),
			) {
				(_, None, _, _) => break None,
				// ph is pronouced as f
				(_, Some(b'p'), Some(b'h'), _) => {
					ascii_alphas.next(); // h
					break Some(b'f');
				},
				// remove consecutive spaces
				(_, Some(b' '), Some(b' '), _) => {
					continue;
				},
				// gh is silent, skip over them
				(_, Some(b'g'), Some(b'h'), _) => {
					ascii_alphas.next(); // h
					continue;
				},
				// skip over "k" in "knight"
				(true, Some(b'k'), Some(b'n'), _) => {
					break ascii_alphas.next(); // n
				},
				// A knight is approaching
				(_, Some(b' '), Some(b'k'), Some(b'n')) => {
					check_silent_first_letter = true;
					break Some(b' ');
				},
				(_, Some(c), _, _) => break Some(c),
			}
		}
	});

	// final part
	ascii_alphas.filter_map(|c| match c {
		b' ' => Some(PhonehashElem::Space),
		b'a' | b'e' | b'i' | b'o' | b'u' | b'y' => Some(PhonehashElem::A),
		b'b' | b'd' | b't' | b'p' => Some(PhonehashElem::B),
		b'f' | b'v' => Some(PhonehashElem::F),
		b'c' | b's' | b'x' | b'k' | b'q' | b'z' => Some(PhonehashElem::S),
		b'g' | b'j' => Some(PhonehashElem::G),
		b'm' | b'n' => Some(PhonehashElem::M),
		b'l' | b'r' | b'w' => Some(PhonehashElem::W),
		_ => None, // h
	})
}

// convenience traits

pub trait CanPhonehash {
	fn phonehash_elements(&self) -> impl Iterator<Item = PhonehashElem>;
	/// Calculates the phoneme hash of the string
	fn phonehash<T: PhonehashRepr>(&self) -> Phonehash<T> {
		self.phonehash_elements().collect()
	}
}
impl<T> CanPhonehash for T
where
	T: AsRef<str>,
{
	fn phonehash_elements(&self) -> impl Iterator<Item = PhonehashElem> {
		phonehash_elements(self.as_ref())
	}
}

#[cfg(test)]
mod test {
	use super::*;
	#[test]
	fn it_werks() {
		let s = "Phởenix Knight DAO++";
		let h: Phonehash<u64> = s.phonehash();
		// initial example
		assert_eq!(h.to_string(), "FMSMBWSBWS___________".to_string());

		// phonetic match
		assert_eq!("knight".phonehash::<u64>().to_string(), "MB___________________");
		assert_eq!("nite".phonehash::<u64>().to_string(), "MB___________________");
		assert_eq!("knight".phonehash::<u8>().to_string(), "MB");
		assert_eq!("nite".phonehash::<u8>().to_string(), "MB");

		// approximate
		assert_eq!("phoenix".phonehash::<u64>().to_string(), "FMS__________________");
		assert_eq!("foneks".phonehash::<u64>().to_string(), "FMS__________________");
		assert_eq!("fone6".phonehash::<u64>().to_string(), "FMS__________________");

		// substring match
		assert_eq!("knight rider".phonehash::<u64>().to_string(), "MBWBW________________");
		assert_eq!("knightrider".phonehash::<u64>().to_string(), "MBWBW________________");
		assert_eq!(
			"knight rheyedhurr".phonehash::<u64>().to_string(),
			"MBWBW________________"
		);
		assert!("knight rider".phonehash::<u64>().starts_with("nite".phonehash::<u64>()));
		assert!(
			!"knight"
				.phonehash::<u64>()
				.starts_with("nite rheyedhurr".phonehash::<u64>())
		);

		// vowel normalization
		assert_eq!("Shiba".phonehash::<u64>().to_string(), "SB___________________");
		assert_eq!("Sheba".phonehash::<u64>().to_string(), "SB___________________");
		assert_eq!(
			"aaaeeeeyyyyyeee lllaaaaammmaaaaaooo".phonehash::<u64>().to_string(),
			"WM___________________"
		);

		// spacing
		assert_eq!("co-op".phonehash::<u64>().to_string(), "SB___________________");
		assert_eq!("co   op".phonehash::<u64>().to_string(), "SB___________________");
	}
}
