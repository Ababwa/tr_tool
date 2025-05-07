use bitflags::bitflags;

macro_rules! impl_i64_enum {
	($name:ty) => {
		impl From<$name> for i64 {
			fn from(value: $name) -> Self {
				value as i64
			}
		}
	};
}

macro_rules! impl_i64_flags {
	($name:ty) => {
		impl From<$name> for i64 {
			fn from(value: $name) -> Self {
				value.0.0 as i64
			}
		}
	};
}

pub enum GameVersion {
	Tr1 = 1,
	Tr2 = 2,
	Tr3 = 3,
	Tr4 = 4,
	Tr5 = 5,
	Trng = 16,
	TombEngine = 18,
}
impl_i64_enum!(GameVersion);

pub enum RoomType {
	Normal = 0,
	Rain = 1,
	Snow = 2,
	Water = 3,
	Quicksand = 4,
}
impl_i64_enum!(RoomType);

pub enum SoundSystem {
	None = 0,
	Xml = 1,
}
impl_i64_enum!(SoundSystem);

pub enum SectorCorner {
	XnZp = 1,
	XpZp = 2,
	XpZn = 3,
	XnZn = 4,
}
impl_i64_enum!(SectorCorner);

pub enum SectorFace {
	WallPositiveZFloor1 = 0,
	WallNegativeZFloor1 = 1,
	WallNegativeXFloor1 = 2,
	WallPositiveXFloor1 = 3,
	WallDiagonalFloor1 = 4,
	
	WallPositiveZFloor2 = 5,
	WallNegativeZFloor2 = 6,
	WallNegativeXFloor2 = 7,
	WallPositiveXFloor2 = 8,
	WallDiagonalFloor2 = 9,
	
	WallPositiveZMiddle = 10,
	WallNegativeZMiddle = 11,
	WallNegativeXMiddle = 12,
	WallPositiveXMiddle = 13,
	WallDiagonalMiddle = 14,
	
	WallPositiveZCeiling1 = 15,
	WallNegativeZCeiling1 = 16,
	WallNegativeXCeiling1 = 17,
	WallPositiveXCeiling1 = 18,
	WallDiagonalCeiling1 = 19,
	
	WallPositiveZCeiling2 = 20,
	WallNegativeZCeiling2 = 21,
	WallNegativeXCeiling2 = 22,
	WallPositiveXCeiling2 = 23,
	WallDiagonalCeiling2 = 24,
	
	Floor = 25,
	FloorTriangle2 = 26,
	Ceiling = 27,
	CeilingTriangle2 = 28,
	
	WallPositiveZFloor3 = 29,
	WallNegativeZFloor3 = 30,
	WallNegativeXFloor3 = 31,
	WallPositiveXFloor3 = 32,
	WallDiagonalFloor3 = 33,
	
	WallPositiveZCeiling3 = 34,
	WallNegativeZCeiling3 = 35,
	WallNegativeXCeiling3 = 36,
	WallPositiveXCeiling3 = 37,
	WallDiagonalCeiling3 = 38,
	
	WallPositiveZFloor4 = 39,
	WallNegativeZFloor4 = 40,
	WallNegativeXFloor4 = 41,
	WallPositiveXFloor4 = 42,
	WallDiagonalFloor4 = 43,
	
	WallPositiveZCeiling4 = 44,
	WallNegativeZCeiling4 = 45,
	WallNegativeXCeiling4 = 46,
	WallPositiveXCeiling4 = 47,
	WallDiagonalCeiling4 = 48,
	
	WallPositiveZFloor5 = 49,
	WallNegativeZFloor5 = 50,
	WallNegativeXFloor5 = 51,
	WallPositiveXFloor5 = 52,
	WallDiagonalFloor5 = 53,
	
	WallPositiveZCeiling5 = 54,
	WallNegativeZCeiling5 = 55,
	WallNegativeXCeiling5 = 56,
	WallPositiveXCeiling5 = 57,
	WallDiagonalCeiling5 = 58,
	
	WallPositiveZFloor6 = 59,
	WallNegativeZFloor6 = 60,
	WallNegativeXFloor6 = 61,
	WallPositiveXFloor6 = 62,
	WallDiagonalFloor6 = 63,
	
	WallPositiveZCeiling6 = 64,
	WallNegativeZCeiling6 = 65,
	WallNegativeXCeiling6 = 66,
	WallPositiveXCeiling6 = 67,
	WallDiagonalCeiling6 = 68,
	
	WallPositiveZFloor7 = 69,
	WallNegativeZFloor7 = 70,
	WallNegativeXFloor7 = 71,
	WallPositiveXFloor7 = 72,
	WallDiagonalFloor7 = 73,
	
	WallPositiveZCeiling7 = 74,
	WallNegativeZCeiling7 = 75,
	WallNegativeXCeiling7 = 76,
	WallPositiveXCeiling7 = 77,
	WallDiagonalCeiling7 = 78,
	
	WallPositiveZFloor8 = 79,
	WallNegativeZFloor8 = 80,
	WallNegativeXFloor8 = 81,
	WallPositiveXFloor8 = 82,
	WallDiagonalFloor8 = 83,
	
	WallPositiveZCeiling8 = 84,
	WallNegativeZCeiling8 = 85,
	WallNegativeXCeiling8 = 86,
	WallPositiveXCeiling8 = 87,
	WallDiagonalCeiling8 = 88,
	
	WallPositiveZFloor9 = 89,
	WallNegativeZFloor9 = 90,
	WallNegativeXFloor9 = 91,
	WallPositiveXFloor9 = 92,
	WallDiagonalFloor9 = 93,
	
	WallPositiveZCeiling9 = 94,
	WallNegativeZCeiling9 = 95,
	WallNegativeXCeiling9 = 96,
	WallPositiveXCeiling9 = 97,
	WallDiagonalCeiling9 = 98,
}
impl_i64_enum!(SectorFace);

impl SectorFace {
	pub fn from_side_level(side: usize, level: i8) -> Self {
		use super::{XN, XP, ZN, ZP};
		match (side, level) {
			(XN, 0) => Self::WallNegativeXMiddle,
			(ZP, 0) => Self::WallPositiveZMiddle,
			(XP, 0) => Self::WallPositiveXMiddle,
			(ZN, 0) => Self::WallNegativeZMiddle,
			
			(XN, 1) => Self::WallNegativeXFloor1,
			(ZP, 1) => Self::WallPositiveZFloor1,
			(XP, 1) => Self::WallPositiveXFloor1,
			(ZN, 1) => Self::WallNegativeZFloor1,
			
			(XN, 2) => Self::WallNegativeXFloor2,
			(ZP, 2) => Self::WallPositiveZFloor2,
			(XP, 2) => Self::WallPositiveXFloor2,
			(ZN, 2) => Self::WallNegativeZFloor2,
			
			(XN, 3) => Self::WallNegativeXFloor3,
			(ZP, 3) => Self::WallPositiveZFloor3,
			(XP, 3) => Self::WallPositiveXFloor3,
			(ZN, 3) => Self::WallNegativeZFloor3,
			
			(XN, 4) => Self::WallNegativeXFloor4,
			(ZP, 4) => Self::WallPositiveZFloor4,
			(XP, 4) => Self::WallPositiveXFloor4,
			(ZN, 4) => Self::WallNegativeZFloor4,
			
			(XN, 5) => Self::WallNegativeXFloor5,
			(ZP, 5) => Self::WallPositiveZFloor5,
			(XP, 5) => Self::WallPositiveXFloor5,
			(ZN, 5) => Self::WallNegativeZFloor5,
			
			(XN, 6) => Self::WallNegativeXFloor6,
			(ZP, 6) => Self::WallPositiveZFloor6,
			(XP, 6) => Self::WallPositiveXFloor6,
			(ZN, 6) => Self::WallNegativeZFloor6,
			
			(XN, 7) => Self::WallNegativeXFloor7,
			(ZP, 7) => Self::WallPositiveZFloor7,
			(XP, 7) => Self::WallPositiveXFloor7,
			(ZN, 7) => Self::WallNegativeZFloor7,
			
			(XN, 8) => Self::WallNegativeXFloor8,
			(ZP, 8) => Self::WallPositiveZFloor8,
			(XP, 8) => Self::WallPositiveXFloor8,
			(ZN, 8) => Self::WallNegativeZFloor8,
			
			(XN, 9) => Self::WallNegativeXFloor9,
			(ZP, 9) => Self::WallPositiveZFloor9,
			(XP, 9) => Self::WallPositiveXFloor9,
			(ZN, 9) => Self::WallNegativeZFloor9,
			
			(XN, -1) => Self::WallNegativeXCeiling1,
			(ZP, -1) => Self::WallPositiveZCeiling1,
			(XP, -1) => Self::WallPositiveXCeiling1,
			(ZN, -1) => Self::WallNegativeZCeiling1,
			
			(XN, -2) => Self::WallNegativeXCeiling2,
			(ZP, -2) => Self::WallPositiveZCeiling2,
			(XP, -2) => Self::WallPositiveXCeiling2,
			(ZN, -2) => Self::WallNegativeZCeiling2,
			
			(XN, -3) => Self::WallNegativeXCeiling3,
			(ZP, -3) => Self::WallPositiveZCeiling3,
			(XP, -3) => Self::WallPositiveXCeiling3,
			(ZN, -3) => Self::WallNegativeZCeiling3,
			
			(XN, -4) => Self::WallNegativeXCeiling4,
			(ZP, -4) => Self::WallPositiveZCeiling4,
			(XP, -4) => Self::WallPositiveXCeiling4,
			(ZN, -4) => Self::WallNegativeZCeiling4,
			
			(XN, -5) => Self::WallNegativeXCeiling5,
			(ZP, -5) => Self::WallPositiveZCeiling5,
			(XP, -5) => Self::WallPositiveXCeiling5,
			(ZN, -5) => Self::WallNegativeZCeiling5,
			
			(XN, -6) => Self::WallNegativeXCeiling6,
			(ZP, -6) => Self::WallPositiveZCeiling6,
			(XP, -6) => Self::WallPositiveXCeiling6,
			(ZN, -6) => Self::WallNegativeZCeiling6,
			
			(XN, -7) => Self::WallNegativeXCeiling7,
			(ZP, -7) => Self::WallPositiveZCeiling7,
			(XP, -7) => Self::WallPositiveXCeiling7,
			(ZN, -7) => Self::WallNegativeZCeiling7,
			
			(XN, -8) => Self::WallNegativeXCeiling8,
			(ZP, -8) => Self::WallPositiveZCeiling8,
			(XP, -8) => Self::WallPositiveXCeiling8,
			(ZN, -8) => Self::WallNegativeZCeiling8,
			
			(XN, -9) => Self::WallNegativeXCeiling9,
			(ZP, -9) => Self::WallPositiveZCeiling9,
			(XP, -9) => Self::WallPositiveXCeiling9,
			(ZN, -9) => Self::WallNegativeZCeiling9,
			
			_ => panic!("unknown sector face: {}, {}", side, level),
		}
	}
}

/// Indicates the corner tri that is a step or wall. The other corner tri must be flat.
// #[derive(Clone, Copy)]
// pub enum DiagonalCorner {
// 	XnZp = 1,
// 	XpZp = 2,
// 	XpZn = 3,
// 	XnZn = 4,
// }

pub fn diag_details_from_corner(corner: u8) -> u8 {
	(1 - (corner % 2)) | (corner + 1)
}

bitflags! {
	pub struct SectorFlags: u16 {
		const WALL = 1 << 0;
		const FORCE_FLOOR_SOLID = 1 << 1;
		const MONKEY = 1 << 2;
		const BOX = 1 << 3;
		const DEATH_FIRE = 1 << 4;
		const DEATH_LAVA = 1 << 5;
		const DEATH_ELECTRICITY = 1 << 6;
		const BEETLE = 1 << 7;
		const TRIGGER_TRIGGERER = 1 << 8;
		const NOT_WALKABLE_FLOOR = 1 << 9;
		const CLIMB_POSITIVE_Z = 1 << 10;
		const CLIMB_NEGATIVE_Z = 1 << 11;
		const CLIMB_POSITIVE_X = 1 << 12;
		const CLIMB_NEGATIVE_X = 1 << 13;
	}
}
impl_i64_flags!(SectorFlags);
