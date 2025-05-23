/*
Terms:
An "offset" marks a starting point of some kind.
An "index" points to an entry in an array.
32-bit color type names list channels in byte-order.
16-bit color type names list channels in bit-order, high first.
*/

use std::{mem, slice};
use bitfield::bitfield;
use glam::{I16Vec2, I16Vec3, IVec3, U16Vec2, U16Vec3, U8Vec2};
use shared::min_max::MinMax;
use tr_readable::{Readable, ToLen};
use crate::u16_cursor::U16Cursor;

pub const ATLAS_SIDE_LEN: usize = 256;
pub const ATLAS_PIXELS: usize = ATLAS_SIDE_LEN * ATLAS_SIDE_LEN;
pub const PALETTE_LEN: usize = 256;
pub const LIGHT_MAP_LEN: usize = 32;
pub const SOUND_MAP_LEN: usize = 256;
pub const ZONE_SIZE: usize = 6;

pub mod blend_mode {
	pub const OPAQUE: u16 = 0;
	pub const TEST: u16 = 1;
}

//model

#[repr(C)]
#[derive(Clone, Debug)]
pub struct RoomVertex {
	/// Relative to room
	pub pos: I16Vec3,
	pub light: u16,
}

macro_rules! decl_face_type {
	($name:ident, $num_indices:literal, $texture_field:ident) => {
		#[repr(C)]
		#[derive(Clone, Debug)]
		pub struct $name {
			pub vertex_indices: [u16; $num_indices],
			pub $texture_field: u16,
		}
	};
}

decl_face_type!(TexturedQuad, 4, object_texture_index);
decl_face_type!(TexturedTri, 3, object_texture_index);

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Sprite {
	pub vertex_index: u16,
	pub sprite_texture_index: u16,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Portal {
	pub adjoining_room_index: u16,
	pub normal: I16Vec3,
	pub vertices: [I16Vec3; 4],
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct NumSectors {
	pub z: u16,
	pub x: u16,
}

impl ToLen for NumSectors {
	fn get_len(&self) -> usize {
		(self.z * self.x) as usize
	}
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Sector {
	pub floor_data_index: u16,
	pub box_index: u16,
	pub room_below_index: u8,
	pub floor: i8,
	pub room_above_index: u8,
	pub ceiling: i8,
}

#[repr(C, packed(2))]
#[derive(Clone, Debug)]
pub struct Light {
	pub pos: IVec3,
	pub brightness: u16,
	pub fade: u32,
}

#[repr(C, packed(2))]
#[derive(Clone, Debug)]
pub struct RoomStaticMesh {
	/// World coords.
	pub pos: IVec3,
	/// Units are 1/65536 of a rotation.
	pub angle: u16,
	pub light: u16,
	/// Matched to `StaticMesh.id` in `Level.static_meshes`.
	pub static_mesh_id: u16,
}

bitfield! {
	#[repr(C)]
	#[derive(Clone, Copy, Debug)]
	pub struct RoomFlags(u16);
	pub water, _: 0;
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
	#[list(u16)] pub quads: Box<[TexturedQuad]>,
	#[list(u16)] pub tris: Box<[TexturedTri]>,
	#[list(u16)] pub sprites: Box<[Sprite]>,
	#[list(u16)] pub portals: Box<[Portal]>,
	pub num_sectors: NumSectors,
	#[list(num_sectors)] pub sectors: Box<[Sector]>,
	pub ambient_light: u16,
	#[list(u16)] pub lights: Box<[Light]>,
	#[list(u16)] pub room_static_meshes: Box<[RoomStaticMesh]>,
	/// Index into `Level.rooms`.
	pub flip_room_index: u16,
	pub flags: RoomFlags,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Animation {
	pub frame_byte_offset: u32,
	pub frame_duration: u8,
	pub num_frames: u8,
	pub state_id: u16,
	pub speed: u32,
	pub accel: u32,
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
pub struct StateChange {
	pub state_id: u16,
	pub num_anim_dispatches: u16,
	pub anim_dispatch_id: u16,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct AnimDispatch {
	pub low_frame: u16,
	pub high_frame: u16,
	pub next_anim_id: u16,
	pub next_frame_id: u16,
}

#[repr(C, packed(2))]
#[derive(Clone, Debug)]
pub struct Model {
	pub id: u32,
	pub num_meshes: u16,
	/// Index into `Level.mesh_offsets`.
	pub mesh_offset_index: u16,
	/// Offset into `Level.mesh_node_data`.
	pub mesh_node_offset: u32,
	/// Byte offset into `Level.frame_data`.
	pub frame_byte_offset: u32,
	/// Index into `Level.animations`.
	pub anim_index: u16,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct BoundBox {
	pub x: MinMax<i16>,
	pub y: MinMax<i16>,
	pub z: MinMax<i16>,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct StaticMesh {
	pub id: u32,
	/// Index into `Level.mesh_offsets`.
	pub mesh_offset_index: u16,
	pub visibility: BoundBox,
	pub collision: BoundBox,
	pub flags: u16,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct ObjectTexture {
	/// One of the blend modes in the `blend_mode` module.
	pub blend_mode: u16,
	/// Index into `Level.atlases`.
	pub atlas_index: u16,
	/// Units are 1/256 of a pixel.
	pub uvs: [U16Vec2; 4],
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct SpriteTexture {
	/// Index into `Level.atlases`.
	pub atlas_index: u16,
	pub pos: U8Vec2,
	pub size: U16Vec2,
	pub world_bounds: [I16Vec2; 2],
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct SpriteSequence {
	pub id: u32,
	pub neg_length: i16,
	/// Index into `Level.sprite_textures`.
	pub sprite_texture_index: u16,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Camera {
	pub pos: IVec3,
	pub room_index: u16,
	pub flags: u16,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct SoundSource {
	pub pos: IVec3,
	pub sound_id: u16,
	pub flags: u16,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct TrBox {
	pub z: MinMax<u32>,
	pub x: MinMax<u32>,
	pub y: i16,
	pub overlap: u16,
}

#[repr(C, packed(2))]
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
	pub flags: u16,
}

/// 6 bits per channel
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct Color24Bit {
	pub r: u8,
	pub g: u8,
	pub b: u8,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct CinematicFrame {
	pub target: I16Vec3,
	pub pos: I16Vec3,
	pub fov: i16,
	pub roll: i16,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct SoundDetails {
	pub sample_index: u16,
	pub volume: u16,
	pub chance: u16,
	pub details: u16,
}

#[derive(Readable, Clone, Debug)]
pub struct Level {
	pub version: u32,
	#[list(u32)] pub atlases: Box<[[u8; ATLAS_PIXELS]]>,
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
	#[list(u32)] pub object_textures: Box<[ObjectTexture]>,
	#[list(u32)] pub sprite_textures: Box<[SpriteTexture]>,
	#[list(u32)] pub sprite_sequences: Box<[SpriteSequence]>,
	#[list(u32)] pub cameras: Box<[Camera]>,
	#[list(u32)] pub sound_sources: Box<[SoundSource]>,
	#[list(u32)] pub boxes: Box<[TrBox]>,
	#[list(u32)] pub overlap_data: Box<[u16]>,
	#[list(boxes)] pub zone_data: Box<[[u16; ZONE_SIZE]]>,
	#[list(u32)] pub animated_textures: Box<[u16]>,
	#[list(u32)] pub entities: Box<[Entity]>,
	#[boxed] pub light_map: Box<[[u8; PALETTE_LEN]; LIGHT_MAP_LEN]>,
	#[boxed] pub palette: Box<[Color24Bit; PALETTE_LEN]>,
	#[list(u16)] pub cinematic_frames: Box<[CinematicFrame]>,
	#[list(u16)] pub demo_data: Box<[u8]>,
	#[boxed] pub sound_map: Box<[u16; SOUND_MAP_LEN]>,
	#[list(u32)] pub sound_details: Box<[SoundDetails]>,
	#[list(u32)] pub sample_data: Box<[u8]>,
	#[list(u32)] pub sample_indices: Box<[u32]>,
}

//extraction

#[derive(Clone, Debug)]
pub enum MeshLighting<'a> {
	Normals(&'a [I16Vec3]),
	Lights(&'a [u16]),
}

decl_face_type!(SolidQuad, 4, color_index);
decl_face_type!(SolidTri, 3, color_index);

macro_rules! decl_mesh {
	(
		$mesh:ident,
		$mesh_lighting:ident,
		$textured_quad:ty,
		$textured_tri:ty,
		$solid_quad:ty,
		$solid_tri:ty
	) => {
		#[derive(Clone, Debug)]
		pub struct $mesh<'a> {
			pub center: glam::I16Vec3,
			pub radius: i32,
			/// If static mesh, relative to `RoomStaticMesh.pos`.
			/// If entity mesh, relative to `Entity.pos`.
			pub vertices: &'a [glam::I16Vec3],
			pub lighting: $mesh_lighting<'a>,
			pub textured_quads: &'a [$textured_quad],
			pub textured_tris: &'a [$textured_tri],
			pub solid_quads: &'a [$solid_quad],
			pub solid_tris: &'a [$solid_tri],
		}
		
		impl<'a> $mesh<'a> {
			pub(crate) fn get(mesh_data: &'a [u16], mesh_byte_offset: u32) -> Self {
				let byte_offset = mesh_byte_offset as usize;
				assert!(byte_offset % size_of::<u16>() == 0);
				let offset = byte_offset / size_of::<u16>();
				let mesh_data = &mesh_data[offset..];
				let mut cursor = U16Cursor::new(mesh_data);
				unsafe {
					Self {
						center: cursor.read(),
						radius: cursor.read(),
						vertices: cursor.u16_len_slice(),
						lighting: match cursor.next() as i16 {
							len if len > 0 => $mesh_lighting::Normals(cursor.slice(len as usize)),
							len => $mesh_lighting::Lights(cursor.slice(-len as usize)),
						},
						textured_quads: cursor.u16_len_slice(),
						textured_tris: cursor.u16_len_slice(),
						solid_quads: cursor.u16_len_slice(),
						solid_tris: cursor.u16_len_slice(),
					}
				}
			}
		}
	}
}
pub(crate) use decl_mesh;

decl_mesh!(Mesh, MeshLighting, TexturedQuad, TexturedTri, SolidQuad, SolidTri);

bitfield! {
	#[repr(C)]
	#[derive(Clone, Debug)]
	pub struct MeshNodeFlags(u32);
	pub pop, _: 0;
	pub push, _: 1;
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct MeshNode {
	pub flags: MeshNodeFlags,
	/// Relative to parent.
	pub offset: IVec3,
}

impl MeshNode {
	pub(crate) fn get<'a>(mesh_node_data: &'a [u32], mesh_node_offset: u32, num_meshes: u16) -> &'a [Self] {
		let offset = mesh_node_offset as usize;
		let len = num_meshes as usize - 1;
		let size = len * (size_of::<Self>() / size_of::<u32>());
		assert!(offset + size <= mesh_node_data.len());
		unsafe {
			let ptr = mesh_node_data.as_ptr().add(offset).cast();
			slice::from_raw_parts(ptr, len)
		}
	}
}

pub(crate) fn get_packed_angles(xy: u16, yz: u16) -> U16Vec3 {
	U16Vec3 {
		x: (xy >> 4) & 1023,
		y: ((xy & 15) << 6) | (yz >> 10),
		z: yz & 1023,
	}
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct FrameRotation(pub u16, pub u16);

impl FrameRotation {
	pub fn get_angles(&self) -> U16Vec3 {
		get_packed_angles(self.1, self.0)
	}
}

#[repr(C)]
#[derive(Debug)]
pub struct Frame {
	pub bound_box: MinMax<I16Vec3>,
	pub offset: I16Vec3,
	pub num_meshes: u16,
	pub rotations: [FrameRotation],
}

impl Level {
	pub fn get_mesh(&self, mesh_byte_offset: u32) -> Mesh {
		Mesh::get(&self.mesh_data, mesh_byte_offset)
	}
	
	pub fn get_mesh_nodes(&self, model: &Model) -> &[MeshNode] {
		MeshNode::get(&self.mesh_node_data, model.mesh_node_offset, model.num_meshes)
	}
	
	pub fn get_frame(&self, model: &Model) -> &Frame {
		/// Size of the known part of Frame in u16s.
		const FRAME_KNOWN_SIZE: usize = 10;
		let byte_offset = model.frame_byte_offset as usize;
		assert!(byte_offset % size_of::<u16>() == 0);
		let offset = (byte_offset / size_of::<u16>()) as usize;
		let len = model.num_meshes as usize;
		let size = FRAME_KNOWN_SIZE + len * (size_of::<FrameRotation>() / size_of::<u16>());
		assert!(offset + size <= self.frame_data.len());
		unsafe {
			let ptr = self.frame_data.as_ptr().add(offset) as usize;
			mem::transmute([ptr, len])//no nice way to make unsized struct
		}
	}
}
