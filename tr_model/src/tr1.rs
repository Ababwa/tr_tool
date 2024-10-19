use std::{io::{Read, Result}, mem::transmute, ptr::addr_of_mut, slice};
use bitfield::bitfield;
use glam::{I16Vec2, I16Vec3, IVec3, U16Vec2, U16Vec3};
use glam_traits::ext::U8Vec2;
use nonmax::{NonMaxU16, NonMaxU8};
use shared::min_max::MinMax;
use tr_readable::{read_boxed_slice_flat, read_flat, read_val_flat, Readable};
use crate::u16_cursor::U16Cursor;

pub const ATLAS_SIDE_LEN: usize = 256;
pub const ATLAS_PIXELS: usize = ATLAS_SIDE_LEN * ATLAS_SIDE_LEN;
pub const PALETTE_LEN: usize = 256;
pub const SOUND_MAP_LEN: usize = 256;
pub const LIGHT_MAP_LEN: usize = 32;
pub const ZONE_MULT: usize = 6;
pub const FRAME_SINGLE_ROT_MASK: u16 = 1023;

pub mod blend_mode {
	pub const OPAQUE: u16 = 0;
	pub const TEST: u16 = 1;
}

//model

#[repr(C)]
#[derive(Clone, Copy)]
pub struct RoomVertex {
	/// Relative to room
	pub pos: I16Vec3,
	pub light: u16,
}

macro_rules! decl_face_type {
	($name:ident, $num_indices:literal, $texture_field:ident) => {
		#[repr(C)]
		#[derive(Clone, Copy)]
		pub struct $name {
			pub vertex_indices: [u16; $num_indices],
			pub $texture_field: u16,
		}
	};
}

decl_face_type!(RoomQuad, 4, object_texture_index);
decl_face_type!(RoomTri, 3, object_texture_index);
decl_face_type!(MeshTexturedQuad, 4, object_texture_index);
decl_face_type!(MeshTexturedTri, 3, object_texture_index);
decl_face_type!(MeshSolidQuad, 4, color_index);
decl_face_type!(MeshSolidTri, 3, color_index);

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Portal {
	/// Index into `Level.rooms`.
	pub adjoining_room_index: u16,
	pub normal: I16Vec3,
	/// Relative to room.
	pub vertices: [I16Vec3; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Sector {
	/// Index into `Level.floor_data`.
	pub floor_data_index: u16,
	/// Index into `BoxData.boxes`.
	pub box_index: Option<NonMaxU16>,
	/// Index into `Level.rooms`.
	pub room_below_id: Option<NonMaxU8>,
	pub floor: i8,
	/// Index into `Level.rooms`.
	pub room_above_index: Option<NonMaxU8>,
	pub ceiling: i8,
}

#[derive(Clone)]
pub struct Sectors {
	pub num_sectors: U16Vec2,
	pub sectors: Box<[Sector]>,
}

impl Readable for Sectors {
	unsafe fn read<R: Read>(reader: &mut R, this: *mut Self) -> Result<()> {
		read_flat(reader, addr_of_mut!((*this).num_sectors))?;
		let len = (*this).num_sectors.element_product() as usize;
		read_boxed_slice_flat(reader, addr_of_mut!((*this).sectors), len)?;
		Ok(())
	}
}

#[repr(C, packed(2))]
#[derive(Clone, Copy)]
pub struct Light {
	pub pos: IVec3,
	pub brightness: u16,
	pub fallout: u32,
}

#[repr(C, packed(2))]
#[derive(Clone, Copy)]
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
	#[derive(Clone, Copy)]
	pub struct RoomFlags(u16);
	pub water, _: 0;
}

#[derive(Readable, Clone)]
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
	#[flat] #[list(u16)] pub lights: Box<[Light]>,
	#[flat] #[list(u16)] pub room_static_meshes: Box<[RoomStaticMesh]>,
	/// Index into `Level.rooms`.
	#[flat] pub flip_room_index: Option<NonMaxU16>,
	#[flat] pub flags: RoomFlags,
}

#[repr(C)]
#[derive(Clone, Copy)]
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
	pub frame_start: u16,
	pub frame_end: u16,
	pub next_anim: u16,
	pub next_frame: u16,
	pub num_state_changes: u16,
	pub state_change_id: u16,
	pub num_anim_commands: u16,
	pub anim_command_id: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StateChange {
	pub state: u16,
	pub num_anim_dispatches: u16,
	pub anim_dispatch_id: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct AnimDispatch {
	pub low_frame: u16,
	pub high_frame: u16,
	pub next_anim_id: u16,
	pub next_frame_id: u16,
}

#[repr(C, packed(2))]
#[derive(Clone, Copy)]
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
	pub anim_index: Option<NonMaxU16>,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct BoundBox {
	pub x: MinMax<i16>,
	pub y: MinMax<i16>,
	pub z: MinMax<i16>,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct StaticMesh {
	pub id: u32,
	/// Index into `Level.mesh_offsets`.
	pub mesh_offset_index: u16,
	pub visibility: BoundBox,
	pub collision: BoundBox,
	pub flags: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct ObjectTexture {
	pub blend_mode: u16,
	/// Index into `Level.atlases`.
	pub atlas_index: u16,
	/// Units are 1/256 of a pixel.
	pub uvs: [U16Vec2; 4],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SpriteTexture {
	/// Index into `Level.atlases`.
	pub atlas_index: u16,
	pub pos: U8Vec2,
	pub size: U16Vec2,
	pub world_bounds: [I16Vec2; 2],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SpriteSequence {
	pub id: u32,
	pub neg_length: i16,
	/// Index into `Level.sprite_textures`.
	pub sprite_index: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Camera {
	/// World coords.
	pub pos: IVec3,
	/// Index into `Level.rooms`.
	pub room_index: u16,
	pub flags: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SoundSource {
	/// World coords.
	pub pos: IVec3,
	pub sound_id: u16,
	pub flags: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct TrBox {
	/// Sectors.
	pub z: MinMax<u32>,
	pub x: MinMax<u32>,
	pub y: i16,
	pub overlap: u16,
}

#[derive(Clone)]
pub struct BoxData {
	pub boxes: Box<[TrBox]>,
	pub overlap_data: Box<[u16]>,
	pub zone_data: Box<[u16]>,
}

impl Readable for BoxData {
	unsafe fn read<R: Read>(reader: &mut R, this: *mut Self) -> Result<()> {
		let boxes_len = read_val_flat::<_, u32>(reader)? as usize;
		read_boxed_slice_flat(reader, addr_of_mut!((*this).boxes), boxes_len)?;
		let overlaps_len = read_val_flat::<_, u32>(reader)? as usize;
		read_boxed_slice_flat(reader, addr_of_mut!((*this).overlap_data), overlaps_len)?;
		read_boxed_slice_flat(reader, addr_of_mut!((*this).zone_data), boxes_len * ZONE_MULT)?;
		Ok(())
	}
}

#[repr(C, packed(2))]
#[derive(Clone, Copy)]
pub struct Entity {
	/// Matched to `Model.id` in `Level.models` or `SpriteSequence.id` in `Level.sprite_sequences`.
	pub model_id: u16,
	/// Index into `Level.rooms`.
	pub room_index: u16,
	/// World coords.
	pub pos: IVec3,
	/// Units are 1/65536th of a rotation.
	pub angle: u16,
	/// If None, use mesh light.
	pub brightness: Option<NonMaxU16>,
	pub flags: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Color6Bit {
	pub r: u8,
	pub g: u8,
	pub b: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct CinematicFrame {
	pub target: I16Vec3,
	pub pos: I16Vec3,
	pub fov: i16,
	pub roll: i16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct SoundDetails {
	/// Index into `Level.sample_indices`.
	pub sample_index: u16,
	pub volume: u16,
	pub chance: u16,
	pub details: u16,
}

#[derive(Readable, Clone)]
pub struct Level {
	#[flat] pub version: u32,
	#[flat] #[list(u32)] pub atlases: Box<[[u8; ATLAS_PIXELS]]>,
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
	#[flat] #[list(u32)] pub object_textures: Box<[ObjectTexture]>,
	#[flat] #[list(u32)] pub sprite_textures: Box<[SpriteTexture]>,
	#[flat] #[list(u32)] pub sprite_sequences: Box<[SpriteSequence]>,
	#[flat] #[list(u32)] pub cameras: Box<[Camera]>,
	#[flat] #[list(u32)] pub sound_sources: Box<[SoundSource]>,
	#[delegate] pub box_data: BoxData,
	#[flat] #[list(u32)] pub animated_textures: Box<[u16]>,
	#[flat] #[list(u32)] pub entities: Box<[Entity]>,
	#[flat] #[boxed] pub light_map: Box<[[u8; PALETTE_LEN]; LIGHT_MAP_LEN]>,
	#[flat] #[boxed] pub palette: Box<[Color6Bit; PALETTE_LEN]>,
	#[flat] #[list(u16)] pub cinematic_frames: Box<[CinematicFrame]>,
	#[flat] #[list(u16)] pub demo_data: Box<[u8]>,
	#[flat] #[boxed] pub sound_map: Box<[u16; SOUND_MAP_LEN]>,
	#[flat] #[list(u32)] pub sound_details: Box<[SoundDetails]>,
	#[flat] #[list(u32)] pub sample_data: Box<[u8]>,
	#[flat] #[list(u32)] pub sample_indices: Box<[u32]>,
}

//extraction

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Sprite {
	/// Index into `Room.vertices`.
	pub vertex_index: u16,
	/// Index into `Level.sprite_textures`.
	pub sprite_texture_index: u16,
}

#[derive(Clone, Copy)]
pub struct RoomGeom<'a> {
	pub vertices: &'a [RoomVertex],
	pub quads: &'a [RoomQuad],
	pub tris: &'a [RoomTri],
	pub sprites: &'a [Sprite],
}

impl Room {
	pub fn get_geom_data(&self) -> RoomGeom {
		let mut cursor = U16Cursor::new(&self.geom_data);
		unsafe {
			RoomGeom {
				vertices: cursor.u16_len_slice(),
				quads: cursor.u16_len_slice(),
				tris: cursor.u16_len_slice(),
				sprites: cursor.u16_len_slice(),
			}
		}
	}
}

#[derive(Clone, Copy)]
pub enum MeshLighting<'a> {
	Normals(&'a [I16Vec3]),
	Lights(&'a [u16]),
}

#[derive(Clone, Copy)]
pub struct Mesh<'a> {
	pub center: I16Vec3,
	pub radius: i32,
	/// If static mesh, relative to `RoomStaticMesh.pos`.
	/// If entity mesh, relative to `Entity.pos`.
	pub vertices: &'a [I16Vec3],
	pub lighting: MeshLighting<'a>,
	pub textured_quads: &'a [MeshTexturedQuad],
	pub textured_tris: &'a [MeshTexturedTri],
	pub solid_quads: &'a [MeshSolidQuad],
	pub solid_tris: &'a [MeshSolidTri],
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct FrameRotation(u16, u16);

impl FrameRotation {
	pub fn get_angles(&self) -> U16Vec3 {
		U16Vec3 {
			x: (self.1 >> 4) & 1023,
			y: ((self.1 & 15) << 6) | (self.0 >> 10),
			z: self.0 & 1023,
		}
	}
}

#[repr(C)]
pub struct Frame {
	pub bound_box: MinMax<I16Vec3>,
	pub offset: I16Vec3,
	pub num_meshes: u16,
	pub rotations: [FrameRotation],
}

bitfield! {
	#[repr(C)]
	#[derive(Clone, Copy)]
	pub struct MeshNodeFlags(u32);
	pub pop, _: 0;
	pub push, _: 1;
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MeshNode {
	pub flags: MeshNodeFlags,
	/// Relative to parent.
	pub offset: IVec3,
}

impl Level {
	pub fn get_mesh(&self, offset: u32) -> Mesh {
		let mut cursor = U16Cursor::new(&self.mesh_data[offset as usize / 2..]);
		unsafe {
			Mesh {
				center: cursor.read(),
				radius: cursor.read(),
				vertices: cursor.u16_len_slice(),
				lighting: match cursor.next() as i16 {
					len if len > 0 => MeshLighting::Normals(cursor.slice(len as usize)),
					len => MeshLighting::Lights(cursor.slice(-len as usize)),
				},
				textured_quads: cursor.u16_len_slice(),
				textured_tris: cursor.u16_len_slice(),
				solid_quads: cursor.u16_len_slice(),
				solid_tris: cursor.u16_len_slice(),
			}
		}
	}
	
	pub fn get_frame(&self, frame_byte_offset: u32) -> &Frame {
		let frame_offset = frame_byte_offset as usize / 2;
		let ptr = self.frame_data[frame_offset..].as_ptr() as usize;
		let len = self.frame_data[frame_offset + 9] as usize;//offset of num_meshes in frame
		unsafe { transmute([ptr, len]) }//no nice way to make dynamically sized struct
	}
	
	/// Should be called with `Model.num_meshes - 1`.
	pub fn get_mesh_nodes(&self, mesh_node_offset: u32, num_meshes: u16) -> &[MeshNode] {
		let ptr = self.mesh_node_data[mesh_node_offset as usize..].as_ptr() as *const MeshNode;
		unsafe { slice::from_raw_parts(ptr, num_meshes as usize) }
	}
	
	// pub fn get_frame(&self, frame_byte_offset: u32, num_meshes: u16) -> Frame {
	// 	let frame_offset = frame_byte_offset as usize / 2;
	// 	let &(bound_box, offset) = unsafe {
	// 		reinterpret::slice_to_ref(&self.frame_data[frame_offset..][..9])
	// 	};//safe: same alignment, pod output
	// 	let mut rotations = Vec::with_capacity(num_meshes as usize);
	// 	let mut frame_offset = frame_offset + 9;
	// 	for _ in 0..num_meshes {
	// 		let word = self.frame_data[frame_offset];
	// 		let (rot, advance) = match word >> 14 {
	// 			0 => {
	// 				let word2 = self.frame_data[frame_offset + 1];
	// 				let rot = U16Vec3 {
	// 					x: (word >> 4) & 1023,
	// 					y: ((word & 15) << 6) | (word2 >> 10),
	// 					z: word2 & 1023,
	// 				};
	// 				(FrameRotation::All(rot), 2)
	// 			},
	// 			axis => {
	// 				let axis = match axis {
	// 					1 => Axis::X,
	// 					2 => Axis::Y,
	// 					_ => Axis::Z,
	// 				};
	// 				let rot = word & FRAME_SINGLE_ROT_MASK;
	// 				(FrameRotation::Single(axis, rot), 1)
	// 			},
	// 		};
	// 		frame_offset += advance;
	// 		rotations.push(rot);
	// 	}
	// 	Frame { bound_box, offset, rotations }
	// }
}
