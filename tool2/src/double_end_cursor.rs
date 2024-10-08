use std::{alloc::{alloc, handle_alloc_error, Layout}, ptr::slice_from_raw_parts_mut};

pub struct DoubleEndBuffer {
	buffer: Box<[u8]>,
	start_pos: usize,
	end_pos: usize,
}

impl DoubleEndBuffer {
	pub fn new(size: usize) -> Self {
		let buffer = unsafe {
			let layout = Layout::array::<u8>(size).unwrap();
			let ptr = alloc(layout);
			if ptr.is_null() {
				handle_alloc_error(layout);
			}
			Box::from_raw(slice_from_raw_parts_mut(ptr, size))
		};
		DoubleEndBuffer { buffer, start_pos: 0, end_pos: size }
	}
	
	pub fn start_pos(&self) -> usize {
		self.start_pos
	}
	
	pub fn write_start(&mut self, bytes: &[u8]) {
		if self.start_pos + bytes.len() > self.end_pos {
			panic!("write_start overlaps end");
		}
		self.buffer[self.start_pos..][..bytes.len()].copy_from_slice(bytes);
		self.start_pos += bytes.len();
	}
	
	pub fn write_end(&mut self, bytes: &[u8]) {
		if self.end_pos - bytes.len() < self.start_pos {
			panic!("write_end overlaps start");
		}
		self.buffer[self.end_pos - bytes.len()..][..bytes.len()].copy_from_slice(bytes);
		self.end_pos -= bytes.len();
	}
	
	pub fn take_buffer(mut self) -> Box<[u8]> {
		self.buffer[self.start_pos..self.end_pos].fill(0);
		self.buffer
	}
}
