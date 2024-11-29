use std::{io::{Read, Result}, ptr::addr_of_mut};
use bitfield::bitfield;
use glam::{I16Vec3, IVec3, U16Vec2, UVec2, Vec3};
use tr_readable::{read_boxed_flat, read_boxed_slice_flat, read_flat, zlib, Readable};
use crate::{
	tr1::{
		AnimDispatch, Camera, Color24Bit, MeshLighting, MeshNode, Model, Portal, RoomFlags, Sectors,
		SoundSource, SpriteSequence, SpriteTexture, StateChange, StaticMesh, ATLAS_PIXELS,
	},
	tr2::{decl_frame, Axis, BoxData, Color16BitArgb, FrameData, SOUND_MAP_LEN},
	tr3::{RoomGeom, RoomStaticMesh, SoundDetails},
	u16_cursor::U16Cursor,
};

//model

#[repr(C, align(4))]
#[derive(Clone, Debug)]
pub struct Color32BitBbga {
	pub b: u8,
	pub g: u8,
	pub r: u8,
	pub a: u8,
}

#[derive(Clone, Debug)]
pub struct Atlases {
	pub num_room_atlases: u16,
	pub num_obj_atlases: u16,
	pub num_bump_atlases: u16,
	pub atlases_32bit: Box<[[Color32BitBbga; ATLAS_PIXELS]]>,
	pub atlases_16bit: Box<[[Color16BitArgb; ATLAS_PIXELS]]>,
	pub misc_images: Box<[[Color32BitBbga; ATLAS_PIXELS]; 2]>,
}

impl Readable for Atlases {
	unsafe fn read<R: Read>(reader: &mut R, this: *mut Self) -> Result<()> {
		read_flat(reader, addr_of_mut!((*this).num_room_atlases))?;
		read_flat(reader, addr_of_mut!((*this).num_obj_atlases))?;
		read_flat(reader, addr_of_mut!((*this).num_bump_atlases))?;
		let num_atlases = (
			(*this).num_room_atlases + (*this).num_obj_atlases + (*this).num_bump_atlases
		) as usize;
		read_boxed_slice_flat(&mut zlib(reader)?, num_atlases, addr_of_mut!((*this).atlases_32bit))?;
		read_boxed_slice_flat(&mut zlib(reader)?, num_atlases, addr_of_mut!((*this).atlases_16bit))?;
		read_boxed_flat(&mut zlib(reader)?, addr_of_mut!((*this).misc_images))?;
		Ok(())
	}
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
	#[flat] pub x: i32,
	/// World coord.
	#[flat] pub z: i32,
	#[flat] pub y_bottom: i32,
	#[flat] pub y_top: i32,
	#[flat] #[list(u32)] pub geom_data: Box<[u16]>,
	#[flat] #[list(u16)] pub portals: Box<[Portal]>,
	#[delegate] pub sectors: Sectors,
	#[flat] pub color: Color32BitBbga,
	#[flat] #[list(u16)] pub lights: Box<[Light]>,
	#[flat] #[list(u16)] pub room_static_meshes: Box<[RoomStaticMesh]>,
	#[flat] pub flip_room_index: u16,
	#[flat] pub flags: RoomFlags,
	#[flat] pub water_details: u8,
	#[flat] pub reverb: u8,
	#[flat] pub flip_group: u8,
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
	pub unused: [u32; 2],
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

#[derive(Readable, Clone, Debug)]
pub struct LevelData {
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
	#[flat] pub spr: [u8; 3],
	#[flat] #[list(u32)] pub sprite_textures: Box<[SpriteTexture]>,
	#[flat] #[list(u32)] pub sprite_sequences: Box<[SpriteSequence]>,
	#[flat] #[list(u32)] pub cameras: Box<[Camera]>,
	#[flat] #[list(u32)] pub flyby_cameras: Box<[FlybyCamera]>,
	#[flat] #[list(u32)] pub sound_sources: Box<[SoundSource]>,
	#[delegate] pub box_data: BoxData,
	#[flat] #[list(u32)] pub animated_textures: Box<[u16]>,
	#[flat] pub animated_textures_uv_count: u8,
	#[flat] pub tex: [u8; 3],
	#[flat] #[list(u32)] pub object_textures: Box<[ObjectTexture]>,
	#[flat] #[list(u32)] pub entities: Box<[Entity]>,
	#[flat] #[list(u32)] pub ais: Box<[Ai]>,
	#[flat] #[list(u16)] pub demo_data: Box<[u8]>,
	#[flat] #[boxed] pub sound_map: Box<[u16; SOUND_MAP_LEN]>,
	#[flat] #[list(u32)] pub sound_details: Box<[SoundDetails]>,
	#[flat] #[list(u32)] pub sample_indices: Box<[u32]>,
	#[flat] pub zero: [u8; 6],
}

#[derive(Readable, Clone, Debug)]
pub struct Sample {
	#[flat] pub uncompressed_size: u32,
	#[flat] #[list(u32)] pub data: Box<[u8]>,
}

#[derive(Readable, Clone, Debug)]
pub struct Level {
	#[flat] pub version: u32,
	#[delegate] pub atlases: Atlases,
	#[zlib] pub level_data: LevelData,
	#[delegate] #[list(u32)] pub samples: Box<[Sample]>,
}

//extraction

impl Room {
	pub fn get_geom(&self) -> RoomGeom {
		RoomGeom::get(&self.geom_data)
	}
}

bitfield! {
	#[repr(C)]
	#[derive(Clone, Debug)]
	pub struct MeshFaceFlags(u16);
	pub additive, _: 0;
}

macro_rules! decl_face_type {
	($name:ident, $num_indices:literal) => {
		#[repr(C)]
		#[derive(Clone, Debug)]
		pub struct $name {
			pub vertex_indices: [u16; $num_indices],
			pub object_texture_index: u16,
			pub flags: MeshFaceFlags,
		}
	};
}

decl_face_type!(MeshQuad, 4);
decl_face_type!(MeshTri, 3);

pub struct Mesh<'a> {
	pub center: I16Vec3,
	pub radius: i32,
	/// If static mesh, relative to `RoomStaticMesh.pos`.
	/// If entity mesh, relative to `Entity.pos`.
	pub vertices: &'a [I16Vec3],
	pub lighting: MeshLighting<'a>,
	pub quads: &'a [MeshQuad],
	pub tris: &'a [MeshTri],
}

impl<'a> Mesh<'a> {
	fn get(mesh_data: &'a [u16], mesh_offset: u32) -> Self {
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

decl_frame!(Frame, FrameData, RotationIterator, FrameRotation, Axis, Model, 0xFFF);

impl Level {
	pub fn get_mesh(&self, mesh_offset: u32) -> Mesh {
		Mesh::get(&self.level_data.mesh_data, mesh_offset)
	}
	
	pub fn get_mesh_nodes(&self, model: &Model) -> &[MeshNode] {
		MeshNode::get(&self.level_data.mesh_node_data, model)
	}
	
	pub fn get_frame(&self, model: &Model) -> Frame {
		Frame::get(&self.level_data.frame_data, model)
	}
}
