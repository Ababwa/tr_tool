use glam_traits::glam::{
	I16Vec2,
	IVec2,
	I64Vec2,
	Vec2,
	DVec2,
	I16Vec3,
	IVec3,
	I64Vec3,
	Vec3,
	Vec3A,
	DVec3,
};

pub trait OrthoRot2 {
	fn rotate_xy(self) -> Self;
	fn rotate_yx(self) -> Self;
}

macro_rules! impl_orthorot2 {
	($type:ty) => {
		impl OrthoRot2 for $type {
			fn rotate_xy(self) -> Self { Self::new(-self.y, self.x) }
			fn rotate_yx(self) -> Self { Self::new(self.y, -self.x) }
		}
	};
}

impl_orthorot2!(I16Vec2);
impl_orthorot2!(IVec2);
impl_orthorot2!(I64Vec2);
impl_orthorot2!(Vec2);
impl_orthorot2!(DVec2);

pub trait OrthoRot3 {
	fn rotate_xy(self) -> Self;
	fn rotate_yx(self) -> Self;
	fn rotate_xz(self) -> Self;
	fn rotate_zx(self) -> Self;
	fn rotate_yz(self) -> Self;
	fn rotate_zy(self) -> Self;
}

macro_rules! impl_orthorot3 {
	($type:ty) => {
		impl OrthoRot3 for $type {
			fn rotate_xy(self) -> Self { Self::new(-self.y, self.x, self.z) }
			fn rotate_yx(self) -> Self { Self::new(self.y, -self.x, self.z) }
			fn rotate_xz(self) -> Self { Self::new(-self.z, self.y, self.x) }
			fn rotate_zx(self) -> Self { Self::new(self.z, self.y, -self.x) }
			fn rotate_zy(self) -> Self { Self::new(self.x, self.z, -self.y) }
			fn rotate_yz(self) -> Self { Self::new(self.x, -self.z, self.y) }
		}
	};
}

impl_orthorot3!(I16Vec3);
impl_orthorot3!(IVec3);
impl_orthorot3!(I64Vec3);
impl_orthorot3!(Vec3);
impl_orthorot3!(Vec3A);
impl_orthorot3!(DVec3);
