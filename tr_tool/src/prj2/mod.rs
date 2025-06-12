mod enums;
mod write;
mod floor_ceiling;
mod walls;

use core::str;
use std::{io::{Error, Result, Write}, path::{self, Path}};
use enums::*;
use floor_ceiling::{ConnectedTris, SingleTri, SteppedTris, SurfaceType, Tris};
use glam::{I16Vec3, Vec3};
use tr_model::tr1;
use write::{Leb128, WriteExt};
use crate::{as_bytes::AsBytes, palette_images_to_rgba, tr_traits::*};

const PRJ2: &[u8; 4] = b"PRJ2";

type V<'a> = &'a mut Vec<u8>;

fn room_type(room_flags: tr1::RoomFlags) -> RoomType {
	if room_flags.water() {
		RoomType::Water
	} else {
		RoomType::Normal
	}
}

const SECTOR_FLAGS_CHUNK_ID: &[u8] = &[0];
const SECTOR_FLOOR_CHUNK_ID: &[u8] = &[7];
const SECTOR_CEILING_CHUNK_ID: &[u8] = &[8];
const SECTOR_FLOOR_DIVISIONS_CHUNK_ID: &[u8] = &[9];
const SECTOR_CEILING_DIVISIONS_CHUNK_ID: &[u8] = &[10];

fn snap_vert(v: I16Vec3) -> Option<I16Vec3> {
	const DIVISOR: I16Vec3 = I16Vec3::new(1024, 256, 1024);
	I16Vec3::ZERO.cmpeq(v % DIVISOR).all().then_some(v / DIVISOR)
}

struct SnappedFace<const N: usize> {
	vertices: [I16Vec3; N],
	object_texture_index: u16,
}

fn snap_face_verts<const N: usize>(
	vertices: &[Option<I16Vec3>], face_vertex_indices: [u16; N],
) -> Option<[I16Vec3; N]> {
	let mut face_vertices = [I16Vec3::ZERO; N];
	for i in 0..N {
		face_vertices[i] = vertices[face_vertex_indices[i] as usize]?;
	}
	Some(face_vertices)
}

fn snap_face<const N: usize>(
	snapped_verts: &[Option<I16Vec3>], face_vertex_indices: [u16; N], object_texture_index: u16,
) -> Option<SnappedFace<N>> {
	Some(SnappedFace {
		vertices: snap_face_verts(&snapped_verts, face_vertex_indices)?,
		object_texture_index,
	})
}

// fn get_dir(verts: &[I16Vec3], dim: usize) -> i16 {
// 	let d1 = (dim + 1) % 3;
// 	let d2 = (dim + 2) % 3;
// 	let a1 = verts[1][d1] - verts[0][d1];
// 	let a2 = verts[1][d2] - verts[0][d2];
// 	let b1 = verts[2][d1] - verts[0][d1];
// 	let b2 = verts[2][d2] - verts[0][d2];
// 	(a2 * b1 - a1 * b2).signum()
// }

/*
x increases east
y increases down
z increases north
*/

struct SnappedFaceRef<'a> {
	vertices: &'a [I16Vec3],
	object_texture_index: u16,
}

impl<'a, const N: usize> From<&'a SnappedFace<N>> for SnappedFaceRef<'a> {
	fn from(face: &'a SnappedFace<N>) -> Self {
		Self {
			vertices: &face.vertices,
			object_texture_index: face.object_texture_index,
		}
	}
}



const XN: usize = 0;
const ZP: usize = 1;
const XP: usize = 2;
const ZN: usize = 3;
// const XNZP: usize = 4;
// const XPZP: usize = 5;
// const XPZN: usize = 6;
// const XNZN: usize = 7;



// struct TextureToAdd {
// 	sector_face: SectorFace,
// 	object_texture_index: u16,
// }

// const CW: isize = -1;
// const CCW: isize = 1;

/*
side indices:
+1+
0 2
+3+
corner indices:
0-1
| |
3-2
*/

pub fn export<W: Write, L: Level>(w: &mut W, level: &L, wad_path: &str, textures_path: &str) -> Result<()> {
	let wad_path = path::absolute(wad_path)?;
	let textures_path = path::absolute(textures_path)?;
	let rgba = palette_images_to_rgba(&level.palette_24bit().unwrap(), &level.atlases_palette().unwrap());
	let height = rgba.len() as u32 / 1024;
	image::save_buffer(&textures_path, &rgba, 256, height, image::ColorType::Rgba8).map_err(Error::other)?;
	w.write_all(PRJ2)?;
	w.write_all(&[0; 4])?;
	w.chunk_stream(|mut s| {
		s.chunk(b"TeSettings", |c: V| {
			c.chunk_stream(|mut s| {
				s.chunk(b"TeGameVersion", Leb128(GameVersion::Tr1))?;
				s.chunk(b"TeSoundSystem", Leb128(SoundSystem::Xml))?;
				s.chunk(b"TeWads", |c: V| {
					c.chunk_stream(|mut s| {
						s.chunk(b"TeWad", |c: V| {
							c.chunk_stream(|mut s| {
								s.chunk(b"TePath", &wad_path)
							})
						})
					})
				})?;
				s.chunk(b"TeTextures", |c: V| {
					c.chunk_stream(|mut s| {
						s.chunk(b"TeLvlTexture", |c: V| {
							c.chunk_stream(|mut s| {
								s.chunk(b"TeI", Leb128(0))?;
								s.chunk(b"TePath", &textures_path)?;
								Ok(())
							})
						})
					})
				})?;
				Ok(())
			})
		})?;
		s.chunk(b"TeRooms", |c: V| {
			c.chunk_stream(|mut s| {
				for (room_index, room) in level.rooms().iter().enumerate() {
					let num_sectors = room.num_sectors();
					let sectors_x = num_sectors.x as i16;
					let sectors_z = num_sectors.z as i16;
					println!("room {}: sectors: x: {}, z: {}", room_index, sectors_x, sectors_z);
					let snapped_verts = room.vertices().iter().map(|v| snap_vert(v.pos())).collect::<Vec<_>>();
					for geom in room.geom() {
						for quad in geom.quads {
							quad.
						}
					}
					let snapped_quads = room.quads.iter().filter_map(|&tr1::TexturedQuad { vertex_indices, object_texture_index }| snap_face(&snapped_verts, vertex_indices, object_texture_index)).collect::<Vec<_>>();
					let snapped_tris = room.tris.iter().filter_map(|&tr1::TexturedTri { vertex_indices, object_texture_index }| snap_face(&snapped_verts, vertex_indices, object_texture_index)).collect::<Vec<_>>();
					s.chunk(b"TeRoom", |c: V| {
						c.leb128(sectors_x)?;
						c.leb128(sectors_z)?;
						c.chunk_stream(|mut s| {
							s.chunk(b"TeI", Leb128(room_index))?;
							s.chunk(b"TePos2", room.pos().as_vec3() / 1024.0)?;
							s.chunk(b"TeAmbient", Vec3::splat(1.0 - (room.ambient_light as f32 / 8191.0)))?;
							s.chunk(b"TeRoomType", Leb128(room_type(room.flags)))?;
							if room.flip_room_index != u16::MAX {
								s.chunk(b"TeAlternate", |c: V| {
									c.chunk_stream(|mut s| {
										s.chunk(b"TeRoom", Leb128(room.flip_room_index))
									})
								})?;
							}
							s.chunk(b"TeSecs", |c: V| {
								c.chunk_stream(|mut s| {
									for (index, sector) in room.sectors.iter().enumerate() {
										let index = index as i16;
										let sec_x = index / sectors_z;
										let sec_z = index % sectors_z;
										println!("sector: x: {}, z: {}", sec_x, sec_z);
										let position = (sec_z * sectors_x + sec_x) as u32;
										if true {//(1..sectors_x - 1).contains(&sec_x) && (1..sectors_z - 1).contains(&sec_z) {
											let mut flags = SectorFlags::empty();
											let floor_ceiling = floor_ceiling::get(&snapped_quads, &snapped_tris, sec_x, sec_z);
											let (divisions, num_ceiling_divs) = walls::get(&snapped_quads, &snapped_tris, sec_x, sec_z, &floor_ceiling);
											assert!(divisions.len() != 1);
											if let [None, None] = floor_ceiling {
												flags |= SectorFlags::WALL;
											}
											let floor_ceiling_heights_diags = floor_ceiling.map(|fc| {
												match fc {
													Some(fc) => {
														let diag_details = match fc.surface_type {
															SurfaceType::Quad(_) => 0,
															SurfaceType::Tris(tris) => {
																let corner = match tris {
																	Tris::SingleTri(SingleTri { corner, .. }) => corner,
																	Tris::ConnectedTris(ConnectedTris { x_equals_z, .. }) => x_equals_z as u8,
																	Tris::SteppedTris(SteppedTris { stepped_corner, .. }) => stepped_corner,
																};
																diag_details_from_corner(corner)
															},
														};
														(fc.corner_heights, diag_details)
													},
													None => ([sector.floor as i16; 4], 0),
												}
											});
											let [floor_heights_diag, ceiling_heights_diag] = floor_ceiling_heights_diags;
											s.chunk(b"TeS", |c: V| {
												c.write_all(position.as_bytes())?;
												c.chunk_stream(|mut s| {
													s.chunk(SECTOR_FLAGS_CHUNK_ID, Leb128(flags))?;
													for ((heights, diag), chunk_id) in [
														(floor_heights_diag, SECTOR_FLOOR_CHUNK_ID),
														(ceiling_heights_diag, SECTOR_CEILING_CHUNK_ID),
													] {
														s.chunk(chunk_id, |c: V| {
															c.leb128(diag)?;
															for corner_height in heights {
																c.leb128(corner_height * -256)?;
															}
															Ok(())
														})?;
													}
													s.chunk(SECTOR_CEILING_DIVISIONS_CHUNK_ID, |c: V| {
														c.leb128(num_ceiling_divs)?;
														for div in divisions[..num_ceiling_divs].iter().rev() {
															for h in div {
																c.leb128(h * -256)?;
															}
														}
														Ok(())
													})?;
													s.chunk(SECTOR_FLOOR_DIVISIONS_CHUNK_ID, |c: V| {
														let floor_divs = &divisions[num_ceiling_divs..];
														c.leb128(floor_divs.len())?;
														for div in floor_divs {
															for h in div {
																c.leb128(h * -256)?;
															}
														}
														Ok(())
													})?;
													Ok(())
												})?;
												Ok(())
											})?;
										}
									}
									Ok(())
								})
							})?;
							Ok(())
						})
					})?;
				}
				Ok(())
			})
		})?;
		Ok(())
	})
}
