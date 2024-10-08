use std::{alloc::{alloc, handle_alloc_error, Layout}, ptr::slice_from_raw_parts_mut};

#[derive(Clone, Copy)]
struct Cursor {
	start: usize,
	end: usize,
}

pub struct MultiCursorBuffer {
	buffer: Box<[u8]>,
	cursors: Vec<Cursor>,
}

impl MultiCursorBuffer {
	pub fn new(size: usize) -> Self {
		let buffer = unsafe {
			let layout = Layout::array::<u8>(size).unwrap();
			let ptr = alloc(layout);
			if ptr.is_null() {
				handle_alloc_error(layout);
			}
			Box::from_raw(slice_from_raw_parts_mut(ptr, size))
		};
		Self { buffer, cursors: vec![] }
	}
	
	///Returns the index of the cursor
	pub fn add_cursor(&mut self, pos: usize) -> usize {
		assert!(pos <= self.buffer.len(), "cannot add cursor beyond length of buffer");
		if let Some(last_cursor) = self.cursors.last() {
			assert!(pos >= last_cursor.end, "cannot add cursor before end of last cursor");
		}
		let index = self.cursors.len();
		self.cursors.push(Cursor { start: pos, end: pos });
		index
	}
	
	///Returns the offset of the beginning of passed bytes
	pub fn write(&mut self, cursor_index: usize, bytes: &[u8]) -> usize {
		let upper_bound = self
			.cursors
			.get(cursor_index + 1)
			.map(|next| next.start)
			.unwrap_or(self.buffer.len());
		let cursor = &mut self.cursors[cursor_index];
		let new_end = cursor.end + bytes.len();
		assert!(new_end <= upper_bound, "cannot write beyond cursor bound");
		self.buffer[cursor.end..new_end].copy_from_slice(bytes);
		let offset = cursor.end;
		cursor.end = new_end;
		offset
	}
	
	///Zeroes gaps between written spaces and returns the underlying buffer
	pub fn into_buffer(mut self) -> Box<[u8]> {
		let mut written = 0;
		for cursor in &self.cursors {
			self.buffer[written..cursor.start].fill(0);
			written = cursor.end;
		}
		self.buffer[written..].fill(0);
		self.buffer
	}
}
