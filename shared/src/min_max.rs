use glam_traits::{GBVec, GVec};

/// Some volume defined by a minimum and maximum.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct MinMax<T> {
	pub min: T,
	pub max: T,
}

impl<T> MinMax<T> where T: Clone {
	pub fn new(a: T) -> Self {
		Self { min: a.clone(), max: a }
	}
}

pub trait VecMinMax<T> {
	fn update(&mut self, v: T);
	fn contains(&self, other: &Self) -> bool;
	fn intersects(&self, other: &Self) -> bool;
}

impl<T> VecMinMax<T> for MinMax<T> where T: GVec {
	fn update(&mut self, a: T) {
		self.min = self.min.min(a);
		self.max = self.max.max(a);
	}
	
	fn contains(&self, other: &Self) -> bool {
		self.min.cmple(other.min).all() && self.max.cmpge(other.max).all()
	}
	
	fn intersects(&self, other: &Self) -> bool {
		self.min.cmplt(other.max).all() && self.max.cmpgt(other.min).all()
	}
}

pub trait VecMinMaxFromIterator: Iterator {
	fn min_max(self) -> Option<MinMax<Self::Item>>;
}

impl<T> VecMinMaxFromIterator for T where T: Iterator, T::Item: GVec {
	fn min_max(mut self) -> Option<MinMax<Self::Item>> {
		let mut min_max = MinMax::new(self.next()?);
		while let Some(a) = self.next() {
			min_max.update(a);
		}
		Some(min_max)
	}
}

pub trait ScalarMinMax<T> {
	fn update(&mut self, a: T);
	fn contains(&self, other: &Self) -> bool;
	fn intersects(&self, other: &Self) -> bool;
}

impl<T> ScalarMinMax<T> for MinMax<T> where T: Copy + Ord {
	fn update(&mut self, a: T) {
		self.min = self.min.min(a);
		self.max = self.max.max(a);
	}
	
	fn contains(&self, other: &Self) -> bool {
		self.min <= other.min && self.max >= other.max
	}
	
	fn intersects(&self, other: &Self) -> bool {
		self.min < other.max && self.max > other.min
	}
}

pub trait ScalarMinMaxFromIterator: Iterator {
	fn min_max(self) -> Option<MinMax<Self::Item>>;
}

impl<T> ScalarMinMaxFromIterator for T where T: Iterator, T::Item: Copy + Ord {
	fn min_max(mut self) -> Option<MinMax<Self::Item>> {
		let mut min_max = MinMax::new(self.next()?);
		while let Some(a) = self.next() {
			min_max.update(a);
		}
		Some(min_max)
	}
}
