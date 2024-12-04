use std::{mem::MaybeUninit, ops::Range};
use glam::IVec3;
use shared::alloc;
use tr_model::{tr1, tr3};
use crate::{
	as_bytes::{ReinterpretAsBytes, ToBytes}, fixed_vec::FixedVec, geom_buffer::GeomBuffer,
	tr_traits::{Level, MeshTexturedFace, ObjectTexture, RoomFace, RoomVertex}, MeshFaceType, ObjectData,
	WrittenFaceArray, WrittenMesh,
};

const MAX_FACES: usize = 65536;
const MAX_SPRITES: usize = 128;
const MAX_OBJ_DATA: usize = 65536;

#[repr(C)]
#[derive(Clone, Copy)]
struct FaceInstance {
	face_array_index: u16,
	face_index: u16,
	transform_index: u16,
	object_data_index: u16,
}

impl FaceInstance {
	fn new(
		face_array_index: usize, face_index: usize, transform_index: usize, object_data_index: usize,
	) -> Self {
		Self {
			face_array_index: face_array_index.try_into().unwrap(),
			face_index: face_index.try_into().unwrap(),
			transform_index: transform_index.try_into().unwrap(),
			object_data_index: object_data_index.try_into().unwrap(),
		}
	}
}

impl ReinterpretAsBytes for FaceInstance {}

#[repr(C)]
struct SpriteInstance {
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
	pub original: u32,
	pub additive: u32,
	pub reverse: u32,
	pub additive_reverse: u32,
	pub end: u32,
}

impl RoomFaceOffsets {
	pub fn original(&self) -> Range<u32> {
		self.original..self.additive
	}
	
	pub fn additive(&self) -> Range<u32> {
		self.additive..self.reverse
	}
	
	pub fn reverse(&self) -> Range<u32> {
		self.reverse..self.additive_reverse
	}
	
	pub fn reverse_additive(&self) -> Range<u32> {
		self.additive_reverse..self.end
	}
}

pub struct Results {
	pub geom_buffer: Box<[u8]>,
	pub face_buffer: Box<[u8]>,
	pub sprite_buffer: Box<[u8]>,
	pub object_data: Vec<ObjectData>,
}

pub struct DataWriter {
	pub geom_buffer: GeomBuffer,
	face_buffer: Box<[MaybeUninit<FaceInstance>; MAX_FACES]>,//raw array for out-of-order initialization
	num_written_faces: usize,
	sprite_buffer: FixedVec<SpriteInstance, MAX_SPRITES>,
	object_data: FixedVec<ObjectData, MAX_OBJ_DATA>,
}

impl DataWriter {
	pub fn new(geom_buffer: GeomBuffer) -> Self {
		Self {
			geom_buffer,
			face_buffer: alloc::array(),
			num_written_faces: 0,
			sprite_buffer: FixedVec::new(),
			object_data: FixedVec::new(),
		}
	}
	
	pub fn write_room_face_array<L: Level, F: RoomFace, O: Fn(usize) -> ObjectData>(
		&mut self, level: &L, vertex_array_offset: usize, faces: &[F], transform_index: usize,
		object_data_maker: O,
	) -> RoomFaceOffsets {
		let face_array_index = self.geom_buffer.write_face_array(faces, vertex_array_offset);
		let mut double_sided = Vec::with_capacity(faces.len());
		let original = self.num_written_faces;
		let mut additive = original + faces.len();
		let reverse = additive;
		for (face_index, face) in faces.iter().enumerate() {
			let blend_mode = level.object_textures()[face.object_texture_index() as usize].blend_mode();
			let is_additive = blend_mode == tr3::blend_mode::ADD;
			let index = if is_additive {
				additive -= 1;
				additive
			} else {
				face_index + additive - faces.len()
			};
			let face_instance = FaceInstance::new(
				face_array_index, face_index, transform_index, self.object_data.len(),
			);
			self.face_buffer[index].write(face_instance);
			self.object_data.push(object_data_maker(face_index));
			if face.double_sided() {
				double_sided.push((face_instance, is_additive));
			}
		}
		let mut additive_reverse = reverse + double_sided.len();
		let end = additive_reverse;
		for (ds_index, &(mut face_instance, is_additive)) in double_sided.iter().enumerate() {
			let index = if is_additive {
				additive_reverse -= 1;
				additive_reverse
			} else {
				ds_index + additive_reverse - double_sided.len()
			};
			let object_data = ObjectData::Reverse { object_data_index: face_instance.object_data_index };
			face_instance.object_data_index = self.object_data.len() as u16;
			self.face_buffer[index].write(face_instance);
			self.object_data.push(object_data);
		}
		self.num_written_faces = end;
		let original = original as u32;
		let additive = additive as u32;
		let reverse = reverse as u32;
		let additive_reverse = additive_reverse as u32;
		let end = end as u32;
		RoomFaceOffsets { original, additive, reverse, additive_reverse, end }
	}
	
	fn mesh_textured_face_array<L, F, O>(
		&mut self, level: &L, face_array: &WrittenFaceArray<F>, transform_index: usize,
		object_data_maker: O,
	) -> MeshTexturedFaceOffsets
	where L: Level, F: MeshTexturedFace, O: Fn(usize) -> ObjectData {
		let opaque = self.num_written_faces;
		let mut additive = opaque + face_array.faces.len();
		let end = additive;
		for (face_index, face) in face_array.faces.iter().enumerate() {
			let blend_mode = level.object_textures()[face.object_texture_index() as usize].blend_mode();
			let index = if blend_mode == tr3::blend_mode::ADD || face.additive() {
				additive -= 1;
				additive
			} else {
				face_index + additive - face_array.faces.len()
			};
			let face_instance = FaceInstance::new(
				face_array.index, face_index, transform_index, self.object_data.len(),
			);
			self.face_buffer[index].write(face_instance);
			self.object_data.push(object_data_maker(face_index));
		}
		self.num_written_faces = end;
		let opaque = opaque as u32;
		let additive = additive as u32;
		let end = end as u32;
		MeshTexturedFaceOffsets { opaque, additive, end }
	}
	
	fn mesh_solid_face_array<F, O: Fn(usize) -> ObjectData>(
		&mut self, face_array: &WrittenFaceArray<F>, transform_index: usize, object_data_maker: O,
	) -> Range<u32> {
		let start = self.num_written_faces;
		let end = start + face_array.faces.len();
		for face_index in 0..face_array.faces.len() {
			let face_instance = FaceInstance::new(
				face_array.index, face_index, transform_index, self.object_data.len(),
			);
			self.face_buffer[start + face_index].write(face_instance);
			self.object_data.push(object_data_maker(face_index));
		}
		self.num_written_faces = end;
		let start = start as u32;
		let end = end as u32;
		start..end
	}
	
	pub fn place_mesh<L: Level, O: Fn(MeshFaceType, usize) -> ObjectData>(
		&mut self, level: &L, mesh: &WrittenMesh<L>, transform_index: usize, object_data_maker: O,
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
	) {
		for &tr1::Sprite { vertex_index, sprite_texture_index } in sprites {
			self.sprite_buffer.push(SpriteInstance {
				pos: room_pos + vertices[vertex_index as usize].pos().as_ivec3(),
				sprite_texture_index,
				object_data_index: self.object_data.len() as u16,
			});
			self.object_data.push(object_data_maker(sprite_texture_index));
		}
	}
	
	pub fn write_entity_sprite(&mut self, entity_index: usize, pos: IVec3, sprite_texture_index: u16) {
		self.sprite_buffer.push(SpriteInstance {
			pos,
			sprite_texture_index,
			object_data_index: self.object_data.len() as u16,
		});
		self.object_data.push(ObjectData::entity_sprite(entity_index));
	}
	
	pub fn done(self) -> Results {
		Results {
			geom_buffer: self.geom_buffer.into_buffer(),
			face_buffer: self.face_buffer.to_bytes(),
			sprite_buffer: self.sprite_buffer.into_inner().to_bytes(),
			object_data: self.object_data.into_vec(),
		}
	}
}
