use std::io::{Read, Result};

use glam::IVec3;
use crate::Readable;
use super::{AnimDispatch, Animation, BoxData, Camera, CinematicFrame, Color3, Color4, Entity, EntityComponentSkip, FrameData, Images, LightMap, Mesh, MeshComponentTr123, MeshNodeData, Meshes, Model, ObjectTexture, Room, RoomVertexComponentTr2, SoundDetails, SoundDetailsComponentTr12, SoundSource, SpriteSequence, SpriteTexture, StateChange, StaticMesh, TrVersion, FRAME_SINGLE_ROT_MASK_TR123, PALETTE_SIZE, SOUND_MAP_SIZE};

#[derive(Readable, Clone, Copy)]
pub struct RoomAmbientLight {
	pub brightness: u16,
	#[skip(2)]
	pub mode: u16,
}

#[derive(Readable, Clone, Copy)]
#[skip_after(4)]
pub struct RoomLight {
	/// World coords
	pub pos: IVec3,
	pub brightness: u16,
	#[skip(2)]
	pub fallout: u32,
}

pub struct Tr2;

impl TrVersion for Tr2 {
	const FRAME_SINGLE_ROT_MASK: u16 = FRAME_SINGLE_ROT_MASK_TR123;
}

#[derive(Readable)]
pub struct Level {
	pub version: u32,
	pub palette3: Box<[Color3; PALETTE_SIZE]>,
	pub palette4: Box<[Color4; PALETTE_SIZE]>,
	pub images: Images,
	#[skip(4)]
	#[list(u16)]
	pub rooms: Box<[Room<RoomVertexComponentTr2, RoomAmbientLight, RoomLight, ()>]>,
	#[list(u32)]
	pub floor_data: Box<[u16]>,
	pub meshes: Meshes<Mesh<MeshComponentTr123>>,
	#[list(u32)]
	pub animations: Box<[Animation<()>]>,
	#[list(u32)]
	pub state_changes: Box<[StateChange]>,
	#[list(u32)]
	pub anim_dispatches: Box<[AnimDispatch]>,
	#[list(u32)]
	pub anim_commands: Box<[u16]>,
	pub mesh_node_data: MeshNodeData,
	pub frame_data: FrameData,
	#[list(u32)]
	pub models: Box<[Model]>,
	#[list(u32)]
	pub static_meshes: Box<[StaticMesh]>,
	#[list(u32)]
	pub object_textures: Box<[ObjectTexture<(), ()>]>,
	#[list(u32)]
	pub sprite_textures: Box<[SpriteTexture]>,
	#[list(u32)]
	pub sprite_sequences: Box<[SpriteSequence]>,
	#[list(u32)]
	pub cameras: Box<[Camera]>,
	#[list(u32)]
	pub sound_sources: Box<[SoundSource]>,
	pub box_data: BoxData<u8>,
	#[list(u32)]
	pub animated_textures: Box<[u16]>,
	#[list(u32)]
	pub entities: Box<[Entity<EntityComponentSkip>]>,
	pub light_map: LightMap,
	#[list(u16)]
	pub cinematic_frames: Box<[CinematicFrame]>,
	#[list(u16)]
	pub demo_data: Box<[u8]>,
	pub sound_map: Box<[u16; SOUND_MAP_SIZE]>,
	#[list(u32)]
	pub sound_details: Box<[SoundDetails<SoundDetailsComponentTr12>]>,
	#[list(u32)]
	pub sample_indices: Box<[u32]>,
}

pub fn read_level<R: Read>(reader: &mut R) -> Result<Level> {
	Level::read(reader)
}
