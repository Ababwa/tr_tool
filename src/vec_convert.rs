use glam_traits::glam::{Vec2, Vec3};

pub trait ToBevy {
	type BevyType;
	fn to_bevy(self) -> Self::BevyType;
}

impl ToBevy for Vec2 {
	type BevyType = bevy::math::Vec2;
	
	fn to_bevy(self) -> Self::BevyType {
		Self::BevyType::new(self.x, self.y)
	}
}

impl ToBevy for Vec3 {
	type BevyType = bevy::math::Vec3;
	
	fn to_bevy(self) -> Self::BevyType {
		Self::BevyType::new(self.x, self.y, self.z)
	}
}

pub trait ToGlam {
	type GlamType;
	fn to_glam(self) -> Self::GlamType;
}

impl ToGlam for bevy::math::Vec2 {
	type GlamType = Vec2;
	
	fn to_glam(self) -> Self::GlamType {
		Self::GlamType::new(self.x, self.y)
	}
}

impl ToGlam for bevy::math::Vec3 {
	type GlamType = Vec3;
	
	fn to_glam(self) -> Self::GlamType {
		Self::GlamType::new(self.x, self.y, self.z)
	}
}