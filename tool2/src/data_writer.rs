use std::{marker::PhantomData, mem::size_of};
use glam::{I16Vec3, Mat4};
use tr_model::tr1;
use crate::{as_bytes::{AsBytes, ReinterpretAsBytes}, multi_cursor::MultiCursorBuffer};

//1 MB
pub const DATA_SIZE: usize = 1048576;

//0..524288, len: 524288 (1/2 buffer)
const GEOM_CURSOR: usize = 0;
const GEOM_OFFSET: usize = 0;

//524288..655360, len: 131072 (1/8 buffer)
const OBJECT_TEXTURES_CURSOR: usize = 1;
const OBJECT_TEXTURES_OFFSET: usize = 524288;

//655360..786432, len: 131072 (1/8 buffer)
const TRANSFORMS_CURSOR: usize = 2;
const TRANSFORMS_OFFSET: usize = 655360;

//786432..917504, len: 131072 (1/8 buffer)
const SPRITE_TEXTURES_CURSOR: usize = 3;
const SPRITE_TEXTURES_OFFSET: usize = 786432;

//917504..1048576, len: 131072 (1/8 buffer)
const FACE_ARRAY_MAP_CURSOR: usize = 4;
const FACE_ARRAY_MAP_OFFSET: usize = 917504;

mod private {
	pub trait Vertex {}
	pub trait Face {}
}

use private::{Vertex, Face};

impl Vertex for tr1::RoomVertex {}
impl Vertex for I16Vec3 {}

impl Face for tr1::RoomQuad {}
impl Face for tr1::RoomTri {}
impl Face for tr1::MeshTexturedQuad {}
impl Face for tr1::MeshTexturedTri {}
impl Face for tr1::MeshSolidQuad {}
impl Face for tr1::MeshSolidTri {}

#[derive(Clone, Copy)]
pub struct FaceArrayRef<T> {
	pub index: u32,
	pub len: u32,
	u: PhantomData<T>,
}

pub struct DataWriter {
	mc: MultiCursorBuffer,
}

impl DataWriter {
	pub fn new() -> Self {
		Self {
			mc: MultiCursorBuffer::new(
				DATA_SIZE,
				&[
					GEOM_OFFSET,
					OBJECT_TEXTURES_OFFSET,
					TRANSFORMS_OFFSET,
					SPRITE_TEXTURES_OFFSET,
					FACE_ARRAY_MAP_OFFSET,
				],
			),
		}
	}
	
	pub fn write_object_textures(&mut self, object_textures: &[tr1::ObjectTexture]) {
		self.mc.get_writer(OBJECT_TEXTURES_CURSOR).write(object_textures.as_bytes()).unwrap();
	}
	
	pub fn write_sprite_textures(&mut self, sprite_textures: &[tr1::SpriteTexture]) {
		self.mc.get_writer(SPRITE_TEXTURES_CURSOR).write(sprite_textures.as_bytes()).unwrap();
	}
	
	pub fn write_vertex_array<V>(&mut self, vertices: &[V]) -> u32
	where V: Vertex + ReinterpretAsBytes {
		let mut geom_writer = self.mc.get_writer(GEOM_CURSOR);
		let offset = geom_writer.get_pos() as u32 / 2;
		geom_writer.write(&(size_of::<V>() as u16 / 2).to_le_bytes()).unwrap();
		geom_writer.write(vertices.as_bytes()).unwrap();
		offset
	}
	
	pub fn write_face_array<F>(&mut self, faces: &[F], vertex_array_offset: u32) -> FaceArrayRef<F>
	where F: Face + ReinterpretAsBytes {
		let mut geom_writer = self.mc.get_writer(GEOM_CURSOR);
		let offset = geom_writer.get_pos() as u32 / 2;
		geom_writer.write(&vertex_array_offset.to_le_bytes()).unwrap();
		geom_writer.write(&(size_of::<F>() as u16 / 2).to_le_bytes()).unwrap();
		geom_writer.write(faces.as_bytes()).unwrap();
		let mut face_array_map_writer = self.mc.get_writer(FACE_ARRAY_MAP_CURSOR);
		let index = face_array_map_writer.get_size() as u32 / 4;
		face_array_map_writer.write(&offset.to_le_bytes()).unwrap();
		FaceArrayRef { index, len: faces.len() as u32, u: PhantomData }
	}
	
	pub fn write_transform(&mut self, transform: &Mat4) -> u32 {
		let mut transforms_writer = self.mc.get_writer(TRANSFORMS_CURSOR);
		let index = (transforms_writer.get_size() / size_of::<Mat4>()) as u32;
		transforms_writer.write(transform.as_bytes()).unwrap();
		index
	}
	
	pub fn into_buffer(self) -> Box<[u8]> {
		println!("GEOM bytes: {}", self.mc.get_size(GEOM_CURSOR));
		println!("OBJECT_TEXTURES bytes: {}", self.mc.get_size(OBJECT_TEXTURES_CURSOR));
		println!("TRANSFORM bytes: {}", self.mc.get_size(TRANSFORMS_CURSOR));
		println!("SPRITE_TEXTURES bytes: {}", self.mc.get_size(SPRITE_TEXTURES_CURSOR));
		println!("FACE_MAP bytes: {}", self.mc.get_size(FACE_ARRAY_MAP_CURSOR));
		self.mc.into_buffer()
	}
}
