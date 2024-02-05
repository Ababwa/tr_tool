use glam_traits::{GBVec, GVec};

#[derive(Clone, Copy)]
pub struct MinMax<V> {
	pub min: V,
	pub max: V,
}

impl<V: GVec> MinMax<V> {
	pub fn new(v: V) -> Self {
		Self { min: v, max: v }
	}
	
	pub fn update(&mut self, v: V) {
		self.min = self.min.min(v);
		self.max = self.max.max(v);
	}
	
	pub fn contains(&self, other: &Self) -> bool {
		self.min.cmple(other.min).all() && self.max.cmpge(other.max).all()
	}
	
	pub fn intersects(&self, other: &Self) -> bool {
		self.min.cmplt(other.max).all() && self.max.cmpgt(other.min).all()
	}
}

impl<V: GVec> From<PosSize<V>> for MinMax<V> {
	fn from(PosSize { pos, size }: PosSize<V>) -> Self {
		Self { min: pos, max: pos + size }
	}
}

#[derive(Clone, Copy)]
pub struct PosSize<V> {
	pub pos: V,
	pub size: V,
}

impl<V: GVec> From<MinMax<V>> for PosSize<V> {
	fn from(MinMax { min, max }: MinMax<V>) -> Self {
		Self { pos: min, size: max - min }
	}
}
