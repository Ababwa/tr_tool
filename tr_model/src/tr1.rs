use std::io::{Cursor, Error, ErrorKind, Read, Result};
use bitfield::bitfield;
use byteorder::{ReadBytesExt, LE};
use glam::{I16Vec2, I16Vec3, IVec3, U16Vec2, U16Vec3};
use glam_traits::ext::U8Vec2;
use nonmax::{NonMaxU16, NonMaxU8};
use shared::MinMax;
use tr_readable::{read_boxed_slice, read_list, Readable};

pub const ATLAS_SIZE: usize = 256;
pub const NUM_PIXELS: usize = ATLAS_SIZE * ATLAS_SIZE;
pub const PALETTE_SIZE: usize = 256;
pub const SOUND_MAP_SIZE: usize = 256;
pub const LIGHT_MAP_SIZE: usize = 32;
pub const ZONE_MULT: usize = 6;

//model

#[repr(C)]
#[derive(Readable, Clone, Copy)]
pub struct RoomVertex {
	/// Relative to room
	pub vertex: I16Vec3,
	pub light: u16,
}

#[repr(C)]
#[derive(Readable, Clone, Copy)]
pub struct Face<const N: usize> {
	/// If room face, index into Room.vertices
	pub vertex_indices: [u16; N],
	/// If textured, index into Level.object_textures
	/// If solid, index into Level.palette
	pub texture_index: u16,
}

pub type Quad = Face<4>;
pub type Tri = Face<3>;

#[repr(C)]
#[derive(Readable, Clone, Copy)]
pub struct Sprite {
	/// Index into Room.vertices
	pub vertex_index: u16,
	/// Index into Level.sprite_textures
	pub texture_index: u16,
}

#[repr(C)]
#[derive(Readable, Clone, Copy)]
pub struct Portal {
	/// Index into Level.rooms
	pub adjoining_room_index: u16,
	pub normal: I16Vec3,
	/// Relative to room
	pub vertices: [I16Vec3; 4],
}

#[repr(C)]
#[derive(Readable, Clone, Copy)]
pub struct Sector {
	/// Index into Level.floor_data
	pub floor_data_index: u16,
	/// Index into BoxData.boxes
	pub box_index: Option<NonMaxU16>,
	/// Index into Level.rooms
	pub room_below_id: Option<NonMaxU8>,
	pub floor: i8,
	/// Index into Level.rooms
	pub room_above_index: Option<NonMaxU8>,
	pub ceiling: i8,
}

#[derive(Clone)]
pub struct Sectors {
	pub num_sectors: U16Vec2,
	pub sectors: Box<[Sector]>,
}

impl Readable for Sectors {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		let num_sectors = U16Vec2::read(reader)?;
		let sectors = read_boxed_slice(reader, num_sectors.element_product() as usize)?;
		Ok(Self { num_sectors, sectors })
	}
}

#[repr(C)]
#[derive(Readable, Clone, Copy)]
pub struct Light {
	pub pos: IVec3,
	pub brightness: u16,
	pub fallout: u32,
}

#[repr(C)]
#[derive(Readable, Clone, Copy)]
pub struct RoomStaticMesh {
	/// World coords
	pub pos: IVec3,
	/// Units are 1/65536 of a rotation
	pub rotation: u16,
	pub color: u16,
	/// Matched to StaticMesh.id in Level.static_meshes
	pub static_mesh_id: u16,
}

bitfield! {
	#[derive(Readable, Clone, Copy)]
	pub struct RoomFlags(u16);
	pub water, _: 0;
}

#[derive(Readable, Clone)]
pub struct Room {
	/// World coord
	pub x: i32,
	/// World coord
	pub z: i32,
	pub y_bottom: i32,
	pub y_top: i32,
	#[skip(4)]
	#[list(u16)]
	pub vertices: Box<[RoomVertex]>,
	#[list(u16)]
	pub quads: Box<[Quad]>,
	#[list(u16)]
	pub tris: Box<[Tri]>,
	#[list(u16)]
	pub sprites: Box<[Sprite]>,
	#[list(u16)]
	pub portals: Box<[Portal]>,
	pub sectors: Sectors,
	pub ambient_light: u16,
	#[list(u16)]
	pub lights: Box<[Light]>,
	#[list(u16)]
	pub room_static_meshes: Box<[RoomStaticMesh]>,
	/// Index into Level.rooms
	pub flip_room_index: Option<NonMaxU16>,
	pub flags: RoomFlags,
}

#[repr(C)]
#[derive(Readable, Clone, Copy)]
pub struct Animation {
	/// Byte offset into Level.frame_data
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
	pub state_change_id: u16,
	pub num_anim_commands: u16,
	pub anim_command_id: u16,
}

#[repr(C)]
#[derive(Readable, Clone, Copy)]
pub struct StateChange {
	pub state: u16,
	pub num_anim_dispatches: u16,
	pub anim_dispatch_id: u16,
}

#[repr(C)]
#[derive(Readable, Clone, Copy)]
pub struct AnimDispatch {
	pub low_frame: u16,
	pub high_frame: u16,
	pub next_anim_id: u16,
	pub next_frame_id: u16,
}

#[repr(C)]
#[derive(Readable, Clone, Copy)]
pub struct Model {
	pub id: u32,
	pub num_meshes: u16,
	/// Index into Level.mesh_offsets
	pub mesh_offset_index: u16,
	/// Offset into Level.mesh_node_data
	pub mesh_node_offset: u32,
	/// Byte offset into Level.frame_data
	pub frame_byte_offset: u32,
	/// Index into Level.animations
	pub anim_index: Option<NonMaxU16>,
}

#[repr(C)]
#[derive(Readable, Clone, Copy)]
pub struct BoundBox {
	pub x: MinMax<i16>,
	pub y: MinMax<i16>,
	pub z: MinMax<i16>,
}

#[repr(C)]
#[derive(Readable, Clone, Copy)]
pub struct StaticMesh {
	pub id: u32,
	/// Index into Level.mesh_offsets
	pub mesh_offset_index: u16,
	pub visibility: BoundBox,
	pub collision: BoundBox,
	pub flags: u16,
}

#[repr(u16)]//makes ObjectTexture match file format byte-for-byte
#[derive(Clone, Copy)]
pub enum BlendMode {
	Opaque,
	Test,
	Add,
}

impl Readable for BlendMode {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		Ok(match reader.read_u16::<LE>()? {
			0 => BlendMode::Opaque,
			1 => BlendMode::Test,
			2 => BlendMode::Add,
			_ => return Err(Error::new(ErrorKind::InvalidData, "invalid blend mode")),
		})
	}
}

#[repr(C)]
#[derive(Readable, Clone, Copy)]
pub struct ObjectTexture {
	pub blend_mode: BlendMode,
	/// Index into Level.atlases
	pub atlas_index: u16,
	/// Units are 1/256 of a pixel
	pub vertices: [U16Vec2; 4],
}

#[repr(C)]
#[derive(Readable, Clone, Copy)]
pub struct SpriteTexture {
	/// Index into Level.atlases
	pub atlas_index: u16,
	pub pos: U8Vec2,
	pub size: U16Vec2,
	pub world_bounds: [I16Vec2; 2],
}

#[repr(C)]
#[derive(Readable, Clone, Copy)]
pub struct SpriteSequence {
	pub id: u32,
	pub neg_length: i16,
	/// Index into Level.sprite_textures
	pub sprite_index: u16,
}

#[repr(C)]
#[derive(Readable, Clone, Copy)]
pub struct Camera {
	/// World coords
	pub pos: IVec3,
	/// Index into Level.rooms
	pub room_index: u16,
	pub flags: u16,
}

#[repr(C)]
#[derive(Readable, Clone, Copy)]
pub struct SoundSource {
	/// World coords
	pub pos: IVec3,
	pub sound_id: u16,
	pub flags: u16,
}

#[repr(C)]
#[derive(Readable, Clone, Copy)]
pub struct TrBox {
	/// Sectors
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
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		let boxes = read_list::<_, _, u32>(reader)?;
		let overlap_data = read_list::<_, _,u32>(reader)?;
		let zone_data = read_boxed_slice(reader, boxes.len() * ZONE_MULT)?;
		Ok(Self { boxes, overlap_data, zone_data })
	}
}

#[repr(C)]
#[derive(Readable, Clone, Copy)]
pub struct Entity {
	/// Matched to Model.id in Level.models
	pub model_id: u16,
	/// Index into Level.rooms
	pub room_index: u16,
	/// World coords
	pub pos: IVec3,
	/// Units are 1/65536th of a rotation
	pub rotation: u16,
	/// If None, use mesh light
	pub brightness: Option<NonMaxU16>,
	pub flags: u16,
}

#[repr(C)]
#[derive(Readable, Clone, Copy)]
pub struct Color3 {
	pub r: u8,
	pub g: u8,
	pub b: u8,
}

#[repr(C)]
#[derive(Readable, Clone, Copy)]
pub struct CinematicFrame {
	pub target: I16Vec3,
	pub pos: I16Vec3,
	pub fov: i16,
	pub roll: i16,
}

#[repr(C)]
#[derive(Readable, Clone, Copy)]
pub struct SoundDetails {
	/// Index into Level.sample_indices
	pub sample_index: u16,
	pub volume: u16,
	pub chance: u16,
	pub details: u16,
}

#[derive(Readable)]
pub struct Level {
	pub version: u32,
	#[flat]
	#[list(u32)]
	pub atlases: Box<[[u8; NUM_PIXELS]]>,
	#[skip(4)]
	#[list(u16)]
	pub rooms: Box<[Room]>,
	#[list(u32)]
	pub floor_data: Box<[u16]>,
	#[list(u32)]
	pub mesh_data: Box<[u16]>,
	/// Byte offsets into Level.mesh_data
	#[list(u32)]
	pub mesh_offsets: Box<[u32]>,
	#[list(u32)]
	pub animations: Box<[Animation]>,
	#[list(u32)]
	pub state_changes: Box<[StateChange]>,
	#[list(u32)]
	pub anim_dispatches: Box<[AnimDispatch]>,
	#[list(u32)]
	pub anim_commands: Box<[u16]>,
	#[list(u32)]
	pub mesh_node_data: Box<[u32]>,
	#[list(u32)]
	pub frame_data: Box<[u16]>,
	#[list(u32)]
	pub models: Box<[Model]>,
	#[list(u32)]
	pub static_meshes: Box<[StaticMesh]>,
	#[list(u32)]
	pub object_textures: Box<[ObjectTexture]>,
	#[list(u32)]
	pub sprite_textures: Box<[SpriteTexture]>,
	#[list(u32)]
	pub sprite_sequences: Box<[SpriteSequence]>,
	#[list(u32)]
	pub cameras: Box<[Camera]>,
	#[list(u32)]
	pub sound_sources: Box<[SoundSource]>,
	pub box_data: BoxData,
	#[list(u32)]
	pub animated_textures: Box<[u16]>,
	#[list(u32)]
	pub entities: Box<[Entity]>,
	#[flat]
	pub light_map: Box<[[u8; PALETTE_SIZE]; LIGHT_MAP_SIZE]>,
	pub palette: Box<[Color3; PALETTE_SIZE]>,
	#[list(u16)]
	pub cinematic_frames: Box<[CinematicFrame]>,
	#[list(u16)]
	pub demo_data: Box<[u8]>,
	pub sound_map: Box<[u16; SOUND_MAP_SIZE]>,
	#[list(u32)]
	pub sound_details: Box<[SoundDetails]>,
	#[list(u32)]
	pub sample_data: Box<[u8]>,
	#[list(u32)]
	pub sample_indices: Box<[u32]>,
}

//extraction

#[derive(Clone)]
pub enum MeshLighting {
	Normals(Box<[I16Vec3]>),
	Lights(Box<[u16]>),
}

impl Readable for MeshLighting {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		Ok(match reader.read_i16::<LE>()? {
			num if num > 0 => Self::Normals(read_boxed_slice(reader, num as usize)?),
			num => Self::Lights(read_boxed_slice(reader, (-num) as usize)?),
		})
	}
}

#[derive(Readable, Clone)]
pub struct Mesh {
	pub center: I16Vec3,
	pub radius: i32,
	/// If static mesh, relative to RoomStaticMesh.pos
	#[list(u16)]
	pub vertices: Box<[I16Vec3]>,
	pub lighting: MeshLighting,
	#[list(u16)]
	pub textured_quads: Box<[Quad]>,
	#[list(u16)]
	pub textured_tris: Box<[Tri]>,
	#[list(u16)]
	pub solid_quads: Box<[Quad]>,
	#[list(u16)]
	pub solid_tris: Box<[Tri]>,
}

bitfield! {
	#[derive(Readable, Clone, Copy)]
	pub struct MeshNodeFlags(u32);
	pub pop, _: 0;
	pub push, _: 1;
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct MeshNode {
	pub flags: MeshNodeFlags,
	/// Relative to parent
	pub offset: IVec3,
}

#[derive(Clone, Copy)]
pub enum Axis {
	X,
	Y,
	Z,
}

#[derive(Clone, Copy)]
pub enum FrameRotation {
	Single(Axis, u16),
	/// All values are
	All(U16Vec3),
}

//TODO: make unsized, get_frame returns ref to memory in-place
#[derive(Clone)]
pub struct Frame {
	pub bound_box: MinMax<I16Vec3>,
	pub offset: I16Vec3,
	pub rotations: Vec<FrameRotation>,
}

impl Level {
	pub fn read_mesh_from_offset(&self, offset: u32) -> Result<Mesh> {
		let mesh_bytes = unsafe { reinterpret::slice(&self.mesh_data) };//safe: [u16] to [u8]
		let mesh_bytes = &mesh_bytes[offset as usize..];
		Mesh::read(&mut Cursor::new(mesh_bytes))
	}
	
	pub fn read_mesh_from_offset_index(&self, offset_index: u16) -> Result<Mesh> {
		self.read_mesh_from_offset(self.mesh_offsets[offset_index as usize])
	}
	
	/// Should be called with Model.num_meshes - 1
	pub fn get_mesh_nodes(&self, mesh_node_offset: u32, num_meshes: u16) -> &[MeshNode] {
		let lo = mesh_node_offset as usize;
		let hi = lo + num_meshes as usize * 4;
		unsafe { reinterpret::slice(&self.mesh_node_data[lo..hi]) }//safe: same alignment, pod output
	}
	
	pub fn get_frame(&self, single_rot_mask: u16, frame_byte_offset: u32, num_meshes: u16) -> Frame {
		let frame_offset = frame_byte_offset as usize / 2;
		let &(bound_box, offset) = unsafe {
			reinterpret::slice_to_ref(&self.frame_data[frame_offset..][..9])
		};//safe: same alignment, pod output
		let mut rotations = Vec::with_capacity(num_meshes as usize);
		let mut frame_offset = frame_offset + 9;
		for _ in 0..num_meshes {
			let word = self.frame_data[frame_offset];
			let (rot, advance) = match word >> 14 {
				0 => {
					let word2 = self.frame_data[frame_offset + 1];
					let rot = U16Vec3 {
						x: (word >> 4) & 1023,
						y: ((word & 15) << 6) | (word2 >> 10),
						z: word2 & 1023,
					};
					(FrameRotation::All(rot), 2)
				},
				axis => {
					let axis = match axis {
						1 => Axis::X,
						2 => Axis::Y,
						_ => Axis::Z,
					};
					let rot = word & single_rot_mask;
					(FrameRotation::Single(axis, rot), 1)
				},
			};
			frame_offset += advance;
			rotations.push(rot);
		}
		Frame { bound_box, offset, rotations }
	}
}
