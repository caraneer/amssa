use crate::phonemes::{Phonehash, PhonehashRepr};

// SAFETY: Phonehash is #[repr(transparent)], so if T safely implements these traits, then a wrapper should too.
unsafe impl<T: PhonehashRepr + bytemuck::Zeroable> bytemuck::Zeroable for Phonehash<T> {}
// SAFETY: See above
unsafe impl<T: PhonehashRepr + bytemuck::Pod> bytemuck::Pod for Phonehash<T> {}
