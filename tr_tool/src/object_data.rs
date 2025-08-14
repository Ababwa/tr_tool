//! Metadata for rendered objects sufficient to locate the object in the level structure.

#[derive(Clone, Copy)]
pub enum RoomFaceKind {
	Quad,
	Tri,
}

#[derive(Clone, Copy)]
pub enum MeshFaceKind {
	TexturedQuad,
	TexturedTri,
	SolidQuad,
	SolidTri,
}

#[derive(Clone, Copy)]
pub struct RoomFace {
	pub room_index: u16,
	pub layer_index: u8,
	pub face_kind: RoomFaceKind,
	pub face_index: u16,
}

#[derive(Clone, Copy)]
pub struct StaticMeshFace {
	pub room_index: u16,
	pub room_static_mesh_index: u8,
	pub face_kind: MeshFaceKind,
	pub face_index: u16,
}

#[derive(Clone, Copy)]
pub struct RoomSprite {
	pub room_index: u16,
	pub sprite_index: u16,
}

#[derive(Clone, Copy)]
pub struct EntityMeshFace {
	pub entity_index: u16,
	pub mesh_index: u8,
	pub face_kind: MeshFaceKind,
	pub face_index: u16,
}

#[derive(Clone, Copy)]
pub struct EntitySprite {
	pub entity_index: u16,
}

pub enum ObjectData {
	RoomFace(RoomFace),
	StaticMeshFace(StaticMeshFace),
	RoomSprite(RoomSprite),
	EntityMeshFace(EntityMeshFace),
	EntitySprite(EntitySprite),
}

pub trait MeshFaceData: Copy {
	fn with_face_index(self, face_index: u16) -> ObjectData;
	fn with_face_kind(self, face_kind: MeshFaceKind) -> Self;
}

impl MeshFaceData for StaticMeshFace {
	fn with_face_index(self, face_index: u16) -> ObjectData {
		let face_data = Self {
			face_index,
			..self
		};
		ObjectData::StaticMeshFace(face_data)
	}
	
	fn with_face_kind(self, face_kind: MeshFaceKind) -> Self {
		Self {
			face_kind,
			..self
		}
	}
}

impl MeshFaceData for EntityMeshFace {
	fn with_face_index(self, face_index: u16) -> ObjectData {
		let face_data = Self {
			face_index,
			..self
		};
		ObjectData::EntityMeshFace(face_data)
	}
	
	fn with_face_kind(self, face_kind: MeshFaceKind) -> Self {
		Self {
			face_kind,
			..self
		}
	}
}
