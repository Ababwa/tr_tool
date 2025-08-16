use std::{mem::MaybeUninit, ops::Range};
use glam::IVec3;
use tr_model::{tr1, tr3};
use crate::{
	boxed_slice::Bsf, object_data::{self, MeshFaceData},
	tr_traits::{Entity, ObjectTexture, RoomFace, RoomVertex, RoomVertexPos, TexturedMeshFace},
};
use super::{counts::Counts, geom_writer::{FaceArrayIndex, TransformIndex}, maps::SpriteEntity};

//TODO: try to compact into 64 bits
#[repr(C)]
#[derive(Clone, Copy)]
pub struct FaceInstance {
	face_array_index: FaceArrayIndex,
	face_index: u16,
	transform_index: TransformIndex,
	object_data_index: u32,
}

#[repr(C)]
pub struct SpriteInstance {
	pos: IVec3,
	sprite_texture_index: u16,
	object_data_index: u16,
}

/**
Defines four ranges, with the reverse ranges being subranges of the obverse ranges:<br>
Opaque obverse: `[opaque_single..additive_single]`<br>
Opaque reverse: `[opaque_double..additive_single]`<br>
Additive obverse: `[additive_single..end]`<br>
Additive reverse: `[additive_double..end]`
*/
pub struct RoomFaceOffsets {
	opaque_single: u32,
	opaque_double: u32,
	additive_single: u32,
	additive_double: u32,
	end: u32,
}

/**
Defines two ranges:<br>
Opaque: `[opaque..additive]`<br>
Additive: `[additive..end]`
*/
pub struct TexturedMeshFaceOffsets {
	opaque: u32,
	additive: u32,
	end: u32,
}

pub struct InstanceWriter {
	face_instances: Box<[MaybeUninit<FaceInstance>]>,
	face_instances_pos: usize,
	sprite_instances: Bsf<SpriteInstance>,
	object_data: Box<[MaybeUninit<object_data::ObjectData>]>,
	object_data_sprites_pos: usize,
	object_data_faces_start: usize,
	object_data_faces_pos: usize,
}

pub struct Instances {
	pub face_instances: Box<[FaceInstance]>,
	pub sprite_instances: Box<[SpriteInstance]>,
	pub object_data: Box<[object_data::ObjectData]>,
}

fn get_additive_double<const N: usize, O: ObjectTexture, F: RoomFace<N>>(
	object_textures: &[O],
	face: &F,
) -> (bool, bool) {
	let object_texture = &object_textures[face.object_texture_index() as usize];
	let additive = object_texture.blend_mode() == tr3::blend_mode::ADD;
	let double = face.double_sided();
	(additive, double)
}

impl RoomFaceOffsets {
	pub const fn opaque_obverse(&self) -> Range<u32> {
		self.opaque_single..self.additive_single
	}
	
	pub const fn opaque_reverse(&self) -> Range<u32> {
		self.opaque_double..self.additive_single
	}
	
	pub const fn additive_obverse(&self) -> Range<u32> {
		self.additive_single..self.end
	}
	
	pub const fn additive_reverse(&self) -> Range<u32> {
		self.additive_double..self.end
	}
}

impl TexturedMeshFaceOffsets {
	pub const fn opaque(&self) -> Range<u32> {
		self.opaque..self.additive
	}
	
	pub const fn additive(&self) -> Range<u32> {
		self.additive..self.end
	}
}

impl InstanceWriter {
	pub fn new(counts: &Counts) -> Self {
		let &Counts { face_instances, sprite_instances, .. } = counts;
		Self {
			face_instances: Box::new_uninit_slice(face_instances),
			face_instances_pos: 0,
			sprite_instances: Bsf::new(sprite_instances),
			object_data: Box::new_uninit_slice(face_instances + sprite_instances),
			object_data_sprites_pos: 0,
			object_data_faces_start: sprite_instances,
			object_data_faces_pos: sprite_instances,
		}
	}
	
	fn write_face_object_data(&mut self, face_data: object_data::ObjectData) -> u32 {
		self.object_data[self.object_data_faces_pos] = MaybeUninit::new(face_data);
		let index = self.object_data_faces_pos as u32;
		self.object_data_faces_pos += 1;
		index
	}
	
	fn write_sprite_object_data(&mut self, sprite_data: object_data::ObjectData) -> u16 {
		self.object_data[self.object_data_sprites_pos] = MaybeUninit::new(sprite_data);
		let index = self.object_data_sprites_pos as u16;
		self.object_data_sprites_pos += 1;
		index
	}
	
	pub fn room_face_array<const N: usize, O: ObjectTexture, F: RoomFace<N>>(
		&mut self,
		object_textures: &[O],
		transform_index: TransformIndex,
		face_array_index: FaceArrayIndex,
		face_data: object_data::RoomFace,
		faces: &[F],
	) -> RoomFaceOffsets {
		let mut counts = [[0; 2]; 2];
		for face in faces {
			let (additive, double) = get_additive_double(object_textures, face);
			counts[additive as usize][double as usize] += 1;
		}
		let [[opaque_single, opaque_double], [additive_single, additive_double]] = counts;
		let opaque_single_pos = self.face_instances_pos;
		let opaque_double_pos = opaque_single_pos + opaque_single;
		let additive_single_pos = opaque_double_pos + opaque_double;
		let additive_double_pos = additive_single_pos + additive_single;
		let end = additive_double_pos + additive_double;
		let mut positions = [
			[opaque_single_pos, opaque_double_pos],
			[additive_single_pos, additive_double_pos],
		];
		for face_index in 0..faces.len() {
			let face_data = object_data::RoomFace {
				face_index: face_index as u16,
				..face_data
			};
			let face_data = object_data::ObjectData::RoomFace(face_data);
			let object_data_index = self.write_face_object_data(face_data);
			let face_instance = FaceInstance {
				face_array_index,
				face_index: face_index as u16,
				transform_index,
				object_data_index,
			};
			let (additive, double) = get_additive_double(object_textures, &faces[face_index]);
			let pos = &mut positions[additive as usize][double as usize];
			self.face_instances[*pos] = MaybeUninit::new(face_instance);
			*pos += 1;
		}
		self.face_instances_pos = end;
		RoomFaceOffsets {
			opaque_single: opaque_single_pos as u32,
			opaque_double: opaque_double_pos as u32,
			additive_single: additive_single_pos as u32,
			additive_double: additive_double_pos as u32,
			end: end as u32,
		}
	}
	
	pub fn textured_mesh_face_array<const N: usize, O, F, D>(
		&mut self,
		object_textures: &[O],
		transform_index: TransformIndex,
		face_array_index: FaceArrayIndex,
		face_data: D,
		faces: &[F],
	) -> TexturedMeshFaceOffsets where O: ObjectTexture, F: TexturedMeshFace<N>, D: MeshFaceData {
		let opaque_pos = self.face_instances_pos;
		let end = opaque_pos + faces.len();
		let additive_pos = end - 1;
		let mut positions = [opaque_pos, additive_pos];//opaque moves up, additive moves down
		for face_index in 0..faces.len() {
			let face_data = face_data.with_face_index(face_index as u16);
			let object_data_index = self.write_face_object_data(face_data);
			let face_instance = FaceInstance {
				face_array_index,
				face_index: face_index as u16,
				transform_index,
				object_data_index,
			};
			let face = &faces[face_index];
			let object_texture = &object_textures[face.object_texture_index() as usize];
			let additive = object_texture.blend_mode() == tr3::blend_mode::ADD || face.additive();
			let pos = &mut positions[additive as usize];
			self.face_instances[*pos] = MaybeUninit::new(face_instance);
			*pos = pos.wrapping_add_signed(1 - 2 * additive as isize);
		}
		self.face_instances_pos = end;
		let [additive_pos, _] = positions;
		TexturedMeshFaceOffsets {
			opaque: opaque_pos as u32,
			additive: additive_pos as u32,
			end: end as u32,
		}
	}
	
	pub fn solid_face_array<F, D: MeshFaceData>(
		&mut self,
		transform_index: TransformIndex,
		face_array_index: FaceArrayIndex,
		face_data: D,
		faces: &[F],
	) -> Range<u32> {
		for face_index in 0..faces.len() {
			let face_data = face_data.with_face_index(face_index as u16);
			let object_data_index = self.write_face_object_data(face_data);
			let face_instance = FaceInstance {
				face_array_index,
				face_index: face_index as u16,
				transform_index,
				object_data_index,
			};
			self.face_instances[self.face_instances_pos + face_index] = MaybeUninit::new(face_instance);
		}
		let start = self.face_instances_pos;
		let end = start + faces.len();
		self.face_instances_pos = end;
		start as u32..end as u32
	}
	
	pub fn room_sprites<V: RoomVertex>(
		&mut self,
		room_vertices: &[V],
		room_pos: IVec3,
		room_index: u16,
		sprites: &[tr1::Sprite],
	) -> Range<u32> {
		let start = self.sprite_instances.filled();
		for sprite_index in 0..sprites.len() {
			let tr1::Sprite { vertex_index, sprite_texture_index } = sprites[sprite_index];
			let sprite_data = object_data::RoomSprite {
				room_index,
				sprite_index: sprite_index as u16,
			};
			let sprite_data = object_data::ObjectData::RoomSprite(sprite_data);
			let object_data_index = self.write_sprite_object_data(sprite_data);
			let pos = room_vertices[vertex_index as usize].pos().as_ivec3() + room_pos;
			let sprite_instance = SpriteInstance {
				pos,
				sprite_texture_index,
				object_data_index,
			};
			self.sprite_instances.push(sprite_instance);
		}
		let end = self.sprite_instances.filled();
		start as u32..end as u32
	}
	
	pub fn entity_sprites<E: Entity>(
		&mut self,
		entities: &[E],
		sprite_entities: &[SpriteEntity],
	) -> Range<u32> {
		let start = self.sprite_instances.filled();
		for &SpriteEntity { entity_index, sprite_sequence } in sprite_entities {
			let sprite_data = object_data::EntitySprite {
				entity_index: entity_index as u16,
			};
			let sprite_data = object_data::ObjectData::EntitySprite(sprite_data);
			let object_data_index = self.write_sprite_object_data(sprite_data);
			let pos = entities[entity_index].pos();
			let sprite_instance = SpriteInstance {
				pos,
				sprite_texture_index: sprite_sequence.sprite_texture_index,
				object_data_index,
			};
			self.sprite_instances.push(sprite_instance);
		}
		let end = self.sprite_instances.filled();
		start as u32..end as u32
	}
	
	pub fn done(self) -> Instances {
		assert!(self.face_instances_pos == self.face_instances.len());
		assert!(self.object_data_sprites_pos == self.object_data_faces_start);
		assert!(self.object_data_faces_pos == self.object_data.len());
		//Safety: Initialized after assert.
		let (face_instances, object_data) = unsafe {
			(self.face_instances.assume_init(), self.object_data.assume_init())
		};
		let sprite_instances = self.sprite_instances.into_boxed_slice();
		Instances {
			face_instances,
			sprite_instances,
			object_data,
		}
	}
}
