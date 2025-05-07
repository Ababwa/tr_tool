use std::{io::{Read, Result}, mem::transmute, slice::Iter};
use bitfield::bitfield;
use glam::{I16Vec3, IVec3, U16Vec2, U16Vec3, UVec2, Vec3};
use tr_readable::{read_into, Readable, ToLen};
use crate::{
	tr1::{
		get_packed_angles, AnimDispatch, Camera, Color24Bit, MeshLighting, MeshNode, Model, NumSectors,
		Portal, RoomFlags, Sector, SoundSource, Sprite, SpriteSequence, SpriteTexture, StateChange,
		StaticMesh, ATLAS_PIXELS,
	},
	tr2::{decl_frame, Axis, Color16BitArgb, FrameData, TrBox, SOUND_MAP_LEN},
	tr3::{DsQuad, RoomStaticMesh, DsTri, RoomVertex, SoundDetails},
	u16_cursor::U16Cursor,
};

pub const EXTENDED_SOUND_MAP_LEN: usize = 1024;

//model

#[derive(Clone, Debug)]
pub struct NumAtlases {
	pub num_room_atlases: u16,
	pub num_obj_atlases: u16,
	pub num_bump_atlases: u16,
}

impl ToLen for NumAtlases {
	fn get_len(&self) -> usize {
		(self.num_room_atlases + self.num_obj_atlases + self.num_bump_atlases) as usize
	}
}

#[repr(C, align(4))]
#[derive(Clone, Debug)]
pub struct Color32BitBgra {
	pub b: u8,
	pub g: u8,
	pub r: u8,
	pub a: u8,
}

#[repr(C, packed(2))]
#[derive(Clone, Debug)]
pub struct Light {
	pub pos: IVec3,
	pub color: Color24Bit,
	pub light_type: u8,
	pub unused: u8,
	pub intensity: u8,
	pub hotspot: f32,
	pub falloff: f32,
	pub length: f32,
	pub cutoff: f32,
	pub direction: Vec3,
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
	#[list(u16)] pub quads: Box<[DsQuad]>,
	#[list(u16)] pub tris: Box<[DsTri]>,
	#[list(u16)] pub sprites: Box<[Sprite]>,
	#[list(u16)] pub portals: Box<[Portal]>,
	pub num_sectors: NumSectors,
	#[list(num_sectors)] pub sectors: Box<[Sector]>,
	pub color: Color32BitBgra,
	#[list(u16)] pub lights: Box<[Light]>,
	#[list(u16)] pub room_static_meshes: Box<[RoomStaticMesh]>,
	pub flip_room_index: u16,
	pub flags: RoomFlags,
	pub water_details: u8,
	pub reverb: u8,
	pub flip_group: u8,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Animation {
	/// Byte offset into `Level.frame_data`.
	pub frame_byte_offset: u32,
	/// 30ths of a second.
	pub frame_duration: u8,
	pub num_frames: u8,
	pub state: u16,
	/// Fixed-point.
	pub speed: u32,
	/// Fixed-point.
	pub accel: u32,
	/// Fixed-point.
	pub lateral_speed: u32,
	/// Fixed-point.
	pub lateral_accel: u32,
	pub frame_start: u16,
	pub frame_end: u16,
	pub next_anim: u16,
	pub next_frame: u16,
	pub num_state_changes: u16,
	pub state_change_index: u16,
	pub num_anim_commands: u16,
	pub anim_command_index: u16,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct FlybyCamera {
	pub pos: IVec3,
	pub direction: IVec3,
	pub sequence: u8,
	pub index: u8,
	pub fov: u16,
	pub roll: u16,
	pub timer: u16,
	pub speed: u16,
	pub flags: u16,
	/// Index into `LevelData.rooms`.
	pub room_index: u32,
}

bitfield! {
	#[repr(C)]
	#[derive(Clone, Copy, Debug)]
	pub struct AtlasIndexFaceType(u16);
	pub tri, _: 15;
	pub atlas_index, _: 14, 0;
}

#[repr(C, packed(2))]
#[derive(Clone, Debug)]
pub struct ObjectTexture {
	/// One of the blend modes in the `blend_mode` module.
	pub blend_mode: u16,
	/// Index into `Level.atlases`.
	pub atlas_index_face_type: AtlasIndexFaceType,
	pub flags: u16,
	/// Units are 1/256 of a pixel.
	pub uvs: [U16Vec2; 4],
	pub unused: [u16; 4],
	pub size: UVec2,
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
	pub brightness: u16,
	pub ocb: u16,
	pub flags: u16,
}

#[derive(Clone, Debug)]
pub struct Ai {
	pub model_id: u16,
	/// Index into `LevelData.rooms`.
	pub room_index: u16,
	/// World coords.
	pub pos: IVec3,
	pub ocb: u16,
	pub flags: u16,
	pub angle: u32,
}

#[derive(Clone, Debug)]
pub enum SoundMap {
	Original(Box<[u16; SOUND_MAP_LEN]>),
	Extended(Box<[u16; EXTENDED_SOUND_MAP_LEN]>),
}

unsafe fn read_sound_map<R: Read>(reader: &mut R, this: *mut SoundMap, demo_data: &Box<[u8]>) -> Result<()> {
	if demo_data.len() == 2048 {
		let mut sound_map = Box::new_uninit();
		read_into(reader, sound_map.as_mut_ptr())?;
		this.write(SoundMap::Extended(sound_map.assume_init()));
	} else {
		let mut sound_map = Box::new_uninit();
		read_into(reader, sound_map.as_mut_ptr())?;
		this.write(SoundMap::Original(sound_map.assume_init()));
	}
	Ok(())
}

#[derive(Readable, Clone, Debug)]
pub struct LevelData {
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
	pub spr: [u8; 3],
	#[list(u32)] pub sprite_textures: Box<[SpriteTexture]>,
	#[list(u32)] pub sprite_sequences: Box<[SpriteSequence]>,
	#[list(u32)] pub cameras: Box<[Camera]>,
	#[list(u32)] pub flyby_cameras: Box<[FlybyCamera]>,
	#[list(u32)] pub sound_sources: Box<[SoundSource]>,
	#[list(u32)] pub boxes: Box<[TrBox]>,
	#[list(u32)] pub overlap_data: Box<[u16]>,
	#[list(boxes)] pub zone_data: Box<[[u16; 10]]>,
	#[list(u32)] pub animated_textures: Box<[u16]>,
	pub animated_textures_uv_count: u8,
	pub tex: [u8; 3],
	#[list(u32)] pub object_textures: Box<[ObjectTexture]>,
	#[list(u32)] pub entities: Box<[Entity]>,
	#[list(u32)] pub ais: Box<[Ai]>,
	#[list(u16)] pub demo_data: Box<[u8]>,
	#[delegate(read_sound_map, demo_data)] pub sound_map: SoundMap,
	#[list(u32)] pub sound_details: Box<[SoundDetails]>,
	#[list(u32)] pub sample_indices: Box<[u32]>,
	pub padding: [u8; 6],
}

#[derive(Readable, Clone, Debug)]
pub struct Sample {
	pub uncompressed_size: u32,
	#[list(u32)] pub data: Box<[u8]>,
}

#[derive(Readable, Clone, Debug)]
pub struct Level {
	pub version: u32,
	pub num_atlases: NumAtlases,
	#[zlib] #[list(num_atlases)] pub atlases_32bit: Box<[[Color32BitBgra; ATLAS_PIXELS]]>,
	#[zlib] #[list(num_atlases)] pub atlases_16bit: Box<[[Color16BitArgb; ATLAS_PIXELS]]>,
	#[zlib] #[boxed] pub misc_images: Box<[[Color32BitBgra; ATLAS_PIXELS]; 2]>,
	#[zlib] #[delegate] pub level_data: LevelData,
	#[list(u32)] #[delegate] pub samples: Box<[Sample]>,
}

//extraction

bitfield! {
	#[repr(C)]
	#[derive(Clone, Debug)]
	pub struct FaceEffects(u16);
	pub additive, _: 0;
}

macro_rules! decl_face_type {
	($name:ident, $num_indices:literal) => {
		#[repr(C)]
		#[derive(Clone, Debug)]
		pub struct $name {
			pub vertex_indices: [u16; $num_indices],
			pub object_texture_index: u16,
			pub flags: FaceEffects,
		}
	};
}

decl_face_type!(EffectsQuad, 4);
decl_face_type!(EffectsTri, 3);

pub struct Mesh<'a> {
	pub center: I16Vec3,
	pub radius: i32,
	/// If static mesh, relative to `RoomStaticMesh.pos`.
	/// If entity mesh, relative to `Entity.pos`.
	pub vertices: &'a [I16Vec3],
	pub lighting: MeshLighting<'a>,
	pub quads: &'a [EffectsQuad],
	pub tris: &'a [EffectsTri],
}

impl<'a> Mesh<'a> {
	pub(crate) fn get(mesh_data: &'a [u16], mesh_offset: u32) -> Self {
		assert!(mesh_offset % 2 == 0);
		let mut cursor = U16Cursor::new(&mesh_data[mesh_offset as usize / 2..]);
		unsafe {
			Self {
				center: cursor.read(),
				radius: cursor.read(),
				vertices: cursor.u16_len_slice(),
				lighting: match cursor.next() as i16 {
					len if len > 0 => MeshLighting::Normals(cursor.slice(len as usize)),
					len => MeshLighting::Lights(cursor.slice(-len as usize)),
				},
				quads: cursor.u16_len_slice(),
				tris: cursor.u16_len_slice(),
			}
		}
	}
}

decl_frame!(Frame, RotationIterator, FrameRotation, 0xFFF);

impl Level {
	pub fn get_mesh(&self, mesh_offset: u32) -> Mesh {
		Mesh::get(&self.level_data.mesh_data, mesh_offset)
	}
	
	pub fn get_mesh_nodes(&self, model: &Model) -> &[MeshNode] {
		MeshNode::get(&self.level_data.mesh_node_data, model.mesh_node_offset, model.num_meshes)
	}
	
	pub fn get_frame(&self, model: &Model) -> Frame {
		Frame::get(&self.level_data.frame_data, model.frame_byte_offset, model.num_meshes)
	}
}
