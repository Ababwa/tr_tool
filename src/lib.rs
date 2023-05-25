use bevy::{prelude::Vec3, math::vec3};

pub mod save_image;

pub trait Rotatable {
	fn rotate_xy(&self) -> Self;
	fn rotate_yx(&self) -> Self;
	fn rotate_xz(&self) -> Self;
	fn rotate_zx(&self) -> Self;
	fn rotate_yz(&self) -> Self;
	fn rotate_zy(&self) -> Self;
}

impl Rotatable for Vec3 {
	fn rotate_xy(&self) -> Self { vec3(-self.y, self.x, self.z) }
	fn rotate_yx(&self) -> Self { vec3(self.y, -self.x, self.z) }
	fn rotate_xz(&self) -> Self { vec3(-self.z, self.y, self.x) }
	fn rotate_zx(&self) -> Self { vec3(self.z, self.y, -self.x) }
	fn rotate_zy(&self) -> Self { vec3(self.x, self.z, -self.y) }
	fn rotate_yz(&self) -> Self { vec3(self.x, -self.z, self.y) }
}
