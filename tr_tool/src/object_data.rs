//! Metadata for rendered objects sufficient to locate the object in the level structure.

use std::fmt::Debug;
use tr_model::tr1;
use crate::tr_traits::{
	self, Entity, Face, Level, Mesh, Model, ObjectTexture, Room, RoomStaticMesh, SolidFace, TexturedFace,
	TexturedMeshFace,
};

#[derive(Clone, Copy, Debug)]
pub enum RoomFaceKind {
	Quad,
	Tri,
}

#[derive(Clone, Copy, Debug)]
pub enum MeshFaceKind {
	TexturedQuad,
	TexturedTri,
	SolidQuad,
	SolidTri,
}

#[derive(Clone, Copy, Debug)]
pub struct RoomFace {
	pub room_index: u16,
	pub layer_index: u8,
	pub face_kind: RoomFaceKind,
	pub face_index: u16,
}

#[derive(Clone, Copy, Debug)]
pub struct StaticMeshFace {
	pub room_index: u16,
	pub room_static_mesh_index: u8,
	pub face_kind: MeshFaceKind,
	pub face_index: u16,
}

#[derive(Clone, Copy, Debug)]
pub struct RoomSprite {
	pub room_index: u16,
	pub sprite_index: u16,
}

#[derive(Clone, Copy, Debug)]
pub struct EntityMeshFace {
	pub entity_index: u16,
	pub mesh_index: u8,
	pub face_kind: MeshFaceKind,
	pub face_index: u16,
}

#[derive(Clone, Copy, Debug)]
pub struct EntitySprite {
	pub entity_index: u16,
}

#[derive(Clone, Copy, Debug)]
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

fn print_face<const N: usize, V: Debug, F: Face<N>>(vertices: &[V], face: &F) {
	for index in face.vertex_indices() {
		println!("{:?}", vertices[index as usize]);
	}
}

fn print_textured_face<const N: usize, O, V, F>(object_textures: &[O], vertices: &[V], face: &F)
where O: ObjectTexture, V: Debug, F: TexturedFace<N> {
	println!("{:?}", object_textures[face.object_texture_index() as usize]);
	print_face(vertices, face);
}

fn print_room_face<const N: usize, L, R, F>(level: &L, room: &R, face: &F)
where L: Level, R: Room, F: tr_traits::RoomFace<N> {
	println!("{:?}", face);
	print_textured_face(level.object_textures(), room.vertices(), face);
}

fn print_textured_mesh_face<const N: usize, L, M, F>(level: &L, mesh: &M, face: &F)
where L: Level, M: Mesh, F: TexturedMeshFace<N> {
	println!("{:?}", face);
	print_textured_face(level.object_textures(), mesh.vertices(), face);
}

fn print_solid_mesh_face<const N: usize, L, M, F>(level: &L, mesh: &M, face: &F)
where L: Level, M: Mesh, F: SolidFace<N> {
	println!("{:?}", face);
	let color_24 = level.palette_24bit().unwrap()[face.color_index_24bit() as usize];
	println!("24 bit color: {:?}", color_24);
	if let Some(color_index_32bit) = face.color_index_32bit() {
		let color_32 = level.palette_32bit().unwrap()[color_index_32bit as usize];
		println!("32 bit color: {:?}", color_32);
	}
	print_face(mesh.vertices(), face);
}

fn print_mesh_face<L: Level>(level: &L, mesh_offset: u32, face_kind: MeshFaceKind, face_index: u16) {
	let mesh = level.get_mesh(mesh_offset);
	let index = face_index as usize;
	match face_kind {
		MeshFaceKind::TexturedQuad => print_textured_mesh_face(level, &mesh, &mesh.textured_quads()[index]),
		MeshFaceKind::TexturedTri => print_textured_mesh_face(level, &mesh, &mesh.textured_tris()[index]),
		MeshFaceKind::SolidQuad => print_solid_mesh_face(level, &mesh, &mesh.solid_quads()[index]),
		MeshFaceKind::SolidTri => print_solid_mesh_face(level, &mesh, &mesh.solid_tris()[index]),
	}
}

fn get_static_mesh(static_meshes: &[tr1::StaticMesh], id: u32) -> &tr1::StaticMesh {
	for static_mesh in static_meshes {
		if static_mesh.id == id {
			return static_mesh;
		}
	}
	panic!("missing static mesh with id {}", id);
}

fn get_model<M: Model>(models: &[M], id: u32) -> &M {
	for model in models {
		if model.id() == id {
			return model;
		}
	}
	panic!("missing model with id {}", id);
}

pub fn print_object_data<L: Level>(level: &L, object_data: ObjectData) {
	match object_data {
		ObjectData::RoomFace(room_face) => {
			println!("{:?}", room_face);
			let room = &level.rooms()[room_face.room_index as usize];
			let layer = room.iter_layers().nth(room_face.layer_index as usize).expect("layer missing");
			let index = room_face.face_index as usize;
			match room_face.face_kind {
				RoomFaceKind::Quad => print_room_face(level, room, &layer.quads[index]),
				RoomFaceKind::Tri => print_room_face(level, room, &layer.tris[index]),
			}
		},
		ObjectData::StaticMeshFace(static_mesh_face) => {
			println!("{:?}", static_mesh_face);
			let room = &level.rooms()[static_mesh_face.room_index as usize];
			let rsm = &room.room_static_meshes()[static_mesh_face.room_static_mesh_index as usize];
			let static_mesh = get_static_mesh(level.static_meshes(), rsm.static_mesh_id() as u32);
			let mesh_offset = level.mesh_offsets()[static_mesh.mesh_offset_index as usize];
			print_mesh_face(level, mesh_offset, static_mesh_face.face_kind, static_mesh_face.face_index);
		},
		ObjectData::RoomSprite(room_sprite) => {
			println!("{:?}", room_sprite);
			let room = &level.rooms()[room_sprite.room_index as usize];
			let sprite = room.sprites()[room_sprite.sprite_index as usize];
			println!("{:?}", sprite);
		},
		ObjectData::EntityMeshFace(entity_mesh_face) => {
			println!("{:?}", entity_mesh_face);
			let entity = &level.entities()[entity_mesh_face.entity_index as usize];
			let model = get_model(level.models(), entity.model_id() as u32);
			let mesh_offset_index = model.mesh_offset_index() + entity_mesh_face.mesh_index as u16;
			let mesh_offset = level.mesh_offsets()[mesh_offset_index as usize];
			print_mesh_face(level, mesh_offset, entity_mesh_face.face_kind, entity_mesh_face.face_index);
		},
		ObjectData::EntitySprite(entity_sprite) => {
			println!("{:?}", entity_sprite);
			let entity = &level.entities()[entity_sprite.entity_index as usize];
			println!("{:?}", entity);
		},
	}
}
