use std::cmp::Ordering;

use strsim::damerau_levenshtein;

use crate::phonemes::{Phonehash, PhonehashRepr};

pub trait SearchableItem: Clone {
	type Repr: PhonehashRepr;

	fn as_phoneme(&self) -> Phonehash<Self::Repr>;
	fn as_str(&self) -> &str;
}

pub trait SearchableList {
	type ListItem: SearchableItem;

	/// The length of the list
	fn len(&self) -> usize;

	/// This should be a wrapper for a get_unchecked function, when called by `phonehash_search`, `index` will always
	/// be less than `self.len()`
	unsafe fn item_at_unchecked(&self, index: usize) -> &Self::ListItem;

	/// Search this list for the following query. This function assumes that the list is sorted by the phoneme hash
	fn phonehash_search(&self, query: &Self::ListItem, max_items: usize) -> Vec<Self::ListItem> {
		// based on the rust stdlib's "binary_search_by" algorithm
		let mut size = self.len();
		if size == 0 {
			return Vec::new();
		}
		let mut base = 0usize;
		// left-biased binary search
		while size > 0 {
			let half = size / 2;
			let mid = base + half;

			let cmp = unsafe { self.item_at_unchecked(mid).as_phoneme() }.cmp(&query.as_phoneme());
			if cmp == Ordering::Less {
				base = mid + 1;
				size -= half + 1;
			} else {
				// Even if equal, we bias left by not discarding left half
				size = half;
			}
		}

		let mut result_with_dist = Vec::new();
		size = self.len();
		let max_item_index = (max_items + base).min(size);
		// fill results with all the equal phonemes, as otherwise we risk discarding the actual desired result
		while base < size {
			// SAFETY: base < size is explicitly checked
			let base_value = unsafe { self.item_at_unchecked(base).clone() };
			if base_value.as_phoneme() != query.as_phoneme() {
				break;
			}
			result_with_dist.push((damerau_levenshtein(base_value.as_str(), query.as_str()), base_value));
			base += 1;
		}
		while base < max_item_index {
			// SAFETY: max_item_index <= size, base < max_item_index
			let base_value = unsafe { self.item_at_unchecked(base).clone() };
			if !base_value.as_phoneme().starts_with(query.as_phoneme()) {
				break;
			}
			result_with_dist.push((damerau_levenshtein(base_value.as_str(), query.as_str()), base_value));
			base += 1;
		}
		result_with_dist.sort_by(|a, b| a.0.cmp(&b.0));
		result_with_dist.into_iter().take(max_items).map(|v| v.1).collect()
	}
}
impl<T: SearchableItem> SearchableList for [T] {
	type ListItem = T;
	fn len(&self) -> usize {
		<[T]>::len(self)
	}
	unsafe fn item_at_unchecked(&self, index: usize) -> &Self::ListItem {
		unsafe { self.get_unchecked(index) }
	}
}
impl<T: SearchableItem> SearchableList for &[T] {
	type ListItem = T;
	fn len(&self) -> usize {
		<[T]>::len(self)
	}
	unsafe fn item_at_unchecked(&self, index: usize) -> &Self::ListItem {
		unsafe { self.get_unchecked(index) }
	}
}
impl<T: SearchableItem> SearchableList for Vec<T> {
	type ListItem = T;
	fn len(&self) -> usize {
		<[T]>::len(self)
	}
	unsafe fn item_at_unchecked(&self, index: usize) -> &Self::ListItem {
		unsafe { self.get_unchecked(index) }
	}
}

#[cfg(test)]
mod test {
	use crate::phonemes::CanPhonehash;

	use super::*;
	#[test]
	fn it_werks() {
		#[derive(Debug, Clone, PartialEq, Eq)]
		struct TestObject {
			str: &'static str,
			phoneme: Phonehash<u64>,
		}
		impl TestObject {
			pub fn new(str: &'static str) -> Self {
				Self {
					str,
					phoneme: str.phonehash(),
				}
			}
		}
		impl SearchableItem for TestObject {
			type Repr = u64;
			fn as_phoneme(&self) -> Phonehash<Self::Repr> {
				self.phoneme
			}
			fn as_str(&self) -> &str {
				self.str
			}
		}

		let mut stuff = vec![
			TestObject::new("aaaa"),
			TestObject::new("knight rider"),
			TestObject::new("nite writer"),
			TestObject::new("neight rheyeder"),
			TestObject::new("the amazing digital circus"),
		];
		stuff.sort_by_key(|v| v.phoneme);

		// Only matching phonemes are returned, even if more are requested.
		// They should also be sorted by distance
		assert_eq!(
			stuff.phonehash_search(&TestObject::new("knight"), 5),
			vec![
				TestObject::new("knight rider"),
				TestObject::new("nite writer"),
				TestObject::new("neight rheyeder"),
			]
		);
		assert_eq!(
			stuff.phonehash_search(&TestObject::new("knight writer"), 5),
			vec![
				TestObject::new("knight rider"),
				TestObject::new("nite writer"),
				TestObject::new("neight rheyeder"),
			]
		);

		// If less than the ones available are requested, all matching phonemes are used but the result is still capped
		assert_eq!(
			stuff.phonehash_search(&TestObject::new("knight rider"), 2),
			vec![TestObject::new("knight rider"), TestObject::new("nite writer"),]
		);
		assert_eq!(
			stuff.phonehash_search(&TestObject::new("nite writer"), 2),
			vec![TestObject::new("nite writer"), TestObject::new("knight rider"),]
		);
		assert_eq!(
			stuff.phonehash_search(&TestObject::new("neight rheyeder"), 2),
			vec![TestObject::new("neight rheyeder"), TestObject::new("knight rider"),]
		);

		// but partial matches are a different story
		assert_eq!(
			stuff.phonehash_search(&TestObject::new("knight"), 2),
			vec![TestObject::new("knight rider"), TestObject::new("nite writer")]
		);
		assert_eq!(
			stuff.phonehash_search(&TestObject::new("nite"), 2),
			vec![TestObject::new("nite writer"), TestObject::new("knight rider")]
		);
		assert_eq!(
			stuff.phonehash_search(&TestObject::new("neight"), 2),
			vec![TestObject::new("knight rider"), TestObject::new("nite writer")]
		);
	}
}
