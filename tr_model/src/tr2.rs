use std::{io::{Read, Result}, mem::transmute, ptr::addr_of_mut, slice::Iter};
use bitfield::bitfield;
use glam::{I16Vec3, IVec3, U16Vec3};
use shared::min_max::MinMax;
use tr_readable::{read_boxed_slice_flat, read_val_flat, Readable};
use crate::{
	decl_mesh1, decl_room_geom, get_packed_angles, tr1::{
		AnimDispatch, Animation, Camera, CinematicFrame, Color24Bit, MeshLighting, MeshNode,
		MeshTexturedQuad, MeshTexturedTri, Model, ObjectTexture, Portal, RoomFlags, RoomQuad, RoomTri,
		Sectors, SoundDetails, SoundSource, Sprite, SpriteSequence, SpriteTexture, StateChange, StaticMesh,
		ATLAS_PIXELS, LIGHT_MAP_LEN, PALETTE_LEN,
	}, u16_cursor::U16Cursor, GenBoxData, GenTrBox,
};

pub const ZONE_FACTOR: usize = 10;
pub const SOUND_MAP_LEN: usize = 370;

//model

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Color32Bit {
	pub r: u8,
	pub g: u8,
	pub b: u8,
	pub unused: u8,
}

bitfield! {
	#[repr(C)]
	#[derive(Clone, Copy)]
	pub struct Color16Bit(u16);
	u8;
	pub a, _: 15;
	pub r, _: 14, 10;
	pub g, _: 9, 5;
	pub b, _: 4, 0;
}

#[derive(Clone)]
pub struct Atlases {
	pub atlases_palette: Box<[[u8; ATLAS_PIXELS]]>,
	pub atlases_16bit: Box<[[Color16Bit; ATLAS_PIXELS]]>,
}

impl Readable for Atlases {
	unsafe fn read<R: Read>(reader: &mut R, this: *mut Self) -> Result<()> {
		let num_atlases = read_val_flat::<_, u32>(reader)? as usize;
		read_boxed_slice_flat(reader, addr_of_mut!((*this).atlases_palette), num_atlases)?;
		read_boxed_slice_flat(reader, addr_of_mut!((*this).atlases_16bit), num_atlases)?;
		Ok(())
	}
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Light {
	pub pos: IVec3,
	pub brightness: u16,
	pub unused1: u16,
	pub fade: u32,
	pub unused2: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RoomStaticMesh {
	/// World coords.
	pub pos: IVec3,
	/// Units are 1/65536 of a rotation.
	pub angle: u16,
	pub light: u16,
	pub unused: u16,
	/// Matched to `StaticMesh.id` in `Level.static_meshes`.
	pub static_mesh_id: u16,
}

#[derive(Readable, Clone)]
pub struct Room {
	/// World coord.
	#[flat] pub x: i32,
	/// World coord.
	#[flat] pub z: i32,
	#[flat] pub y_bottom: i32,
	#[flat] pub y_top: i32,
	#[flat] #[list(u32)] pub geom_data: Box<[u16]>,
	#[flat] #[list(u16)] pub portals: Box<[Portal]>,
	#[delegate] pub sectors: Sectors,
	#[flat] pub ambient_light: u16,
	#[flat] pub unused: u16,
	#[flat] pub light_mode: u16,
	#[flat] #[list(u16)] pub lights: Box<[Light]>,
	#[flat] #[list(u16)] pub room_static_meshes: Box<[RoomStaticMesh]>,
	/// Index into `Level.rooms`.
	#[flat] pub alt_room_index: u16,
	#[flat] pub flags: RoomFlags,
}

pub type TrBox = GenTrBox<u8>;

pub type BoxData = GenBoxData<TrBox, ZONE_FACTOR>;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Entity {
	/// Matched to `Model.id` in `Level.models` or `SpriteSequence.id` in `Level.sprite_sequences`.
	pub model_id: u16,
	/// Index into `Level.rooms`.
	pub room_index: u16,
	/// World coords.
	pub pos: IVec3,
	/// Units are 1/65536th of a rotation.
	pub angle: u16,
	/// If max, use mesh light.
	pub brightness1: u16,
	pub brightness2: u16,
	pub flags: u16,
}

#[derive(Readable, Clone)]
pub struct Level {
	#[flat] pub version: u32,
	#[flat] #[boxed] pub palette_24bit: Box<[Color24Bit; PALETTE_LEN]>,
	#[flat] #[boxed] pub palette_32bit: Box<[Color32Bit; PALETTE_LEN]>,
	#[delegate] pub atlases: Atlases,
	#[flat] pub unused: u32,
	#[delegate] #[list(u16)] pub rooms: Box<[Room]>,
	#[flat] #[list(u32)] pub floor_data: Box<[u16]>,
	#[flat] #[list(u32)] pub mesh_data: Box<[u16]>,
	/// Byte offsets into `Level.mesh_data`.
	#[flat] #[list(u32)] pub mesh_offsets: Box<[u32]>,
	#[flat] #[list(u32)] pub animations: Box<[Animation]>,
	#[flat] #[list(u32)] pub state_changes: Box<[StateChange]>,
	#[flat] #[list(u32)] pub anim_dispatches: Box<[AnimDispatch]>,
	#[flat] #[list(u32)] pub anim_commands: Box<[u16]>,
	#[flat] #[list(u32)] pub mesh_node_data: Box<[u32]>,
	#[flat] #[list(u32)] pub frame_data: Box<[u16]>,
	#[flat] #[list(u32)] pub models: Box<[Model]>,
	#[flat] #[list(u32)] pub static_meshes: Box<[StaticMesh]>,
	#[flat] #[list(u32)] pub object_textures: Box<[ObjectTexture]>,
	#[flat] #[list(u32)] pub sprite_textures: Box<[SpriteTexture]>,
	#[flat] #[list(u32)] pub sprite_sequences: Box<[SpriteSequence]>,
	#[flat] #[list(u32)] pub cameras: Box<[Camera]>,
	#[flat] #[list(u32)] pub sound_sources: Box<[SoundSource]>,
	#[delegate] pub box_data: BoxData,
	#[flat] #[list(u32)] pub animated_textures: Box<[u16]>,
	#[flat] #[list(u32)] pub entities: Box<[Entity]>,
	#[flat] #[boxed] pub light_map: Box<[[u8; PALETTE_LEN]; LIGHT_MAP_LEN]>,
	#[flat] #[list(u16)] pub cinematic_frames: Box<[CinematicFrame]>,
	#[flat] #[list(u16)] pub demo_data: Box<[u8]>,
	#[flat] #[boxed] pub sound_map: Box<[u16; SOUND_MAP_LEN]>,
	#[flat] #[list(u32)] pub sound_details: Box<[SoundDetails]>,
	#[flat] #[list(u32)] pub sample_indices: Box<[u32]>,
}

//extraction

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RoomVertex {
	/// Relative to room
	pub pos: I16Vec3,
	pub unused: u16,
	pub attrs: u16,
	pub light: u16,
}

decl_room_geom!(RoomGeom, RoomVertex, RoomQuad, RoomTri, Sprite);

impl Room {
	pub fn get_geom(&self) -> RoomGeom {
		RoomGeom::get(&self.geom_data)
	}
}

macro_rules! decl_solid_face_type {
	($name:ident, $num_indices:literal) => {
		#[repr(C)]
		#[derive(Clone, Copy)]
		pub struct $name {
			pub vertex_indices: [u16; $num_indices],
			pub color_index_24bit: u8,
			pub color_index_32bit: u8,
		}
	};
}

decl_solid_face_type!(MeshSolidQuad, 4);
decl_solid_face_type!(MeshSolidTri, 3);

decl_mesh1!(Mesh, MeshLighting, MeshTexturedQuad, MeshTexturedTri, MeshSolidQuad, MeshSolidTri);

#[repr(C)]
pub struct FrameData {
	pub bound_box: MinMax<I16Vec3>,
	pub offset: I16Vec3,
	pub rotation_data: [u16],
}

pub struct Frame<'a> {
	pub num_meshes: usize,
	pub frame_data: &'a FrameData,
}

pub struct RotationIterator<'a> {
	rotation_data: Iter<'a, u16>,
	remaining: usize,
}

pub enum Axis { X, Y, Z }

pub enum FrameRotation {
	AllAxes(U16Vec3),
	SingleAxis(Axis, u16),
}

impl Iterator for RotationIterator<'_> {
	type Item = FrameRotation;
	
	fn next(&mut self) -> Option<Self::Item> {
		if self.remaining == 0 {
			return None;
		}
		self.remaining -= 1;
		let word1 = *self.rotation_data.next().unwrap();
		let rotation = match word1 >> 14 {
			0 => {
				let word2 = *self.rotation_data.next().unwrap();
				let angles = get_packed_angles(word1, word2);
				FrameRotation::AllAxes(angles)
			},
			axis => {
				let axis = match axis {
					1 => Axis::X,
					2 => Axis::Y,
					_ => Axis::Z,//only 3 possible
				};
				let angle = word1 & 1023;
				FrameRotation::SingleAxis(axis, angle)
			},
		};
		Some(rotation)
	}
}

impl<'a> Frame<'a> {
	pub(crate) fn get(frame_data: &'a [u16], model: &Model) -> Frame<'a> {
		let frame_data = &frame_data[model.frame_byte_offset as usize / 2..];
		let ptr = frame_data[..9].as_ptr() as usize;
		let frame_data = unsafe { transmute([ptr, frame_data.len() - 9]) };
		Frame {
			num_meshes: model.num_meshes as usize,
			frame_data,
		}
	}
	
	pub fn iter_rotations(&self) -> RotationIterator {
		RotationIterator {
			rotation_data: self.frame_data.rotation_data.iter(),
			remaining: self.num_meshes,
		}
	}
}

impl Level {
	pub fn get_mesh(&self, mesh_offset: u32) -> Mesh {
		Mesh::get(&self.mesh_data, mesh_offset)
	}
	
	pub fn get_mesh_nodes(&self, model: &Model) -> &[MeshNode] {
		MeshNode::get(&self.mesh_node_data, model)
	}
	
	pub fn get_frame(&self, model: &Model) -> Frame {
		Frame::get(&self.frame_data, model)
	}
}
