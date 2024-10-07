pub struct VecTail<'a, T> {
	vec: &'a mut Vec<T>,
	start: usize,
}

impl<'a, T> VecTail<'a, T> {
	pub fn new(vec: &'a mut Vec<T>) -> Self {
		Self { vec, start: 0 }
	}
	
	pub fn split(&mut self, len: usize) -> &'a [T] {
		assert!(self.start + len <= self.vec.len());
		let head = unsafe { std::slice::from_raw_parts(self.vec.as_ptr().add(self.start), len) };
		self.start += len;
		head
	}
	
	pub fn split_one(&mut self) -> &'a T {
		&self.split(1)[0]
	}
	
	pub fn push(&mut self, item: T) {
		assert!(self.vec.len() < self.vec.capacity());
		self.vec.push(item);
	}
}
