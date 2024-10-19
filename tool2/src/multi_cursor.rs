use std::{alloc::{alloc, handle_alloc_error, Layout}, ops::Range, ptr::slice_from_raw_parts_mut};

pub struct Writer<'a> {
	buffer: &'a mut Box<[u8]>,
	range: &'a mut Range<usize>,
	upper_bound: usize,
}

impl<'a> Writer<'a> {
	/// Range of written bytes. `end` is the index of the next byte to be written.
	pub fn range(&self) -> Range<usize> {
		self.range.clone()
	}
	
	/// Slice of written bytes.
	pub fn slice(&self) -> &[u8] {
		&self.buffer[self.range()]
	}
	
	pub fn write(&mut self, bytes: &[u8]) {
		self.buffer[self.range.end..self.upper_bound][..bytes.len()].copy_from_slice(bytes);
		self.range.end += bytes.len();
	}
}

pub struct MultiCursorBuffer {
	buffer: Box<[u8]>,
	/// Ranges of written bytes.
	ranges: Vec<Range<usize>>,
}

impl MultiCursorBuffer {
	/// Returns the index of the cursor.
	pub fn add_cursor(&mut self, pos: usize) -> usize {
		assert!(pos <= self.buffer.len(), "cannot add cursor beyond end of buffer");
		if let Some(last_cursor) = self.ranges.last() {
			assert!(pos >= last_cursor.end, "cannot add cursor before end of last cursor");
		}
		let index = self.ranges.len();
		self.ranges.push(pos..pos);
		index
	}
	
	pub fn new(size: usize, cursor_positions: &[usize]) -> Self {
		let buffer = unsafe {
			let layout = Layout::array::<u8>(size).unwrap();
			let ptr = alloc(layout);
			if ptr.is_null() {
				handle_alloc_error(layout);
			}
			Box::from_raw(slice_from_raw_parts_mut(ptr, size))
		};
		let mut mc = Self { buffer, ranges: Vec::with_capacity(cursor_positions.len()) };
		for &cursor_pos in cursor_positions {
			mc.add_cursor(cursor_pos);
		}
		mc
	}
	
	pub fn get_writer(&mut self, cursor_index: usize) -> Writer {
		let upper_bound = self
			.ranges
			.get(cursor_index + 1)
			.map(|next| next.start)
			.unwrap_or(self.buffer.len());
		let buffer = &mut self.buffer;
		let range = &mut self.ranges[cursor_index];
		Writer { buffer, range, upper_bound }
	}
	
	/// Zeroes the gaps between written sections and returns the underlying buffer.
	pub fn into_buffer(mut self) -> Box<[u8]> {
		let mut written = 0;
		for cursor in &self.ranges {
			self.buffer[written..cursor.start].fill(0);
			written = cursor.end;
		}
		self.buffer[written..].fill(0);
		self.buffer
	}
}
