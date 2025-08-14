use crate::boxed_slice::Bsf;

pub trait PushGet<T> {
	// fn push_get(&mut self, value: T) -> &T;
	// fn push_get_mut(&mut self, value: T) -> &mut T;
	fn push_get_index(&mut self, value: T) -> usize;
}

impl<T> PushGet<T> for Vec<T> {
	fn push_get_index(&mut self, value: T) -> usize {
		let index = self.len();
		self.push(value);
		index
	}
}

impl<T> PushGet<T> for Bsf<T> {
	fn push_get_index(&mut self, value: T) -> usize {
		let index = self.filled();
		self.push(value);
		index
	}
}
