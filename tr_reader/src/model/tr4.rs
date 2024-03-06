use std::io::{Read, Result};
use bitfield::bitfield;
use byteorder::{ReadBytesExt, LE};
use glam::{I16Vec3, IVec3, U16Vec2, Vec3};
use crate::{get_zlib, read_boxed_slice_raw, Readable};
use super::{
	AnimDispatch, BlendMode, BoxData, Camera, Color3, Entity, Face, FrameData, MeshComponent, MeshNodeData, Meshes, Model, ObjectTextureAtlasAndTriangle, ObjectTextureDetails, Room, SoundDetails, SoundSource, SpriteSequence, SpriteTexture, StateChange, StaticMesh, TrVersion, NUM_PIXELS, SOUND_MAP_SIZE
};

pub struct Tr4;

impl TrVersion for Tr4 {
	const FRAME_SINGLE_ROT_MASK: u16 = 4095;
}

// 1 sector unit = 1024 world coord units

pub struct Images {
	pub num_room_images: u16,
	pub num_obj_images: u16,
	pub num_bump_maps: u16,
	pub images32: Box<[[u32; NUM_PIXELS]]>,
	pub images16: Box<[[u16; NUM_PIXELS]]>,
	pub misc_images: Box<[[u32; NUM_PIXELS]; 2]>,
}

impl Readable for Images {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		let num_room_images = reader.read_u16::<LE>()?;
		let num_obj_images = reader.read_u16::<LE>()?;
		let num_bump_maps = reader.read_u16::<LE>()?;
		let num_images = (num_room_images + num_obj_images + num_bump_maps) as usize;
		let images32 = unsafe { read_boxed_slice_raw(&mut get_zlib(reader)?, num_images)? };//arrays of primitives
		let images16 = unsafe { read_boxed_slice_raw(&mut get_zlib(reader)?, num_images)? };
		let misc_images = unsafe { read_boxed_slice_raw(&mut get_zlib(reader)?, 2)?.try_into().ok().unwrap() };//exactly 2
		Ok(Images {
			num_room_images,
			num_obj_images,
			num_bump_maps,
			images32,
			images16,
			misc_images,
		})
	}
}

#[derive(Readable, Clone, Copy)]
pub struct Light {
	/// World coords
	pub pos: IVec3,
	pub color: Color3,
	pub light_type: u8,
	#[skip_1]
	pub intensity: u8,
	pub hotspot: f32,
	pub falloff: f32,
	pub length: f32,
	pub cutoff: f32,
	pub direction: Vec3,
}

bitfield! {
	#[derive(Readable, Clone, Copy)]
	pub struct MeshEffects(u16);
	pub additive, _: 0;
	pub shiny, _: 1;
	pub shine_strength, _: 7, 2;
}

#[derive(Readable, Clone, Copy)]
pub struct MeshFace<const N: usize> {
	/// Vertex_ids id into Mesh.vertices
	pub face: Face<N>,
	pub effects: MeshEffects,
}

#[derive(Readable)]
pub struct Mesh {
	pub center: I16Vec3,
	pub radius: i32,
	/// Relative to RoomStaticMesh.pos if static mesh
	#[list_u16]
	pub vertices: Box<[I16Vec3]>,
	pub component: MeshComponent,
	#[list_u16]
	pub quads: Box<[MeshFace<4>]>,
	#[list_u16]
	pub tris: Box<[MeshFace<3>]>,
}

#[derive(Readable, Clone, Copy)]
pub struct Anim {
	/// Byte offset into LevelData.frame_data
	pub frame_byte_offset: u32,
	/// 30ths of a second
	pub frame_duration: u8,
	pub num_frames: u8,
	pub state: u16,
	/// Fixed-point
	pub speed: u32,
	/// Fixed-point
	pub accel: u32,
	/// Fixed-point
	pub lateral_speed: u32,
	/// Fixed-point
	pub lateral_accel: u32,
	pub frame_start: u16,
	pub frame_end: u16,
	pub next_anim: u16,
	pub next_frame: u16,
	pub num_state_changes: u16,
	/// Id? into LevelData.state_changes
	pub state_change_id: u16,
	pub num_anim_commands: u16,
	/// Id? into LevelData.anim_commands
	pub anim_command_id: u16,
}

#[derive(Readable, Clone, Copy)]
pub struct FlybyCamera {
	/// World coords
	pub pos: IVec3,
	pub direction: IVec3,
	pub chain: u8,
	pub index: u8,
	pub fov: u16,
	pub roll: i16,
	pub timer: u16,
	pub speed: u16,
	pub flags: u16,
	/// Index into LevelData.rooms
	pub room_index: u32,
}

#[derive(Readable, Clone, Copy)]
pub struct ObjectTexture {
	pub blend_mode: BlendMode,
	pub atlas_and_triangle: ObjectTextureAtlasAndTriangle,
	pub details: ObjectTextureDetails,
	/// Units are 1/256th of a pixel
	pub vertices: [U16Vec2; 4],
	#[skip_8]
	pub width: u32,
	pub height: u32,
}

#[derive(Readable, Clone, Copy)]
pub struct Ai {
	/// Id into Level.models
	pub model_id: u16,
	/// Index into LevelData.rooms
	pub room_index: u16,
	/// World coords
	pub pos: IVec3,
	pub ocb: u16,
	pub flags: u16,
	pub angle: i32,
}

#[derive(Readable)]
pub struct LevelData {
	#[skip_4]
	#[list_u16]
	pub rooms: Box<[Room<Light>]>,
	#[list_u32]
	pub floor_data: Box<[u16]>,
	pub meshes: Meshes<Mesh>,
	#[list_u32]
	pub animations: Box<[Anim]>,
	#[list_u32]
	pub state_changes: Box<[StateChange]>,
	#[list_u32]
	pub anim_dispatches: Box<[AnimDispatch]>,
	#[list_u32]
	pub anim_commands: Box<[u16]>,
	pub mesh_node_data: MeshNodeData,
	pub frame_data: FrameData,
	#[list_u32]
	pub models: Box<[Model]>,
	#[list_u32]
	pub static_meshes: Box<[StaticMesh]>,
	pub spr: [u8; 3],
	#[list_u32]
	pub sprite_textures: Box<[SpriteTexture]>,
	#[list_u32]
	pub sprite_sequences: Box<[SpriteSequence]>,
	#[list_u32]
	pub cameras: Box<[Camera]>,
	#[list_u32]
	pub flyby_cameras: Box<[FlybyCamera]>,
	#[list_u32]
	pub sound_sources: Box<[SoundSource]>,
	pub box_data: BoxData,
	#[list_u32]
	pub animated_textures: Box<[u16]>,
	pub animated_textures_uv_count: u8,
	pub tex: [u8; 3],
	#[list_u32]
	pub object_textures: Box<[ObjectTexture]>,
	#[list_u32]
	pub entities: Box<[Entity]>,
	#[list_u32]
	pub ais: Box<[Ai]>,
	#[list_u16]
	pub demo_data: Box<[u8]>,
	pub sound_map: Box<[u16; SOUND_MAP_SIZE]>,
	#[list_u32]
	pub sound_details: Box<[SoundDetails]>,
	#[list_u32]
	pub sample_indices: Box<[u32]>,
	pub zero: [u8; 6],
}

#[derive(Readable)]
pub struct Sample {
	pub uncompressed: u32,
	#[list_u32]
	pub data: Box<[u8]>,
}

#[derive(Readable)]
pub struct Level {
	pub version: u32,
	pub images: Images,
	#[zlib]
	pub level_data: LevelData,
	#[list_u32]
	pub samples: Box<[Sample]>,
}

pub fn read_level<R: Read>(reader: &mut R) -> Result<Level> {
	Level::read(reader)
}
