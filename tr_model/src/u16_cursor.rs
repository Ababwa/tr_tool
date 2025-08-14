use std::{mem::{align_of, size_of}, ptr, slice};

pub struct U16Cursor<'a> {
	buf: &'a [u16],
}

impl<'a> U16Cursor<'a> {
	pub fn new(buf: &'a [u16]) -> Self {
		Self {
			buf,
		}
	}
	
	pub fn next(&mut self) -> u16 {
		let val = self.buf[0];
		self.buf = &self.buf[1..];
		val
	}
	
	pub unsafe fn read<T>(&mut self) -> T {
		const {
			assert!(size_of::<T>() % 2 == 0, "Read size must be a multiple of 2.");
		}
		let new_buf = &self.buf[size_of::<T>() / 2..];//do first to ensure buffer is long enough
		let val = ptr::read_unaligned(self.buf.as_ptr().cast());
		self.buf = new_buf;
		val
	}
	
	pub unsafe fn slice<T>(&mut self, len: usize) -> &'a [T] {
		const {
			assert!(align_of::<T>() <= 2, "Align must be 2 or less.");
		}
		assert!((size_of::<T>() * len) % 2 == 0, "Slice size must be a multiple of 2.");
		let new_buf = &self.buf[(size_of::<T>() * len) / 2..];
		let slice = slice::from_raw_parts(self.buf.as_ptr().cast(), len);
		self.buf = new_buf;
		slice
	}
	
	pub unsafe fn u16_len_slice<T>(&mut self) -> &'a [T] {
		let len = self.next() as usize;
		self.slice(len)
	}
}
