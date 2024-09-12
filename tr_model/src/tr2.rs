use std::io::{Read, Result};
use glam::IVec3;
use tr_readable::Readable;
use super::{
	generic::{Animation, Entity, Meshes, ObjectTexture, Room, SoundDetails, TrVersion},
	shared::{
		AnimDispatch, BoxDataTr234, Camera, CinematicFrame, Color3, Color4, EntityComponentSkip,
		FrameData, ImagesTr23, LightMap, MeshComponentTr123, MeshNodeData, Model,
		SoundDetailsComponentTr12, SoundSource, SpriteSequence, SpriteTexture, StateChange,
		StaticMesh, PALETTE_SIZE, SOUND_MAP_SIZE_TR234,
	},
};

#[derive(Readable, Clone, Copy)]
pub struct RoomVertexLight {
	#[skip(2)]
	pub flags: u16,
	pub brightness: u16,
}

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
	type AnimationComponent = ();
	type EntityComponent = EntityComponentSkip;
	type MeshComponent = MeshComponentTr123;
	type ObjectTextureComponent = ();
	type ObjectTextureDetails = ();
	type RoomAmbientLight = RoomAmbientLight;
	type RoomExtra = ();
	type RoomLight = RoomLight;
	type RoomVertexLight = RoomVertexLight;
	type SoundDetailsComponent = SoundDetailsComponentTr12;
}

#[derive(Readable)]
pub struct Level {
	pub version: u32,
	pub palette3: Box<[Color3; PALETTE_SIZE]>,
	pub palette4: Box<[Color4; PALETTE_SIZE]>,
	pub images: ImagesTr23,
	#[skip(4)]
	#[list(u16)]
	pub rooms: Box<[Room<Tr2>]>,
	#[list(u32)]
	pub floor_data: Box<[u16]>,
	pub meshes: Meshes<Tr2>,
	#[list(u32)]
	pub animations: Box<[Animation<Tr2>]>,
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
	pub object_textures: Box<[ObjectTexture<Tr2>]>,
	#[list(u32)]
	pub sprite_textures: Box<[SpriteTexture]>,
	#[list(u32)]
	pub sprite_sequences: Box<[SpriteSequence]>,
	#[list(u32)]
	pub cameras: Box<[Camera]>,
	#[list(u32)]
	pub sound_sources: Box<[SoundSource]>,
	pub box_data: BoxDataTr234,
	#[list(u32)]
	pub animated_textures: Box<[u16]>,
	#[list(u32)]
	pub entities: Box<[Entity<Tr2>]>,
	pub light_map: LightMap,
	#[list(u16)]
	pub cinematic_frames: Box<[CinematicFrame]>,
	#[list(u16)]
	pub demo_data: Box<[u8]>,
	pub sound_map: Box<[u16; SOUND_MAP_SIZE_TR234]>,
	#[list(u32)]
	pub sound_details: Box<[SoundDetails<Tr2>]>,
	#[list(u32)]
	pub sample_indices: Box<[u32]>,
}

pub fn read_level<R: Read>(reader: &mut R) -> Result<Level> {
	Level::read(reader)
}
