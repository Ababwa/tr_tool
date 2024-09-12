use std::io::{Read, Result};
use byteorder::ReadBytesExt;
use glam::{I16Vec3, IVec3};
use tr_readable::Readable;
use super::{
	generic::{Animation, Entity, Meshes, ObjectTexture, Room, SoundDetails, TrVersion},
	shared::{
		AnimDispatch, BoxDataTr234, Camera, CinematicFrame, Color3, Color4, EntityComponentSkip,
		FrameData, ImagesTr23, LightMap, MeshComponentTr123, MeshNodeData, Model,
		RoomVertexLightTr34, SoundDetailsComponentTr345, SoundSource, SpriteSequence,
		SpriteTexture, StateChange, StaticMesh, PALETTE_SIZE, SOUND_MAP_SIZE_TR234,
	},
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
	type AnimationComponent = ();
	type EntityComponent = EntityComponentSkip;
	type MeshComponent = MeshComponentTr123;
	type ObjectTextureComponent = ();
	type ObjectTextureDetails = ();
	type RoomAmbientLight = RoomAmbientLight;
	type RoomExtra = RoomExtra;
	type RoomLight = RoomLight;
	type RoomVertexLight = RoomVertexLightTr34;
	type SoundDetailsComponent = SoundDetailsComponentTr345;
}

#[derive(Readable)]
pub struct Level {
	pub version: u32,
	pub palette3: Box<[Color3; PALETTE_SIZE]>,
	pub palette4: Box<[Color4; PALETTE_SIZE]>,
	pub images: ImagesTr23,
	#[skip(4)]
	#[list(u16)]
	pub rooms: Box<[Room<Tr3>]>,
	#[list(u32)]
	pub floor_data: Box<[u16]>,
	pub meshes: Meshes<Tr3>,
	#[list(u32)]
	pub animations: Box<[Animation<Tr3>]>,
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
	pub box_data: BoxDataTr234,
	#[list(u32)]
	pub animated_textures: Box<[u16]>,
	#[list(u32)]
	pub object_textures: Box<[ObjectTexture<Tr3>]>,
	#[list(u32)]
	pub entities: Box<[Entity<Tr3>]>,
	pub light_map: LightMap,
	#[list(u16)]
	pub cinematic_frames: Box<[CinematicFrame]>,
	#[list(u16)]
	pub demo_data: Box<[u8]>,
	pub sound_map: Box<[u16; SOUND_MAP_SIZE_TR234]>,
	#[list(u32)]
	pub sound_details: Box<[SoundDetails<Tr3>]>,
	#[list(u32)]
	pub sample_indices: Box<[u32]>,
}

pub fn read_level<R: Read>(reader: &mut R) -> Result<Level> {
	Level::read(reader)
}
