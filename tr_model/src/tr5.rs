use glam::{IVec3, U16Vec2, Vec3};
use tr_readable::Readable;
use crate::{tr1::RoomFlags, tr4::Color32BitBgra};

#[derive(Readable, Clone, Debug)]
pub struct Room {
	pub xela: [u8; 4],
	pub unused1: [u32; 6],
	pub pos1: IVec3,
	pub y_bottom1: i32,
	pub y_top1: i32,
	pub sectors_size: U16Vec2,
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
}

#[derive(Readable, Clone, Debug)]
pub struct Level {
	pub version: u32,
	#[delegate] pub atlases: Atlases,
	pub lara_type: u16,
	pub weather_type: u16,
	pub padding: [u8; 28],
	pub level_data_uncompressed_size: u32,
	pub level_data_compressed_size: u32,
	pub unused: u32,
}
