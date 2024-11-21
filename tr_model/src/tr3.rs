use bitfield::bitfield;
use glam::{I16Vec3, IVec3};
use tr_readable::Readable;
use crate::{
	decl_room_geom, tr1::{
		AnimDispatch, Animation, Camera, CinematicFrame, Color24Bit, MeshNode, Model, ObjectTexture, Portal,
		RoomFlags, Sectors, SoundSource, Sprite, SpriteSequence, SpriteTexture, StateChange, StaticMesh,
		LIGHT_MAP_LEN, PALETTE_LEN,
	}, tr2::{Atlases, BoxData, Color16Bit, Color32Bit, Entity, Frame, Mesh, SOUND_MAP_LEN},
	u16_cursor::U16Cursor,
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

#[repr(C)]
#[derive(Clone, Debug)]
pub struct RoomStaticMesh {
	/// World coords.
	pub pos: IVec3,
	/// Units are 1/65536 of a rotation.
	pub angle: u16,
	pub color: Color16Bit,
	pub unused: u16,
	/// Matched to `StaticMesh.id` in `Level.static_meshes`.
	pub static_mesh_id: u16,
}

#[derive(Readable, Clone, Debug)]
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
	#[flat] pub unused1: u16,
	#[flat] #[list(u16)] pub lights: Box<[Light]>,
	#[flat] #[list(u16)] pub room_static_meshes: Box<[RoomStaticMesh]>,
	/// Index into `Level.rooms`.
	#[flat] pub alt_room_index: u16,
	#[flat] pub flags: RoomFlags,
	#[flat] pub water_details: u8,
	#[flat] pub reverb: u8,
	#[flat] pub unused2: u8,
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
	#[flat] #[list(u32)] pub sprite_textures: Box<[SpriteTexture]>,
	#[flat] #[list(u32)] pub sprite_sequences: Box<[SpriteSequence]>,
	#[flat] #[list(u32)] pub cameras: Box<[Camera]>,
	#[flat] #[list(u32)] pub sound_sources: Box<[SoundSource]>,
	#[delegate] pub box_data: BoxData,
	#[flat] #[list(u32)] pub animated_textures: Box<[u16]>,
	#[flat] #[list(u32)] pub object_textures: Box<[ObjectTexture]>,
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
#[derive(Clone, Debug)]
pub struct RoomVertex {
	/// Relative to room
	pub pos: I16Vec3,
	pub unused: u16,
	pub attrs: u16,
	pub color: Color16Bit,
}

bitfield! {
	#[repr(C)]
	#[derive(Clone, Debug)]
	pub struct RoomFaceTexture(u16);
	pub object_texture_index, _: 0, 14;
	pub double_sided, _: 15;
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

decl_room_geom!(RoomGeom, RoomVertex, RoomQuad, RoomTri, Sprite);

impl Room {
	pub fn get_geom(&self) -> RoomGeom {
		RoomGeom::get(&self.geom_data)
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
