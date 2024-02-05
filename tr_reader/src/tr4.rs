use std::{io::{Read, Result, Cursor}, slice};
use bitfield::bitfield;
use byteorder::{ReadBytesExt, LE};
use crate::{Readable, read_boxed_slice, read_list, get_zlib};

pub const IMG_DIM: usize = 256;
pub const NUM_PIXELS: usize = IMG_DIM * IMG_DIM;

pub struct Images {
	pub num_room_images: u16,
	pub num_obj_images: u16,
	pub num_bump_maps: u16,
	pub images32: Box<[[u8; NUM_PIXELS * 4]]>,
	pub images16: Box<[[u8; NUM_PIXELS * 2]]>,
	pub misc_images: Box<[[u8; NUM_PIXELS * 4]; 2]>,
}

fn read_big_zlib<R: Read, const N: usize>(reader: &mut R, len: usize) -> Result<Box<[[u8; N]]>> {
	let mut reader = get_zlib(reader)?;
	let mut vec = Vec::with_capacity(len);
	unsafe {//no safe way to do this currently
		vec.set_len(len);
		let buf = slice::from_raw_parts_mut(vec.as_mut_ptr() as *mut u8, len * N);
		reader.read_exact(buf)?;
	}
	Ok(vec.into_boxed_slice())
}

impl Readable for Images {
    fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let num_room_images = reader.read_u16::<LE>()?;
        let num_obj_images = reader.read_u16::<LE>()?;
        let num_bump_maps = reader.read_u16::<LE>()?;
		let num_images = (num_room_images + num_obj_images + num_bump_maps) as usize;
		let images32 = read_big_zlib(reader, num_images)?;
		let images16 = read_big_zlib(reader, num_images)?;
		let misc_images = read_big_zlib(reader, 2)?.try_into().ok().unwrap();//exactly 2
		Ok(Images {
			num_room_images,
			num_obj_images,
			num_bump_maps,
			images32,
			images16,
			misc_images,
		})
    }
}

#[derive(Readable, Clone, Copy)]
pub struct Vertex<T: Readable> {
	pub x: T,
	pub y: T,
	pub z: T,
}

#[derive(Readable, Clone, Copy)]
pub struct RoomVertex {
	pub vertex: Vertex<i16>,//relative to Room
	#[skip_2]
	pub flags: u16,
	pub color: u16,
}

bitfield! {
	#[derive(Readable, Clone, Copy)]
	pub struct RoomFaceTextureDetails(u16);
	pub texture_id, _: 14, 0;//id into LevelData.object_textures
	pub double_sided, _: 15;
}

#[derive(Readable, Clone, Copy)]
pub struct RoomFace<const N: usize> {
	pub vertex_ids: [u16; N],//id into Room.vertices
	pub texture_details: RoomFaceTextureDetails,
}

#[derive(Readable, Clone, Copy)]
pub struct Sprite {
	pub vertex_id: u16,//id into Room.vertices
	pub texture_id: u16,//id into LevelData.sprite_textures
}

#[derive(Readable, Clone, Copy)]
pub struct Portal {
	pub adjoining_room_id: u16,//id into LevelData.rooms
	pub normal: Vertex<i16>,
	pub vertices: [Vertex<i16>; 4],//relative to Room
}

bitfield! {
	#[derive(Readable, Clone, Copy)]
	pub struct SectorMaterialAndBox(u16);
	pub material, _: 3, 0;//footstep sound
	pub box_id, _: 14, 4;//id into BoxData.boxes
}

#[derive(Readable, Clone, Copy)]
pub struct Sector {
	pub floor_data_id: u16,//id into LevelData.floor_data
	pub material_and_box: SectorMaterialAndBox,
	pub room_below_id: u8,//id into LevelData.Rooms, 255 = none
	pub floor: i8,
	pub room_above_id: u8,//id into LevelData.Rooms, 255 = none
	pub ceiling: i8,
}

#[derive(Readable, Clone, Copy)]
pub struct Light {
	pub pos: Vertex<i32>,//world coords
	pub r: u8,//color
	pub g: u8,
	pub b: u8,
	pub light_type: u8,
	#[skip_1]
	pub intensity: u8,
	pub hotspot: f32,
	pub falloff: f32,
	pub length: f32,
	pub cutoff: f32,
	pub direction: Vertex<f32>,//direction
}

#[derive(Readable, Clone, Copy)]
pub struct RoomStaticMesh {
	pub pos: Vertex<i32>,//world coords
	pub rotation: u16,//units are 1/65536th of a rotation
	pub color: u16,
	#[skip_2]
	pub static_mesh_id: u16,//id into LevelData.static_meshes
}

pub struct Sectors {
	pub sectors_x: u16,
	pub sectors_y: u16,
	pub sectors: Box<[Sector]>,
}

impl Readable for Sectors {
    fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let sectors_x = reader.read_u16::<LE>()?;
		let sectors_y = reader.read_u16::<LE>()?;
		let sectors = read_boxed_slice(reader, (sectors_x * sectors_y) as usize)?;
		Ok(Sectors { sectors_x, sectors_y, sectors })
    }
}

#[derive(Readable)]
pub struct Room {
	pub x: i32,//world coords
	pub z: i32,
	pub y_bottom: i32,
	pub y_top: i32,
	#[skip_4]
	#[list_u16]
	pub vertices: Box<[RoomVertex]>,
	#[list_u16]
	pub quads: Box<[RoomFace<4>]>,
	#[list_u16]
	pub triangles: Box<[RoomFace<3>]>,
	#[list_u16]
	pub sprites: Box<[Sprite]>,
	#[list_u16]
	pub portals: Box<[Portal]>,
	pub sectors: Sectors,
	pub color: u32,//argb
	#[list_u16]
	pub lights: Box<[Light]>,
	#[list_u16]
	pub room_static_meshes: Box<[RoomStaticMesh]>,
	pub flip_room_id: u16,//id into LevelData.rooms, 65535 = none
	pub flags: u16,
	pub water_effect: u8,
	pub reverb: u8,
	pub flip_group: u8,
}

pub struct Meshes {
	pub meshes: Box<[Mesh]>,
	pub pointers: Box<[u32]>,
}

impl Readable for Meshes {
    fn read<R: Read>(reader: &mut R) -> Result<Self> {
        let num_mesh_bytes = 2 * reader.read_u32::<LE>()? as u64;
		let mesh_bytes = read_boxed_slice::<_, u8>(reader, num_mesh_bytes as usize)?;
		let pointers = read_list::<_, _, u32>(reader)?;
		let mut mesh_vec = Vec::with_capacity(pointers.len());
		let mut mesh_bytes_cursor = Cursor::new(mesh_bytes);
		while mesh_bytes_cursor.position() < num_mesh_bytes {
			mesh_vec.push(Mesh::read(&mut mesh_bytes_cursor)?);
			let pos = mesh_bytes_cursor.position();
			mesh_bytes_cursor.set_position(pos + ((4 - (pos % 4)) % 4));//4-align position
		}
		let meshes = mesh_vec.into_boxed_slice();
		Ok(Meshes { meshes, pointers })
    }
}

pub enum MeshComponent {
	Normals(Box<[Vertex<i16>]>),
	Lights(Box<[u16]>),
}

impl Readable for MeshComponent {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		let num = reader.read_i16::<LE>()?;
		Ok(if num > 0 {
			MeshComponent::Normals(read_boxed_slice(reader, num as usize)?)
		} else {
			MeshComponent::Lights(read_boxed_slice(reader, (-num) as usize)?)
		})
	}
}

#[derive(Readable, Clone, Copy)]
pub struct MeshFace<const N: usize> {
	pub vertex_ids: [u16; N],//id into Room.vertices
	pub texture_id: u16,//id into LevelData.object_textures
	pub light_effects: u16,
}

#[derive(Readable)]
pub struct Mesh {
	pub center: Vertex<i16>,
	pub radius: i32,
	#[list_u16]
	pub vertices: Box<[Vertex<i16>]>,
	pub component: MeshComponent,
	#[list_u16]
	pub quads: Box<[MeshFace<4>]>,
	#[list_u16]
	pub triangles: Box<[MeshFace<3>]>,
}

#[derive(Readable, Clone, Copy)]
pub struct Anim {
	pub frame_offset: u32,//byte offset into LevelData.frames
	pub frame_duration: u8,//30ths of a second
	pub num_frames: u8,
	pub state: u16,
	pub speed: u32,//fixed-point
	pub accel: u32,//fixed-point
	pub lateral_speed: u32,//fixed-point
	pub lateral_accel: u32,//fixed-point
	pub frame_start: u16,
	pub frame_end: u16,
	pub next_anim: u16,
	pub next_frame: u16,
	pub num_state_changes: u16,
	pub state_change_id: u16,//id into LevelData.state_changes
	pub num_anim_commands: u16,
	pub anim_command_id: u16,//id into LevelData.anim_commands
}

#[derive(Readable, Clone, Copy)]
pub struct StateChange {
	pub state: u16,
	pub num_anim_dispatches: u16,
	pub anim_dispatch_id: u16,//id into LevelData.anim_dispatches
}

#[derive(Readable, Clone, Copy)]
pub struct AnimDispatch {
	pub low_frame: u16,
	pub high_frame: u16,
	pub next_anim_id: u16,//id into LevelData.animations
	pub next_frame_id: u16,//id into LevelData.frames
}

#[derive(Readable, Clone, Copy)]
pub struct MeshNode {
	pub flags: u8,
	pub x: i8,//relative to parent
	pub y: i8,
	pub z: i8,
}

#[derive(Readable, Clone, Copy)]
pub struct Model {
	pub id: u32,
	pub num_meshes: u16,
	pub mesh_id: u16,//id into LevelData.mesh_data.mesh_pointers
	pub mesh_node_id: u32,//id into LevelData.mesh_nodes
	pub frame_offset: u32,//byte offset into LevelData.frames
	pub anim_id: u16,//id into LevelData.animations, 65536 = none
}

#[derive(Readable, Clone, Copy)]
pub struct BoundBox {
	pub x_min: i16,
	pub x_max: i16,
	pub y_min: i16,
	pub y_max: i16,
	pub z_min: i16,
	pub z_max: i16,
}

#[derive(Readable, Clone, Copy)]
pub struct StaticMesh {
	pub id: u32,
	pub mesh_id: u16,//id into LevelData.mesh_data.mesh_pointers
	pub visibility: BoundBox,
	pub collision: BoundBox,
	pub flags: u16,//unused
}

#[derive(Readable, Clone, Copy)]
pub struct SpriteTexture {
	pub atlas: u16,
	#[skip_2]
	pub width: u16,
	pub height: u16,
	pub left: i16,
	pub top: i16,
	pub right: i16,
	pub bottom: i16,
}

#[derive(Readable, Clone, Copy)]
pub struct SpriteSequence {
	pub sprite_id: u32,
	pub neg_length: i16,
	pub offset: u16,
}

#[derive(Readable, Clone, Copy)]
pub struct Camera {
	pub pos: Vertex<i32>,//world coords
	pub room_id: u16,//id into LevelData.rooms
	pub flags: u16,
}

#[derive(Readable, Clone, Copy)]
pub struct FlybyCamera {
	pub pos: Vertex<i32>,//world coords
	pub direction: Vertex<i32>,
	pub chain: u8,
	pub index: u8,
	pub fov: u16,
	pub roll: i16,
	pub timer: u16,
	pub speed: u16,
	pub flags: u16,
	pub room_id: u32,//id into LevelData.rooms
}

#[derive(Readable, Clone, Copy)]
pub struct SoundSource {
	pub pos: Vertex<i32>,//world coords
	pub sound_id: u16,
	pub flags: u16,
}

#[derive(Readable, Clone, Copy)]
pub struct TrBox {
	pub z_min: u8,//sectors
	pub z_max: u8,
	pub x_min: u8,
	pub x_max: u8,
	pub y: i16,
	pub overlap: u16,
}

pub struct BoxData {
	pub boxes: Box<[TrBox]>,
	pub overlaps: Box<[u16]>,
	pub zones: Box<[u16]>,
}

impl Readable for BoxData {
	fn read<R: Read>(reader: &mut R) -> Result<Self> {
		let num_boxes = reader.read_u32::<LE>()? as usize;
		let boxes = read_boxed_slice(reader, num_boxes)?;
		let num_overlaps = reader.read_u32::<LE>()? as usize;
		let overlaps = read_boxed_slice(reader, num_overlaps)?;
		let zones = read_boxed_slice(reader, num_boxes * 10)?;
		Ok(BoxData { boxes, overlaps, zones })
	}
}

#[derive(Readable, Clone, Copy)]
pub struct ObjectTextureVertex {
	pub x: u16,//fixed-point
	pub y: u16,
}

bitfield! {
	#[derive(Readable, Clone, Copy)]
	pub struct ObjectTextureAtlasAndTriangle(u16);
	pub atlas_id, _: 14, 0;//id into Images.images (16 or 32)
	pub triangle, _: 15;
}

bitfield! {
	#[derive(Readable, Clone, Copy)]
	pub struct ObjectTextureDetails(u16);
	pub mapping_correction, _: 2, 0;
	pub bump_mapping, _: 10, 9;
	pub room_texture, _: 15;
}

#[derive(Readable, Clone, Copy)]
pub struct ObjectTexture {
	pub blend_mode: u16,
	pub atlas_and_triangle: ObjectTextureAtlasAndTriangle,
	pub details: ObjectTextureDetails,
	pub vertices: [ObjectTextureVertex; 4],
	#[skip_8]
	pub width: u32,
	pub height: u32,
}

#[derive(Readable, Clone, Copy)]
pub struct Entity {
	pub model_id: u16,//matched to Model.id
	pub room_id: u16,//id into LevelData.rooms
	pub pos: Vertex<i32>,//world coords
	pub angle: i16,
	pub light_intensity: u16,//65535 = use mesh light
	pub ocb: u16,
	pub flags: u16,
}

#[derive(Readable, Clone, Copy)]
pub struct Ai {
	pub model_id: u16,//matched to Model.id
	pub room_id: u16,//id into LevelData.rooms
	pub pos: Vertex<i32>,//world coords
	pub ocb: u16,
	pub flags: u16,
	pub angle: i32,
}

#[derive(Readable, Clone, Copy)]
pub struct SoundDetail {
	#[skip_2]
	pub volume: u8,
	pub range: u8,//in sectors
	pub chance: u8,
	pub pitch: u8,
	pub flags: u16,
}

#[derive(Readable)]
pub struct LevelData {
	#[skip_4]
	#[list_u16]
	pub rooms: Box<[Room]>,
	#[list_u32]
	pub floor_data: Box<[u16]>,
	pub meshes: Meshes,
	#[list_u32]
	pub animations: Box<[Anim]>,
	#[list_u32]
	pub state_changes: Box<[StateChange]>,
	#[list_u32]
	pub anim_dispatches: Box<[AnimDispatch]>,
	#[list_u32]
	pub anim_commands: Box<[u16]>,
	#[list_u32]
	pub mesh_nodes: Box<[MeshNode]>,
	#[list_u32]
	pub frames: Box<[u16]>,
	#[list_u32]
	pub models: Box<[Model]>,
	#[list_u32]
	pub static_meshes: Box<[StaticMesh]>,
	pub spr: [u8; 3],
	#[list_u32]
	pub sprite_textures: Box<[SpriteTexture]>,
	#[list_u32]
	pub sprite_sequences: Box<[SpriteSequence]>,
	#[list_u32]
	pub cameras: Box<[Camera]>,
	#[list_u32]
	pub flyby_cameras: Box<[FlybyCamera]>,
	#[list_u32]
	pub sound_sources: Box<[SoundSource]>,
	pub box_data: BoxData,
	#[list_u32]
	pub animated_textures: Box<[u16]>,
	pub animated_textures_uv_count: u8,
	pub tex: [u8; 3],
	#[list_u32]
	pub object_textures: Box<[ObjectTexture]>,
	#[list_u32]
	pub entities: Box<[Entity]>,
	#[list_u32]
	pub ais: Box<[Ai]>,
	#[list_u16]
	pub demo_data: Box<[u8]>,
	pub sound_map: Box<[u16; 370]>,
	#[list_u32]
	pub sound_details: Box<[SoundDetail]>,
	#[list_u32]
	pub sample_indices: Box<[u32]>,
	pub zero: [u8; 6],
}

#[derive(Readable)]
pub struct Sample {
	pub uncompressed: u32,
	#[list_u32]
	pub data: Box<[u8]>,
}

#[derive(Readable)]
pub struct Level {
	pub version: u32,
	pub images: Images,
	#[zlib]
	pub level_data: LevelData,
	#[list_u32]
	pub samples: Box<[Sample]>,
}
