use std::ops::Neg;
use glam_traits::{SignedVec2, SignedVec3};

pub trait OrthoRot2 {
	fn rotate_xy(self) -> Self;
	fn rotate_yx(self) -> Self;
}

impl<V> OrthoRot2 for V where V: SignedVec2, V::Scalar: Neg<Output = V::Scalar> {
	fn rotate_xy(self) -> Self { Self::new(-self.y(), self.x()) }
	fn rotate_yx(self) -> Self { Self::new(self.y(), -self.x()) }
}

pub trait OrthoRot3 {
	fn rotate_xy(self) -> Self;
	fn rotate_yx(self) -> Self;
	fn rotate_xz(self) -> Self;
	fn rotate_zx(self) -> Self;
	fn rotate_yz(self) -> Self;
	fn rotate_zy(self) -> Self;
}

impl<V> OrthoRot3 for V where V: SignedVec3, V::Scalar: Neg<Output = V::Scalar> {
	fn rotate_xy(self) -> Self { Self::new(-self.y(), self.x(), self.z()) }
	fn rotate_yx(self) -> Self { Self::new(self.y(), -self.x(), self.z()) }
	fn rotate_xz(self) -> Self { Self::new(-self.z(), self.y(), self.x()) }
	fn rotate_zx(self) -> Self { Self::new(self.z(), self.y(), -self.x()) }
	fn rotate_zy(self) -> Self { Self::new(self.x(), self.z(), -self.y()) }
	fn rotate_yz(self) -> Self { Self::new(self.x(), -self.z(), self.y()) }
}
