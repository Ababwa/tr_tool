use std::mem::MaybeUninit;
use shared::alloc;

pub struct FixedVec<T, const N: usize> {
	array: Box<[MaybeUninit<T>; N]>,
	len: usize,
}

impl<T, const N: usize> FixedVec<T, N> {
	pub fn new() -> Self {
		Self { array: alloc::array(), len: 0 }
	}
	
	pub fn push(&mut self, value: T) {
		self.array[self.len].write(value);
		self.len += 1;
	}
	
	pub fn len(&self) -> usize {
		self.len
	}
	
	pub fn into_inner(self) -> Box<[MaybeUninit<T>; N]> {
		self.array
	}
	
	pub fn into_vec(self) -> Vec<T> {
		//safe: 0..self.len is initialized
		unsafe { Vec::from_raw_parts(Box::into_raw(self.array).cast(), self.len, N) }
	}
}
