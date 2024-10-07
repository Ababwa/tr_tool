use std::{mem::{align_of, size_of}, ptr::read_unaligned, slice::from_raw_parts};

#[derive(Clone, Copy)]
pub struct U16Cursor<'a> {
	buf: &'a [u16],
}

impl<'a> U16Cursor<'a> {
	pub fn new(buf: &'a [u16]) -> Self {
		Self { buf }
	}
	
	pub fn next(&mut self) -> u16 {
		let val = self.buf[0];
		self.buf = &self.buf[1..];
		val
	}
	
	pub unsafe fn read<T>(&mut self) -> T {
		assert!(size_of::<T>() % 2 == 0, "read size must be a multiple of 2");
		let new_buf = &self.buf[size_of::<T>() / 2..];//do first to ensure buffer is long enough
		let val = read_unaligned(self.buf.as_ptr() as *const T);
		self.buf = new_buf;
		val
	}
	
	pub unsafe fn slice<T>(&mut self, len: usize) -> &'a [T] {
		assert!(align_of::<T>() <= 2, "slice align must be 2 or less");
		assert!((size_of::<T>() * len) % 2 == 0, "slice size must be a multiple of 2");
		let new_buf = &self.buf[(size_of::<T>() * len) / 2..];
		let slice = from_raw_parts(self.buf.as_ptr() as *const T, len);
		self.buf = new_buf;
		slice
	}
	
	pub unsafe fn u16_len_slice<T>(&mut self) -> &'a [T] {
		let len = self.next() as usize;
		self.slice(len)
	}
}
