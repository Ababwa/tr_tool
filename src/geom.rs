use std::ops::{Add, Sub};
use glam_traits::{GBVec, GVec};

/// Some volume defined by a minimum and maximum
#[derive(Clone, Copy)]
pub struct MinMax<T> {
	pub min: T,
	pub max: T,
}

impl<T: Copy> MinMax<T> {
	pub fn new(a: T) -> Self {
		Self { min: a, max: a }
	}
}

pub trait VecMinMax<V> {
	fn update(&mut self, v: V);
	fn contains(&self, other: &Self) -> bool;
	fn intersects(&self, other: &Self) -> bool;
	fn from_iter<I: IntoIterator<Item = V>>(iter: I) -> Option<Self> where Self: Sized;
}

impl<V: GVec> VecMinMax<V> for MinMax<V> {
	fn update(&mut self, v: V) {
		self.min = self.min.min(v);
		self.max = self.max.max(v);
	}
	
	fn contains(&self, other: &Self) -> bool {
		self.min.cmple(other.min).all() && self.max.cmpge(other.max).all()
	}
	
	fn intersects(&self, other: &Self) -> bool {
		self.min.cmplt(other.max).all() && self.max.cmpgt(other.min).all()
	}
	
	fn from_iter<I: IntoIterator<Item = V>>(iter: I) -> Option<Self> {
		let mut iter = iter.into_iter();
		let mut minmax = Self::new(iter.next()?);
		while let Some(v) = iter.next() {
			minmax.update(v);
		}
		Some(minmax)
	}
}

pub trait ScalarMinMax<S> {
	fn update(&mut self, s: S);
	fn contains(&self, other: &Self) -> bool;
	fn intersects(&self, other: &Self) -> bool;
	fn from_iter<I: IntoIterator<Item = S>>(iter: I) -> Option<Self> where Self: Sized;
}

impl<S: Copy + Ord> ScalarMinMax<S> for MinMax<S> {
	fn update(&mut self, s: S) {
		self.min = self.min.min(s);
		self.max = self.max.max(s);
	}
	
	fn contains(&self, other: &Self) -> bool {
		self.min <= other.min && self.max >= other.max
	}
	
	fn intersects(&self, other: &Self) -> bool {
		self.min < other.max && self.max > other.min
	}
	
	fn from_iter<I: IntoIterator<Item = S>>(iter: I) -> Option<Self> where Self: Sized {
		let mut iter = iter.into_iter();
		let mut minmax = Self::new(iter.next()?);
		while let Some(v) = iter.next() {
			minmax.update(v);
		}
		Some(minmax)
	}
}

impl<T: Copy + Add<Output = T>> From<PosSize<T>> for MinMax<T> {
	fn from(PosSize { pos, size }: PosSize<T>) -> Self {
		Self { min: pos, max: pos + size }
	}
}

/// Some volume defined by a position and a size
#[derive(Clone, Copy)]
pub struct PosSize<T> {
	pub pos: T,
	pub size: T,
}

impl<T: Copy + Sub<Output = T>> From<MinMax<T>> for PosSize<T> {
	fn from(MinMax { min, max }: MinMax<T>) -> Self {
		Self { pos: min, size: max - min }
	}
}
