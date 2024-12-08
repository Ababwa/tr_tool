use std::ops::Range;
use glam::IVec3;
use tr_model::{tr1, tr3};
use crate::{
	as_bytes::ReinterpretAsBytes, geom_buffer::{self, GeomBuffer},
	tr_traits::{Level, MeshTexturedFace, ObjectTexture, RoomFace, RoomVertex}, MeshFaceType, ObjectData,
	WrittenFaceArray, WrittenMesh,
};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct FaceInstance {
	face_array_index: u16,
	face_index: u16,
	transform_index: u16,
	object_data_index: u32,
}

impl ReinterpretAsBytes for FaceInstance {}

#[repr(C)]
pub struct SpriteInstance {
	pos: IVec3,
	sprite_texture_index: u16,
	object_data_index: u16,
}

impl ReinterpretAsBytes for SpriteInstance {}

pub struct MeshTexturedFaceOffsets {
	pub opaque: u32,
	pub additive: u32,
	pub end: u32,
}

impl MeshTexturedFaceOffsets {
	pub fn opaque(&self) -> Range<u32> {
		self.opaque..self.additive
	}
	
	pub fn additive(&self) -> Range<u32> {
		self.additive..self.end
	}
}

pub struct MeshFaceOffsets {
	pub textured_quads: MeshTexturedFaceOffsets,
	pub textured_tris: MeshTexturedFaceOffsets,
	pub solid_quads: Range<u32>,
	pub solid_tris: Range<u32>,
}

pub struct RoomFaceOffsets {
	pub opaque_obverse: u32,
	pub opaque_reverse: u32,
	pub additive_obverse: u32,
	pub additive_reverse: u32,
	pub end: u32,
}

impl RoomFaceOffsets {
	pub fn opaque_obverse(&self) -> Range<u32> {
		self.opaque_obverse..self.opaque_reverse
	}
	
	pub fn opaque_reverse(&self) -> Range<u32> {
		self.opaque_reverse..self.additive_obverse
	}
	
	pub fn additive_obverse(&self) -> Range<u32> {
		self.additive_obverse..self.additive_reverse
	}
	
	pub fn additive_reverse(&self) -> Range<u32> {
		self.additive_reverse..self.end
	}
}

pub struct Output {
	pub geom_output: geom_buffer::Output,
	pub face_buffer: Vec<FaceInstance>,
	pub sprite_buffer: Vec<SpriteInstance>,
	pub object_data: Vec<ObjectData>,
}

pub struct DataWriter {
	pub geom_buffer: GeomBuffer,
	face_buffer: Vec<FaceInstance>,
	sprite_buffer: Vec<SpriteInstance>,
	object_data: Vec<ObjectData>,
}

impl DataWriter {
	pub fn new(geom_buffer: GeomBuffer) -> Self {
		Self {
			geom_buffer,
			face_buffer: vec![],
			sprite_buffer: vec![],
			object_data: vec![],
		}
	}
	
	fn add_object_data(&mut self, object_data: ObjectData) -> u32 {
		let index = self.object_data.len() as u32;
		self.object_data.push(object_data);
		index
	}
	
	pub fn write_room_face_array<L: Level, F: RoomFace, O: Fn(u16) -> ObjectData>(
		&mut self, level: &L, vertex_array_offset: u32, faces: &[F], transform_index: u16,
		object_data_maker: O,
	) -> RoomFaceOffsets {
		let face_array_index = self.geom_buffer.write_face_array(faces, vertex_array_offset);
		let mut opaque_obverse_faces = Vec::with_capacity(faces.len());
		let mut opaque_reverse_faces = Vec::with_capacity(faces.len());
		let mut additive_obverse_faces = Vec::with_capacity(faces.len());
		let mut additive_reverse_faces = Vec::with_capacity(faces.len());
		for (face_index, face) in faces.iter().enumerate() {
			let face_index = face_index as u16;
			let blend_mode = level.object_textures()[face.object_texture_index() as usize].blend_mode();
			let (obverse, reverse) = if blend_mode == tr3::blend_mode::ADD {
				(&mut additive_obverse_faces, &mut additive_reverse_faces)
			} else {
				(&mut opaque_obverse_faces, &mut opaque_reverse_faces)
			};
			let object_data_index = self.add_object_data(object_data_maker(face_index));
			obverse.push(FaceInstance {
				face_array_index,
				face_index,
				transform_index,
				object_data_index,
			});
			if face.double_sided() {
				let object_data_index = self.add_object_data(ObjectData::Reverse { object_data_index });
				reverse.push(FaceInstance {
					face_array_index,
					face_index,
					transform_index,
					object_data_index,
				});
			}
		}
		let additional =
			opaque_obverse_faces.len() +
			opaque_reverse_faces.len() +
			additive_obverse_faces.len() +
			additive_reverse_faces.len();
		self.face_buffer.reserve(additional);
		let opaque_obverse = self.face_buffer.len() as u32;
		self.face_buffer.extend(opaque_obverse_faces);
		let opaque_reverse = self.face_buffer.len() as u32;
		self.face_buffer.extend(opaque_reverse_faces);
		let additive_obverse = self.face_buffer.len() as u32;
		self.face_buffer.extend(additive_obverse_faces);
		let additive_reverse = self.face_buffer.len() as u32;
		self.face_buffer.extend(additive_reverse_faces);
		let end = self.face_buffer.len() as u32;
		RoomFaceOffsets { opaque_obverse, opaque_reverse, additive_obverse, additive_reverse, end }
	}
	
	fn mesh_textured_face_array<L, F, O>(
		&mut self, level: &L, face_array: &WrittenFaceArray<F>, transform_index: u16,
		object_data_maker: O,
	) -> MeshTexturedFaceOffsets
	where L: Level, F: MeshTexturedFace, O: Fn(u16) -> ObjectData {
		let mut opaque_faces = Vec::with_capacity(face_array.faces.len());
		let mut additive_faces = Vec::with_capacity(face_array.faces.len());
		for (face_index, face) in face_array.faces.iter().enumerate() {
			let face_index = face_index as u16;
			let blend_mode = level.object_textures()[face.object_texture_index() as usize].blend_mode();
			let faces_list = if blend_mode == tr3::blend_mode::ADD || face.additive() {
				&mut additive_faces
			} else {
				&mut opaque_faces
			};
			let object_data_index = self.add_object_data(object_data_maker(face_index));
			faces_list.push(FaceInstance {
				face_array_index: face_array.index,
				face_index,
				transform_index,
				object_data_index,
			});
		}
		self.face_buffer.reserve(face_array.faces.len());
		let opaque = self.face_buffer.len() as u32;
		self.face_buffer.extend(opaque_faces);
		let additive = self.face_buffer.len() as u32;
		self.face_buffer.extend(additive_faces);
		let end = self.face_buffer.len() as u32;
		MeshTexturedFaceOffsets { opaque, additive, end }
	}
	
	fn mesh_solid_face_array<F, O: Fn(u16) -> ObjectData>(
		&mut self, face_array: &WrittenFaceArray<F>, transform_index: u16, object_data_maker: O,
	) -> Range<u32> {
		self.face_buffer.reserve(face_array.faces.len());
		let start = self.face_buffer.len() as u32;
		for face_index in 0..face_array.faces.len() as u16 {
			let object_data_index = self.add_object_data(object_data_maker(face_index));
			self.face_buffer.push(FaceInstance {
				face_array_index: face_array.index,
				face_index,
				transform_index,
				object_data_index,
			});
		}
		let end = self.face_buffer.len() as u32;
		start..end
	}
	
	pub fn place_mesh<L: Level, O: Fn(MeshFaceType, u16) -> ObjectData>(
		&mut self, level: &L, mesh: &WrittenMesh<L>, transform_index: u16, object_data_maker: O,
	) -> MeshFaceOffsets {
		MeshFaceOffsets {
			textured_quads: self.mesh_textured_face_array(
				level, &mesh.textured_quads, transform_index,
				|face_index| object_data_maker(MeshFaceType::TexturedQuad, face_index),
			),
			textured_tris: self.mesh_textured_face_array(
				level, &mesh.textured_tris, transform_index,
				|face_index| object_data_maker(MeshFaceType::TexturedTri, face_index),
			),
			solid_quads: self.mesh_solid_face_array(
				&mesh.solid_quads, transform_index,
				|face_index| object_data_maker(MeshFaceType::SolidQuad, face_index),
			),
			solid_tris: self.mesh_solid_face_array(
				&mesh.solid_tris, transform_index,
				|face_index| object_data_maker(MeshFaceType::SolidTri, face_index),
			),
		}
	}
	
	pub fn sprite_offset(&self) -> u32 {
		self.sprite_buffer.len() as u32
	}
	
	pub fn write_room_sprites<V: RoomVertex, O: Fn(u16) -> ObjectData>(
		&mut self, room_pos: IVec3, vertices: &[V], sprites: &[tr1::Sprite], object_data_maker: O,
	) -> Range<u32> {
		let start = self.sprite_buffer.len() as u32;
		for &tr1::Sprite { vertex_index, sprite_texture_index } in sprites {
			let object_data_index = self.add_object_data(object_data_maker(sprite_texture_index)) as u16;
			self.sprite_buffer.push(SpriteInstance {
				pos: room_pos + vertices[vertex_index as usize].pos().as_ivec3(),
				sprite_texture_index,
				object_data_index,
			});
		}
		let end = self.sprite_buffer.len() as u32;
		start..end
	}
	
	pub fn write_entity_sprite(&mut self, entity_index: u16, pos: IVec3, sprite_texture_index: u16) {
		let object_data_index = self.add_object_data(ObjectData::EntitySprite { entity_index }) as u16;
		self.sprite_buffer.push(SpriteInstance { pos, sprite_texture_index, object_data_index });
	}
	
	pub fn done<O: ReinterpretAsBytes>(
		self, object_textures: &[O], sprite_textures: &[tr1::SpriteTexture],
	) -> Output {
		Output {
			geom_output: self.geom_buffer.into_buffer(object_textures, sprite_textures),
			face_buffer: self.face_buffer,
			sprite_buffer: self.sprite_buffer,
			object_data: self.object_data,
		}
	}
}
