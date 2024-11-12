use std::{marker::PhantomData, mem::size_of};
use glam::{I16Vec3, Mat4};
use tr_model::tr1;
use crate::{
	as_bytes::{AsBytes, ReinterpretAsBytes}, multi_cursor::MultiCursorBuffer, version_traits::{Level, Mesh, Room, RoomGeom}
};

//2 MB
pub const DATA_SIZE: usize = 2097152;

//0..1048576, len: 1048576 (1/2 buffer)
const GEOM_CURSOR: usize = 0;
const GEOM_OFFSET: usize = 0;

//1048576..1310720, len: 262144 (1/8 buffer)
const OBJECT_TEXTURES_CURSOR: usize = 1;
const OBJECT_TEXTURES_OFFSET: usize = 1048576;

//1310720..1572864, len: 262144 (1/8 buffer)
const TRANSFORMS_CURSOR: usize = 2;
const TRANSFORMS_OFFSET: usize = 1310720;

//1572864..1835008, len: 262144 (1/8 buffer)
const SPRITE_TEXTURES_CURSOR: usize = 3;
const SPRITE_TEXTURES_OFFSET: usize = 1572864;

//1835008..2097152, len: 262144 (1/8 buffer)
const FACE_ARRAY_MAP_CURSOR: usize = 4;
const FACE_ARRAY_MAP_OFFSET: usize = 1835008;

//type constraints, not strictly necessary
mod private {
	pub trait Vertex<L, D> {}
	pub trait Face<L, D> {}
}

use private::{Vertex, Face};

impl<L: Level> Vertex<L, [(); 0]> for I16Vec3 {}
impl<L: Level> Vertex<L, [(); 1]> for <<L::Room as Room>::RoomGeom<'_> as RoomGeom>::RoomVertex {}

impl<L: Level> Face<L, [(); 0]> for tr1::RoomQuad {}
impl<L: Level> Face<L, [(); 0]> for tr1::RoomTri {}
impl<L: Level> Face<L, [(); 0]> for tr1::MeshTexturedQuad {}
impl<L: Level> Face<L, [(); 0]> for tr1::MeshTexturedTri {}
impl<L: Level> Face<L, [(); 1]> for <L::Mesh<'_> as Mesh>::SolidQuad {}
impl<L: Level> Face<L, [(); 2]> for <L::Mesh<'_> as Mesh>::SolidTri {}

#[derive(Clone, Copy)]
pub struct FaceArrayRef<FaceType> {
	pub index: u32,
	pub len: u32,
	u: PhantomData<FaceType>,
}

pub struct DataWriter<LevelType> {
	mc: MultiCursorBuffer,
	u: PhantomData<LevelType>,
}

impl<L> DataWriter<L> {
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
			u: PhantomData,
		}
	}
	
	pub fn write_object_textures(&mut self, object_textures: &[tr1::ObjectTexture]) {
		self.mc.get_writer(OBJECT_TEXTURES_CURSOR).write(object_textures.as_bytes()).unwrap();
	}
	
	pub fn write_sprite_textures(&mut self, sprite_textures: &[tr1::SpriteTexture]) {
		self.mc.get_writer(SPRITE_TEXTURES_CURSOR).write(sprite_textures.as_bytes()).unwrap();
	}
	
	pub fn write_vertex_array<V, D>(&mut self, vertices: &[V]) -> u32
	where V: Vertex<L, D> + ReinterpretAsBytes {
		let mut geom_writer = self.mc.get_writer(GEOM_CURSOR);
		let offset = geom_writer.get_pos() as u32 / 2;
		geom_writer.write(&(size_of::<V>() as u16 / 2).to_le_bytes()).unwrap();
		geom_writer.write(vertices.as_bytes()).unwrap();
		offset
	}
	
	pub fn write_face_array<F, D>(&mut self, faces: &[F], vertex_array_offset: u32) -> FaceArrayRef<F>
	where F: Face<L, D> + ReinterpretAsBytes {
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
