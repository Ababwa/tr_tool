use std::{mem::transmute, slice::Iter};
use bitfield::bitfield;
use glam::{I16Vec3, IVec3, U16Vec2, U16Vec3};
use shared::min_max::MinMax;
use tr_readable::Readable;
use crate::{
	get_room_geom, tr1::{
		decl_mesh, get_packed_angles, AnimDispatch, Animation, Camera, CinematicFrame, Color24Bit,
		MeshLighting, MeshNode, MeshTexturedQuad, MeshTexturedTri, Model, ObjectTexture, Portal, RoomFlags,
		RoomQuad, RoomTri, Sector, SoundDetails, SoundSource, Sprite, SpriteSequence, SpriteTexture,
		StateChange, StaticMesh, ATLAS_PIXELS, LIGHT_MAP_LEN, PALETTE_LEN,
	}, u16_cursor::U16Cursor
};

pub const SOUND_MAP_LEN: usize = 370;

//model

#[repr(C, align(4))]
#[derive(Clone, Debug)]
pub struct Color32BitRgb {
	pub r: u8,
	pub g: u8,
	pub b: u8,
}

bitfield! {
	#[repr(C)]
	#[derive(Clone, Debug)]
	pub struct Color16BitArgb(u16);
	u8;
	pub a, _: 15;
	pub r, _: 14, 10;
	pub g, _: 9, 5;
	pub b, _: 4, 0;
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Light {
	pub pos: IVec3,
	pub brightness: u16,
	pub unused1: u16,
	pub fade: u32,
	pub unused2: u32,
}

#[repr(C)]
#[derive(Clone, Debug)]
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

#[derive(Readable, Clone, Debug)]
pub struct Room {
	/// World coord.
	pub x: i32,
	/// World coord.
	pub z: i32,
	pub y_bottom: i32,
	pub y_top: i32,
	#[list(u32)] pub geom_data: Box<[u16]>,
	#[list(u16)] pub portals: Box<[Portal]>,
	pub sectors_size: U16Vec2,
	#[list(sectors_size)] pub sectors: Box<[Sector]>,
	pub ambient_light: u16,
	pub unused: u16,
	pub light_mode: u16,
	#[list(u16)] pub lights: Box<[Light]>,
	#[list(u16)] pub room_static_meshes: Box<[RoomStaticMesh]>,
	/// Index into `Level.rooms`.
	pub flip_room_index: u16,
	pub flags: RoomFlags,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct TrBox {
	pub z: MinMax<u8>,
	pub x: MinMax<u8>,
	pub y: i16,
	pub overlap: u16,
}

#[repr(C)]
#[derive(Clone, Debug)]
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

#[derive(Readable, Clone, Debug)]
pub struct Level {
	pub version: u32,
	#[boxed] pub palette_24bit: Box<[Color24Bit; PALETTE_LEN]>,
	#[boxed] pub palette_32bit: Box<[Color32BitRgb; PALETTE_LEN]>,
	#[list(u32)] pub atlases_palette: Box<[[u8; ATLAS_PIXELS]]>,
	#[list(atlases_palette)] pub atlases_16bit: Box<[[Color16BitArgb; ATLAS_PIXELS]]>,
	pub unused: u32,
	#[list(u16)] #[delegate] pub rooms: Box<[Room]>,
	#[list(u32)] pub floor_data: Box<[u16]>,
	#[list(u32)] pub mesh_data: Box<[u16]>,
	/// Byte offsets into `Level.mesh_data`.
	#[list(u32)] pub mesh_offsets: Box<[u32]>,
	#[list(u32)] pub animations: Box<[Animation]>,
	#[list(u32)] pub state_changes: Box<[StateChange]>,
	#[list(u32)] pub anim_dispatches: Box<[AnimDispatch]>,
	#[list(u32)] pub anim_commands: Box<[u16]>,
	#[list(u32)] pub mesh_node_data: Box<[u32]>,
	#[list(u32)] pub frame_data: Box<[u16]>,
	#[list(u32)] pub models: Box<[Model]>,
	#[list(u32)] pub static_meshes: Box<[StaticMesh]>,
	#[list(u32)] pub object_textures: Box<[ObjectTexture]>,
	#[list(u32)] pub sprite_textures: Box<[SpriteTexture]>,
	#[list(u32)] pub sprite_sequences: Box<[SpriteSequence]>,
	#[list(u32)] pub cameras: Box<[Camera]>,
	#[list(u32)] pub sound_sources: Box<[SoundSource]>,
	#[list(u32)] pub boxes: Box<[TrBox]>,
	#[list(u32)] pub overlap_data: Box<[u16]>,
	#[list(boxes)] pub zone_data: Box<[[u16; 10]]>,
	#[list(u32)] pub animated_textures: Box<[u16]>,
	#[list(u32)] pub entities: Box<[Entity]>,
	#[boxed] pub light_map: Box<[[u8; PALETTE_LEN]; LIGHT_MAP_LEN]>,
	#[list(u16)] pub cinematic_frames: Box<[CinematicFrame]>,
	#[list(u16)] pub demo_data: Box<[u8]>,
	#[boxed] pub sound_map: Box<[u16; SOUND_MAP_LEN]>,
	#[list(u32)] pub sound_details: Box<[SoundDetails]>,
	#[list(u32)] pub sample_indices: Box<[u32]>,
}

//extraction

#[repr(C)]
#[derive(Clone, Debug)]
pub struct RoomVertex {
	/// Relative to room
	pub pos: I16Vec3,
	pub unused: u16,
	pub attrs: u16,
	pub light: u16,
}

#[derive(Clone, Debug)]
pub struct RoomGeom<'a> {
	pub vertices: &'a [RoomVertex],
	pub quads: &'a [RoomQuad],
	pub tris: &'a [RoomTri],
	pub sprites: &'a [Sprite],
}

impl Room {
	pub fn get_geom(&self) -> RoomGeom {
		let (vertices, quads, tris, sprites) = unsafe { get_room_geom(&self.geom_data) };
		RoomGeom { vertices, quads, tris, sprites }
	}
}

macro_rules! decl_solid_face_type {
	($name:ident, $num_indices:literal) => {
		#[repr(C)]
		#[derive(Clone, Debug)]
		pub struct $name {
			pub vertex_indices: [u16; $num_indices],
			pub color_index_24bit: u8,
			pub color_index_32bit: u8,
		}
	};
}

decl_solid_face_type!(MeshSolidQuad, 4);
decl_solid_face_type!(MeshSolidTri, 3);

decl_mesh!(Mesh, MeshLighting, MeshTexturedQuad, MeshTexturedTri, MeshSolidQuad, MeshSolidTri);

#[repr(C)]
#[derive(Debug)]
pub struct FrameData {
	pub bound_box: MinMax<I16Vec3>,
	pub offset: I16Vec3,
	pub rotation_data: [u16],
}

#[derive(Clone, Debug)]
pub enum Axis {
	X,
	Y,
	Z,
}

macro_rules! decl_frame {
	($frame:ident, $rotation_iterator:ident, $frame_rotation:ident, $single_angle_mask:literal) => {
		#[derive(Clone, Debug)]
		pub struct $frame<'a> {
			pub num_meshes: usize,
			pub frame_data: &'a FrameData,
		}

		#[derive(Clone, Debug)]
		pub struct $rotation_iterator<'a> {
			rotation_data: Iter<'a, u16>,
			remaining: usize,
		}

		#[derive(Clone, Debug)]
		pub enum $frame_rotation {
			AllAxes(U16Vec3),
			SingleAxis(Axis, u16),
		}

		impl Iterator for $rotation_iterator<'_> {
			type Item = $frame_rotation;
			
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
						Self::Item::AllAxes(angles)
					},
					axis => {
						let axis = match axis {
							1 => Axis::X,
							2 => Axis::Y,
							_ => Axis::Z,//only 3 possible
						};
						let angle = word1 & $single_angle_mask;
						Self::Item::SingleAxis(axis, angle)
					},
				};
				Some(rotation)
			}
		}

		impl<'a> $frame<'a> {
			pub(crate) fn get(frame_data: &'a [u16], model: &Model) -> Self {
				let frame_data = &frame_data[model.frame_byte_offset as usize / 2..];
				let ptr = frame_data[..9].as_ptr() as usize;
				let frame_data = unsafe { transmute([ptr, frame_data.len() - 9]) };
				Self { num_meshes: model.num_meshes as usize, frame_data }
			}
			
			pub fn iter_rotations(&self) -> $rotation_iterator<'a> {
				$rotation_iterator {
					rotation_data: self.frame_data.rotation_data.iter(),
					remaining: self.num_meshes,
				}
			}
		}
	};
}
pub(crate) use decl_frame;

decl_frame!(Frame, RotationIterator, FrameRotation, 0x3FF);

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
