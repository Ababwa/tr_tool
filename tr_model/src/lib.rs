#[cfg(target_endian = "big")]
const _: () = panic!("big endian not supported");

mod u16_cursor;
pub mod tr1;
pub mod tr2;
pub mod tr3;
// pub mod tr4;

use glam::U16Vec3;

pub use tr_readable::Readable;

//model

macro_rules! decl_box_data {
	($tr_box:ident, $box_data:ident, $scalar:ty, $zone_mult:literal) => {
		#[repr(C)]
		#[derive(Clone, Debug)]
		pub struct $tr_box {
			/// Sectors.
			pub z: MinMax<$scalar>,
			pub x: MinMax<$scalar>,
			pub y: i16,
			pub overlap: u16,
		}
		
		#[derive(Clone, Debug)]
		pub struct $box_data {
			pub boxes: Box<[$tr_box]>,
			pub overlap_data: Box<[u16]>,
			pub zone_data: Box<[u16]>,
		}
		
		impl Readable for $box_data {
			unsafe fn read<R: Read>(reader: &mut R, this: *mut Self) -> Result<()> {
				let num_boxes = read_val_flat::<_, u32>(reader)? as usize;
				read_boxed_slice_flat(reader, addr_of_mut!((*this).boxes), num_boxes)?;
				let num_overlaps = read_val_flat::<_, u32>(reader)? as usize;
				read_boxed_slice_flat(reader, addr_of_mut!((*this).overlap_data), num_overlaps)?;
				read_boxed_slice_flat(reader, addr_of_mut!((*this).zone_data), num_boxes * $zone_mult)?;
				Ok(())
			}
		}
	};
}
pub(crate) use decl_box_data;

//extraction

macro_rules! decl_room_geom {
	($room_geom:ident, $room_vertex:ty, $room_quad:ty, $room_tri:ty, $sprite:ty) => {
		#[derive(Clone, Debug)]
		pub struct $room_geom<'a> {
			pub vertices: &'a [$room_vertex],
			pub quads: &'a [$room_quad],
			pub tris: &'a [$room_tri],
			pub sprites: &'a [$sprite],
		}
		
		impl<'a> $room_geom<'a> {
			fn get(geom_data: &'a [u16]) -> Self {
				let mut cursor = U16Cursor::new(geom_data);
				unsafe {
					Self {
						vertices: cursor.u16_len_slice(),
						quads: cursor.u16_len_slice(),
						tris: cursor.u16_len_slice(),
						sprites: cursor.u16_len_slice(),
					}
				}
			}
		}
	};
}
pub(crate) use decl_room_geom;

macro_rules! decl_mesh1 {
	(
		$mesh:ident, $mesh_lighting:ident, $textured_quad:ty, $textured_tri:ty, $solid_quad:ty,
		$solid_tri:ty
	) => {
		#[derive(Clone, Debug)]
		pub struct $mesh<'a> {
			pub center: I16Vec3,
			pub radius: i32,
			/// If static mesh, relative to `RoomStaticMesh.pos`.
			/// If entity mesh, relative to `Entity.pos`.
			pub vertices: &'a [I16Vec3],
			pub lighting: $mesh_lighting<'a>,
			pub textured_quads: &'a [$textured_quad],
			pub textured_tris: &'a [$textured_tri],
			pub solid_quads: &'a [$solid_quad],
			pub solid_tris: &'a [$solid_tri],
		}
		
		impl<'a> $mesh<'a> {
			pub(crate) fn get(mesh_data: &'a [u16], mesh_offset: u32) -> Self {
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
						textured_quads: cursor.u16_len_slice(),
						textured_tris: cursor.u16_len_slice(),
						solid_quads: cursor.u16_len_slice(),
						solid_tris: cursor.u16_len_slice(),
					}
				}
			}
		}
	};
}
pub(crate) use decl_mesh1;

fn get_packed_angles(xy: u16, yz: u16) -> U16Vec3 {
	U16Vec3 {
		x: (xy >> 4) & 1023,
		y: ((xy & 15) << 6) | (yz >> 10),
		z: yz & 1023,
	}
}
