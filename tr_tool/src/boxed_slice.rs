/*!
Boxed slice builders. Avoids the possible reallocation of `Vec::into_boxed_slice` as `Vec` doesn't guarantee
exact allocation size.
*/

use std::{mem::{ManuallyDrop, MaybeUninit}, ptr};

// pub fn new_cloned<T: Clone>(value: T, len: usize) -> Box<[T]> {
// 	let mut slice = Box::new_uninit_slice(len);
// 	if let Some((last, first)) = slice.split_last_mut() {
// 		for slot in first {
// 			*slot = MaybeUninit::new(value.clone());
// 		}
// 		*last = MaybeUninit::new(value);
// 	}
// 	//Safety: Fully initialized.
// 	unsafe {
// 		slice.assume_init()
// 	}
// }

pub fn new_copied<T: Copy>(value: T, len: usize) -> Box<[T]> {
	let mut slice = Box::new_uninit_slice(len);
	for slot in &mut slice {
		*slot = MaybeUninit::new(value);
	}
	//Safety: Fully initialized.
	unsafe {
		slice.assume_init()
	}
}

/// Boxed slice filler.
pub struct Bsf<T> {
	slice: Box<[MaybeUninit<T>]>,
	filled: usize,
}

impl<T> Bsf<T> {
	pub fn new(len: usize) -> Self {
		Self {
			slice: Box::new_uninit_slice(len),
			filled: 0,
		}
	}
	
	pub const fn push(&mut self, value: T) {
		self.slice[self.filled] = MaybeUninit::new(value);
		self.filled += 1;
	}
	
	pub const fn filled(&self) -> usize {
		self.filled
	}
	
	// pub fn as_slice(&self) -> &[T] {
	// 	//Safety: Slice initialized part.
	// 	unsafe {
	// 		slice::from_raw_parts(self.slice.as_ptr().cast(), self.filled)
	// 	}
	// }
	
	pub fn into_boxed_slice(self) -> Box<[T]> {
		assert!(self.filled == self.slice.len());
		let this = ManuallyDrop::new(self);
		/*
		Safety: `this` won't free `slice` when it goes out of scope. Copied slice has normal drop
		behavior. Fully initialzied after assert.
		*/
		unsafe {
			let slice = ptr::read(&this.slice);
			slice.assume_init()
		}
	}
}

impl<T: Copy> Bsf<T> {
	pub const fn extend_copy(&mut self, other: &[T]) {
		assert!(self.filled + other.len() <= self.slice.len());
		//Safety: Uninitialized part of `slice` cannot be in `other`. Bounds are safe after assert.
		unsafe {
			let dest = self.slice.as_mut_ptr().add(self.filled);
			ptr::copy_nonoverlapping(other.as_ptr().cast(), dest, other.len());
		}
		self.filled += other.len();
	}
}

impl<T> Drop for Bsf<T> {
	fn drop(&mut self) {
		let ptr = self.slice.as_mut_ptr();
		let init_slice = ptr::slice_from_raw_parts_mut(ptr.cast::<T>(), self.filled);
		//Safety: Drop initialized part.
		unsafe {
			ptr::drop_in_place(init_slice);
		}
	}
}
