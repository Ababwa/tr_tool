use std::io::{Read, Result};
use byteorder::{ReadBytesExt, LE};
use glam::{I16Vec3, IVec3, U16Vec2};
use crate::{read_boxed_slice_raw, Readable};
use super::{
	AnimDispatch, BlendMode, BoxData, Camera, Color3, Color4, Entity, Face, FrameData, MeshComponent, MeshNodeData, Meshes, Model, ObjectTextureAtlasAndTriangle, Room, SoundDetails, SoundSource, SpriteSequence, SpriteTexture, StateChange, StaticMesh, TrVersion, NUM_PIXELS, PALETTE_SIZE, SOUND_MAP_SIZE
};

pub const LIGHT_MAP_SIZE: usize = 32;

pub struct Tr3;

impl TrVersion for Tr3 {
	const FRAME_SINGLE_ROT_MASK: u16 = 1023;
}

pub struct Images {
	pub pallete_images: Box<[[u8; NUM_PIXELS]]>,
	pub images16: Box<[[u16; NUM_PIXELS]]>,
}

impl Readable for Images {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		let num_images = reader.read_u32::<LE>()? as usize;
		let pallete_images = unsafe { read_boxed_slice_raw(reader, num_images)? };//safe: arrays of primitives
		let images16 = unsafe { read_boxed_slice_raw(reader, num_images)? };
		Ok(Images { pallete_images, images16 })
	}
}

#[derive(Readable)]
pub struct SunLight {
	pub normal: I16Vec3,
	pub unused: u16,
}

#[derive(Readable)]
pub struct PointLight {
	pub intensity: u32,
	pub falloff: u32,
}

pub enum LightComponent {
	Sun(SunLight),
	Point(PointLight),
}

pub struct Light {
	/// World coords
	pub pos: IVec3,
	pub color: Color3,
	pub component: LightComponent,
}

impl Readable for Light {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		let pos = IVec3::read(reader)?;
		let color = Color3::read(reader)?;
		let component = match reader.read_u8()? {
			0 => LightComponent::Sun(SunLight::read(reader)?),
			1 => LightComponent::Point(PointLight::read(reader)?),
			a => panic!("unknown light type: {}", a),
		};
		Ok(Light { pos, color, component })
	}
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
	pub textured_quads: Box<[Face<4>]>,
	#[list_u16]
	pub textured_tris: Box<[Face<3>]>,
	#[list_u16]
	pub colored_quads: Box<[Face<4>]>,
	#[list_u16]
	pub colored_tris: Box<[Face<3>]>,
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
pub struct ObjectTexture {
	pub blend_mode: BlendMode,
	pub atlas_and_triangle: ObjectTextureAtlasAndTriangle,
	/// Units are 1/256th of a pixel
	pub vertices: [U16Vec2; 4],
}

pub struct LightMap(pub Box<[[u8; PALETTE_SIZE]; LIGHT_MAP_SIZE]>);

impl Readable for LightMap {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		unsafe {
			Ok(Self(read_boxed_slice_raw(reader, 32)?.try_into().ok().unwrap()))//exactly 32
		}//array of bytes
	}
}

#[derive(Readable)]
pub struct CinematicFrame {
	pub target: I16Vec3,
	pub pos: I16Vec3,
	pub fov: i16,
	pub roll: i16,
}

#[derive(Readable)]
pub struct Level {
	pub version: u32,
	pub palette3: Box<[Color3; PALETTE_SIZE]>,
	pub palette4: Box<[Color4; PALETTE_SIZE]>,
	pub images: Images,
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
	#[list_u32]
	pub sprite_textures: Box<[SpriteTexture]>,
	#[list_u32]
	pub sprite_sequences: Box<[SpriteSequence]>,
	#[list_u32]
	pub cameras: Box<[Camera]>,
	#[list_u32]
	pub sound_sources: Box<[SoundSource]>,
	pub box_data: BoxData,
	#[list_u32]
	pub animated_textures: Box<[u16]>,
	#[list_u32]
	pub object_textures: Box<[ObjectTexture]>,
	#[list_u32]
	pub entities: Box<[Entity]>,
	pub light_map: LightMap,
	#[list_u16]
	pub cinematic_frames: Box<[CinematicFrame]>,
	#[list_u16]
	pub demo_data: Box<[u8]>,
	pub sound_map: Box<[u16; SOUND_MAP_SIZE]>,
	#[list_u32]
	pub sound_details: Box<[SoundDetails]>,
	#[list_u32]
	pub sample_indices: Box<[u32]>,
}

pub fn read_level<R: Read>(reader: &mut R) -> Result<Level> {
	Level::read(reader)
}
