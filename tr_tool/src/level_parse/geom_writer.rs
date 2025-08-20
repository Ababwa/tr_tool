use std::ptr;
use glam::Mat4;
use tr_model::tr1;
use crate::{level_parse::counts::Counts, round_up, tr_traits::Face, GEOM_BUFFER_SIZE};

pub const VERTEX_ARRAY_HEADER_SIZE: usize = 4;
pub const FACE_ARRAY_HEADER_SIZE: usize = 8;

/**
Buffer has the following structure:
```
{
	geom: [mixed],
	padding: [u8],
	transforms: [Mat4],
	face_array_offsets: [u32],
	object_textures: [ObjectTexture],
	sprite_textures: [SpriteTexture],
}
```
`geom` contains vertex and face arrays and is 4-aligned.<br>
`padding` pads to 16-align transforms.<br>
`face_array_offsets` contains offsets of face arrays in `geom` in 4-byte units.
*/
#[repr(C, align(16))]
pub struct GeomBuffer([u8; GEOM_BUFFER_SIZE]);

pub struct GeomOutput {
	pub buffer: Box<GeomBuffer>,
	/// Offset of transforms in 16-byte units.
	pub transforms_offset: u32,
	/// Offset of face array offsets in 4-byte units.
	pub face_array_offsets_offset: u32,
	/// Offset of object textures in 2-byte units.
	pub object_textures_offset: u32,
	/// Offset of sprite textures in 2-byte units.
	pub sprite_textures_offset: u32,
}

pub struct GeomWriter {
	buffer: Box<GeomBuffer>,
	geom_end: usize,
	transforms_offset: usize,
	face_array_offsets_offset: usize,
	object_textures_offset: usize,
	sprite_textures_offset: usize,
	geom_pos: usize,
	transforms_pos: usize,
	face_array_offsets_pos: usize,
}

//to avoid mixing
#[repr(C)] #[derive(Clone, Copy)] pub struct VertexArrayOffset(u32);
#[repr(C)] #[derive(Clone, Copy)] pub struct FaceArrayIndex(u16);
#[repr(C)] #[derive(Clone, Copy)] pub struct TransformIndex(u16);

impl GeomWriter {
	pub fn new<O>(object_textures: &[O], sprite_textures: &[tr1::SpriteTexture], counts: &Counts) -> Self {
		let object_textures_size = size_of_val(object_textures);
		let sprite_textures_size = size_of_val(sprite_textures);
		let geom_end = counts.vertex_arrays_size + counts.face_arrays_size;
		let transforms_offset = round_up!(geom_end, 16);
		let face_array_offsets_offset = transforms_offset + counts.transforms * 64;
		let object_textures_offset = face_array_offsets_offset + counts.face_arrays * 4;
		let sprite_textures_offset = object_textures_offset + object_textures_size;
		let total = sprite_textures_offset + sprite_textures_size;
		let ratio = total as f32 / GEOM_BUFFER_SIZE as f32 * 100.0;
		println!("geom buffer: {}/{} ({}%)", total, GEOM_BUFFER_SIZE, ratio);
		assert!(total <= GEOM_BUFFER_SIZE);
		//Safety: Uninitialized bytes are never used. Writes are in bounds after assert.
		let buffer = unsafe {
			let mut buffer = Box::<GeomBuffer>::new_uninit().assume_init();
			let object_textures_ptr = buffer.0.as_mut_ptr().add(object_textures_offset).cast();
			ptr::copy_nonoverlapping(object_textures.as_ptr(), object_textures_ptr, object_textures.len());
			let sprite_textures_ptr = buffer.0.as_mut_ptr().add(sprite_textures_offset).cast();
			ptr::copy_nonoverlapping(sprite_textures.as_ptr(), sprite_textures_ptr, sprite_textures.len());
			buffer
		};
		Self {
			buffer,
			geom_end,
			transforms_offset,
			face_array_offsets_offset,
			object_textures_offset,
			sprite_textures_offset,
			geom_pos: 0,
			transforms_pos: transforms_offset,
			face_array_offsets_pos: face_array_offsets_offset,
		}
	}
	
	/**
	Writes the following record to the geometry buffer 4-aligned:
	```
	{
		vertex_size: u32,
		vertices: [V],
	}
	```
	`vertex_size` is the size of `V` in 2-byte units.<br>
	`V` has alignment 2 or 4.<br>
	Returns the offset of the record in 4-byte units.
	*/
	pub fn vertex_array<V>(&mut self, vertices: &[V]) -> VertexArrayOffset {
		let size = VERTEX_ARRAY_HEADER_SIZE + round_up!(size_of_val(vertices), 4);
		assert!(self.geom_pos + size <= self.transforms_offset);
		//Safety: Writes are in bounds after assert.
		unsafe {
			let ptr = self.buffer.0.as_mut_ptr().add(self.geom_pos);
			*ptr.cast() = (size_of::<V>() / 2) as u32;
			ptr::copy_nonoverlapping(vertices.as_ptr(), ptr.add(4).cast(), vertices.len());
			let offset = (self.geom_pos / 4) as u32;
			self.geom_pos += size;
			VertexArrayOffset(offset)
		}
	}
	
	/**
	Writes the following record to the geometry buffer 4-aligned:
	```
	{
		vertex_array_offset: u32,
		face_size: u16,
		texture_offset: u16,
		faces: [F],
	}
	```
	`vertex_array_offset` is the offset of the vertex array in 4-byte units.<br>
	`face_size` is the size of `F` in 2-byte units.<br>
	`texture_offset` is the offset of texture data in `F` in 2-byte units.<br>
	`F` has alignment 2.<br>
	Writes the offset of the record in 4-byte units to the face array offsets table.<br>
	Returns the index of the offset in the table.
	*/
	pub fn face_array<const N: usize, F: Face<N>>(
		&mut self,
		faces: &[F],
		vertex_array_offset: VertexArrayOffset,
	) -> FaceArrayIndex {
		let size = FACE_ARRAY_HEADER_SIZE + round_up!(size_of_val(faces), 4);
		assert!(self.geom_pos + size <= self.transforms_offset);
		assert!(self.face_array_offsets_pos + 4 <= self.object_textures_offset);
		//Safety: Writes are in bounds after assert.
		unsafe {
			let ptr = self.buffer.0.as_mut_ptr().add(self.geom_pos);
			*ptr.cast() = vertex_array_offset;
			*ptr.add(4).cast() = (size_of::<F>() / 2) as u16;
			*ptr.add(6).cast() = N as u16;
			ptr::copy_nonoverlapping(faces.as_ptr(), ptr.add(8).cast(), faces.len());
			let offset = (self.geom_pos / 4) as u32;
			self.geom_pos += size;
			*self.buffer.0.as_mut_ptr().add(self.face_array_offsets_pos).cast() = offset;
			let index = ((self.face_array_offsets_pos - self.face_array_offsets_offset) / 4) as u16;
			self.face_array_offsets_pos += 4;
			FaceArrayIndex(index)
		}
	}
	
	pub fn transform(&mut self, transform: &Mat4) -> TransformIndex {
		assert!(self.transforms_pos + 64 <= self.face_array_offsets_offset);
		//Safety: Writes are in bounds after assert.
		unsafe {
			let index = ((self.transforms_pos - self.transforms_offset) / 64) as u16;
			*self.buffer.0.as_mut_ptr().add(self.transforms_pos).cast() = *transform;
			self.transforms_pos += 64;
			TransformIndex(index)
		}
	}
	
	pub fn done(self) -> GeomOutput {
		assert!(self.geom_pos == self.geom_end);
		assert!(self.transforms_pos == self.face_array_offsets_offset);
		assert!(self.face_array_offsets_pos == self.object_textures_offset);
		GeomOutput {
			buffer: self.buffer,
			transforms_offset: (self.transforms_offset / 16) as u32,
			face_array_offsets_offset: (self.face_array_offsets_offset / 4) as u32,
			object_textures_offset: (self.object_textures_offset / 2) as u32,
			sprite_textures_offset: (self.sprite_textures_offset / 2) as u32,
		}
	}
}
