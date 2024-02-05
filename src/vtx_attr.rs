use bevy::render::mesh::VertexAttributeValues;
use glam_traits::glam::{Vec2, Vec3};

pub struct VtxAttr<T>(pub T);

impl From<VtxAttr<Vec<Vec2>>> for VertexAttributeValues {
	fn from(VtxAttr(v): VtxAttr<Vec<Vec2>>) -> Self {
		VertexAttributeValues::Float32x2(v.into_iter().map(|v| v.into()).collect())
	}
}

impl From<VtxAttr<Vec<Vec3>>> for VertexAttributeValues {
	fn from(VtxAttr(v): VtxAttr<Vec<Vec3>>) -> Self {
		VertexAttributeValues::Float32x3(v.into_iter().map(|v| v.into()).collect())
	}
}
