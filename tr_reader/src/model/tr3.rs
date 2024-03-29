use std::io::{Read, Result};
use byteorder::ReadBytesExt;
use glam::{I16Vec3, IVec3};
use crate::Readable;
use super::{
	AnimDispatch, Animation, BoxData, Camera, CinematicFrame, Color3, Color4, Entity, EntityComponentSkip, FrameData, Images, LightMap, Mesh, MeshComponentTr123, MeshNodeData, Meshes, Model, ObjectTexture, Room, RoomVertexComponentTr34, SoundDetails, SoundDetailsComponentTr345, SoundSource, SpriteSequence, SpriteTexture, StateChange, StaticMesh, TrVersion, FRAME_SINGLE_ROT_MASK_TR123, PALETTE_SIZE, SOUND_MAP_SIZE
};

#[derive(Readable, Clone, Copy)]
#[skip_after(2)]
pub struct RoomAmbientLight {
	pub brightness: u16,
}

#[derive(Readable, Clone, Copy)]
#[skip_after(2)]
pub struct SunLight {
	pub normal: I16Vec3,
}

#[derive(Readable, Clone, Copy)]
pub struct PointLight {
	pub intensity: u32,
	pub falloff: u32,
}

#[derive(Clone, Copy)]
pub enum LightComponent {
	Sun(SunLight),
	Point(PointLight),
}

#[derive(Clone, Copy)]
pub struct RoomLight {
	/// World coords
	pub pos: IVec3,
	pub color: Color3,
	pub component: LightComponent,
}

impl Readable for RoomLight {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		let pos = IVec3::read(reader)?;
		let color = Color3::read(reader)?;
		let component = match reader.read_u8()? {
			0 => LightComponent::Sun(SunLight::read(reader)?),
			1 => LightComponent::Point(PointLight::read(reader)?),
			a => panic!("unknown light type: {}", a),
		};
		Ok(RoomLight { pos, color, component })
	}
}

#[derive(Readable, Clone, Copy)]
#[skip_after(1)]
pub struct RoomExtra {
	pub water_effect: u8,
	pub reverb: u8,
}

pub struct Tr3;

impl TrVersion for Tr3 {
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
	pub rooms: Box<[Room<RoomVertexComponentTr34, RoomAmbientLight, RoomLight, RoomExtra>]>,
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
	pub object_textures: Box<[ObjectTexture<(), ()>]>,
	#[list(u32)]
	pub entities: Box<[Entity<EntityComponentSkip>]>,
	pub light_map: LightMap,
	#[list(u16)]
	pub cinematic_frames: Box<[CinematicFrame]>,
	#[list(u16)]
	pub demo_data: Box<[u8]>,
	pub sound_map: Box<[u16; SOUND_MAP_SIZE]>,
	#[list(u32)]
	pub sound_details: Box<[SoundDetails<SoundDetailsComponentTr345>]>,
	#[list(u32)]
	pub sample_indices: Box<[u32]>,
}

pub fn read_level<R: Read>(reader: &mut R) -> Result<Level> {
	Level::read(reader)
}
