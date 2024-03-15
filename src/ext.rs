use std::future::Future;
use glam::Mat4;
use pollster::block_on;
use shared::reinterpret;
use crate::load::{SolidVertex, SpriteVertex, TexturedVertex};

pub trait Wait: Future {
	fn wait(self) -> Self::Output;
}

impl<T: Future> Wait for T {
	fn wait(self) -> Self::Output {
		block_on(self)
	}
}

pub trait AsBytes {
	fn as_bytes(&self) -> &[u8];
}

pub trait SafeAsBytes {}

impl<T: SafeAsBytes> AsBytes for T {
	fn as_bytes(&self) -> &[u8] {
		unsafe { reinterpret::ref_to_slice(self) }
	}
}

impl<T: SafeAsBytes> AsBytes for [T] {
	fn as_bytes(&self) -> &[u8] {
		unsafe { reinterpret::slice(self) }
	}
}

impl SafeAsBytes for Mat4 {}
impl SafeAsBytes for TexturedVertex {}
impl SafeAsBytes for SolidVertex {}
impl SafeAsBytes for SpriteVertex {}
