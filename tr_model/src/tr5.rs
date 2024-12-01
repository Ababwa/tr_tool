use glam::{IVec3, Vec3};
use shared::min_max::MinMax;
use tr_readable::Readable;
use crate::{tr1::{NumSectors, Portal, RoomFlags, Sector, ATLAS_PIXELS}, tr2::Color16BitArgb, tr3::RoomStaticMesh, tr4::{Color32BitBgra, NumAtlases}};

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Light {
	pub pos: Vec3,
	pub color: Vec3,
	pub unused1: u32,
	pub in_val: f32,
	pub out_val: f32,
	pub radius_in: f32,
	pub radius_out: f32,
	pub range: f32,
	pub direction: Vec3,
	pub pos2: IVec3,
	pub direction2: IVec3,
	pub light_type: u8,
	pub unused2: [u8; 3],
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct FogBulb {
	pub pos: Vec3,
	pub color: Vec3,
	pub unused: u32,
	pub in_val: f32,
	pub out_val: f32,
}

#[repr(C)]
#[derive(Clone, Debug)]
pub struct Layer {
	pub num_vertices: u32,
	pub unused1: u16,
	pub num_quads: u16,
	pub num_tris: u16,
	pub unused2: [u16; 3],
	pub bound_box: MinMax<Vec3>,
	pub unused3: [u32; 4],
}

#[derive(Readable, Clone, Debug)]
pub struct Room {
	pub xela: [u8; 4],
	pub unused1: [u32; 6],
	pub pos1: IVec3,
	pub y_bottom1: i32,
	pub y_top1: i32,
	pub num_sectors: NumSectors,
	pub color: Color32BitBgra,
	pub num_lights: u16,
	pub num_room_static_meshes: u16,
	pub reverb: u8,
	pub flip_group: u8,
	pub water_details: u16,
	pub unused2: [u32; 5],
	pub flip_room_index: u16,
	pub flags: RoomFlags,
	pub unused3: [u32; 5],
	pub pos2: Vec3,
	pub unused4: [u32; 6],
	pub num_tris: u32,
	pub num_quads: u32,
	pub unused5: [u32; 3],
	pub num_fog_bulbs: u32,
	pub y_top2: i32,
	pub y_bottom2: i32,
	pub num_layers: u32,
	pub unused6: [u32; 4],
	pub num_vertex_bytes: u32,
	pub unused7: [u32; 4],
	#[list(num_lights)] pub lights: Box<[Light]>,
	#[list(num_fog_bulbs)] pub fog_bulbs: Box<[FogBulb]>,
	#[list(num_sectors)] pub sectors: Box<[Sector]>,
	#[list(u16)] pub portals: Box<[Portal]>,
	pub unused8: u16,
	#[list(num_room_static_meshes)] pub room_static_meshes: Box<[RoomStaticMesh]>,
	#[list(num_layers)] pub layers: Box<[Layer]>,
}

#[derive(Readable, Clone, Debug)]
pub struct Level {
	pub version: u32,
	pub num_atlases: NumAtlases,
	#[zlib] #[list(num_atlases)] pub atlases_32bit: Box<[[Color32BitBgra; ATLAS_PIXELS]]>,
	#[zlib] #[list(num_atlases)] pub atlases_16bit: Box<[[Color16BitArgb; ATLAS_PIXELS]]>,
	#[zlib] #[boxed] pub misc_images: Box<[[Color32BitBgra; ATLAS_PIXELS]; 3]>,
	pub lara_type: u16,
	pub weather_type: u16,
	pub padding: [u8; 28],
	pub level_data_uncompressed_size: u32,
	pub level_data_compressed_size: u32,
	pub unused: u32,
	//rooms
}
