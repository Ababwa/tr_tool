use std::{marker::PhantomData, mem::size_of};
use glam::{I16Vec3, Mat4};
use tr_model::tr1;
use crate::{as_bytes::{AsBytes, ReinterpretAsBytes}, multi_cursor::MultiCursorBuffer};

//1 MB
pub const DATA_SIZE: usize = 1048576;

//0..524288, len: 524288 (1/2 buffer)
const GEOM_CURSOR: usize = 0;
const GEOM_OFFSET: usize = 0;

//524288..786432, len: 262144 (1/4 buffer)
const TRANSFORM_CURSOR: usize = 1;
const TRANSFORM_OFFSET: usize = 524288;

//786432..1048576, len: 262144 (1/4 buffer)
const INDEX_CURSOR: usize = 2;
const INDEX_OFFSET: usize = 786432;

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
		Self { mc: MultiCursorBuffer::new(DATA_SIZE, &[GEOM_OFFSET, TRANSFORM_OFFSET, INDEX_OFFSET]) }
	}
	
	pub fn write_object_textures(&mut self, object_textures: &[tr1::ObjectTexture]) {
		self.mc.get_writer(GEOM_CURSOR).write(object_textures.as_bytes());
	}
	
	pub fn write_vertex_array<V>(&mut self, vertices: &[V]) -> u32
	where V: Vertex + ReinterpretAsBytes {
		let mut geom_writer = self.mc.get_writer(GEOM_CURSOR);
		let offset = geom_writer.range().end as u32 / 2;
		geom_writer.write(&(size_of::<V>() as u16 / 2).to_le_bytes());
		geom_writer.write(vertices.as_bytes());
		offset
	}
	
	pub fn write_face_array<F>(&mut self, faces: &[F], vertex_array_offset: u32) -> FaceArrayRef<F>
	where F: Face + ReinterpretAsBytes {
		let mut geom_writer = self.mc.get_writer(GEOM_CURSOR);
		let offset = geom_writer.range().end as u32 / 2;
		geom_writer.write(&vertex_array_offset.to_le_bytes());
		geom_writer.write(&(size_of::<F>() as u16 / 2).to_le_bytes());
		geom_writer.write(faces.as_bytes());
		let mut index_writer = self.mc.get_writer(INDEX_CURSOR);
		let index = index_writer.slice().len() as u32 / 4;
		index_writer.write(&offset.to_le_bytes());
		FaceArrayRef { index, len: faces.len() as u32, u: PhantomData::default() }
	}
	
	pub fn write_transform(&mut self, transform: &Mat4) -> u32 {
		let mut transform_writer = self.mc.get_writer(TRANSFORM_CURSOR);
		let index = (transform_writer.slice().len() / size_of::<Mat4>()) as u32;
		transform_writer.write(transform.as_bytes());
		index
	}
	
	pub fn into_buffer(mut self) -> Box<[u8]> {
		println!("geom bytes: {}", self.mc.get_writer(GEOM_CURSOR).slice().len());
		println!("transform bytes: {}", self.mc.get_writer(TRANSFORM_CURSOR).slice().len());
		println!("index bytes: {}", self.mc.get_writer(INDEX_CURSOR).slice().len());
		self.mc.into_buffer()
	}
}
