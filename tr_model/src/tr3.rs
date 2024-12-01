use bitfield::bitfield;
use glam::{I16Vec3, IVec3};
use tr_readable::Readable;
use crate::{
	tr1::{
		AnimDispatch, Animation, Camera, CinematicFrame, Color24Bit, MeshNode, Model,
		NumSectors, ObjectTexture, Portal, RoomFlags, Sector, SoundSource, Sprite, SpriteSequence,
		SpriteTexture, StateChange, StaticMesh, ATLAS_PIXELS, LIGHT_MAP_LEN, PALETTE_LEN,
	},
	tr2::{Color16BitArgb, Color32BitRgb, Entity, Frame, Mesh, TrBox, SOUND_MAP_LEN},
};

pub mod blend_mode {
	pub const OPAQUE: u16 = 0;
	pub const TEST: u16 = 1;
	pub const ADD: u16 = 2;
}

pub mod light_type {
	pub const SUN: u8 = 0;
	pub const POINT: u8 = 1;
}

//model

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Light {
	pub pos: IVec3,
	pub color: Color24Bit,
	/// One of the light types in the `light_type` module.
	pub light_type: u8,
	pub light_data: [u32; 2],
}

bitfield! {
	#[repr(C)]
	#[derive(Clone, Debug)]
	pub struct Color16BitRgb(u16);
	u8;
	pub r, _: 14, 10;
	pub g, _: 9, 5;
	pub b, _: 4, 0;
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct RoomStaticMesh {
	/// World coords.
	pub pos: IVec3,
	/// Units are 1/65536 of a rotation.
	pub angle: u16,
	pub color: Color16BitRgb,
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
	pub geom_data_size: u32,
	#[list(u16)] pub vertices: Box<[RoomVertex]>,
	#[list(u16)] pub quads: Box<[RoomQuad]>,
	#[list(u16)] pub tris: Box<[RoomTri]>,
	#[list(u16)] pub sprites: Box<[Sprite]>,
	#[list(u16)] pub portals: Box<[Portal]>,
	pub num_sectors: NumSectors,
	#[list(num_sectors)] pub sectors: Box<[Sector]>,
	pub ambient_light: u16,
	pub unused1: u16,
	#[list(u16)] pub lights: Box<[Light]>,
	#[list(u16)] pub room_static_meshes: Box<[RoomStaticMesh]>,
	/// Index into `Level.rooms`.
	pub flip_room_index: u16,
	pub flags: RoomFlags,
	pub water_details: u8,
	pub reverb: u8,
	pub unused2: u8,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct SoundDetails {
	/// Index into `Level.sample_indices`.
	pub sample_index: u16,
	pub volume: u8,
	pub range: u8,
	pub chance: u8,
	pub pitch: u8,
	pub details: u16,
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
	#[list(u32)] pub sprite_textures: Box<[SpriteTexture]>,
	#[list(u32)] pub sprite_sequences: Box<[SpriteSequence]>,
	#[list(u32)] pub cameras: Box<[Camera]>,
	#[list(u32)] pub sound_sources: Box<[SoundSource]>,
	#[list(u32)] pub boxes: Box<[TrBox]>,
	#[list(u32)] pub overlap_data: Box<[u16]>,
	#[list(boxes)] pub zone_data: Box<[[u16; 10]]>,
	#[list(u32)] pub animated_textures: Box<[u16]>,
	#[list(u32)] pub object_textures: Box<[ObjectTexture]>,
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
	pub color: Color16BitRgb,
}

bitfield! {
	#[repr(C)]
	#[derive(Clone, Debug)]
	pub struct RoomFaceTexture(u16);
	pub double_sided, _: 15;
	pub object_texture_index, _: 14, 0;
}

macro_rules! decl_face_type {
	($name:ident, $num_indices:literal) => {
		#[repr(C)]
		#[derive(Clone, Debug)]
		pub struct $name {
			pub vertex_indices: [u16; $num_indices],
			pub texture: RoomFaceTexture,
		}
	};
}

decl_face_type!(RoomQuad, 4);
decl_face_type!(RoomTri, 3);

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
