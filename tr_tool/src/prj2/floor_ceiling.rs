use glam::{i16vec2, Vec3Swizzles};
use super::SnappedFace;

pub struct Quad {
	pub object_texture_index: u16,
}

fn get_quads(snapped_quads: &[SnappedFace<4>], sec_x: i16, sec_z: i16) -> [Option<([i16; 4], Quad)>; 2] {
	let sec_pos = i16vec2(sec_x, sec_z);
	let mut floor_ceiling = [const { None }; 2];
	let mut fc_indices = &[0, 1][..];
	'outer: for &SnappedFace { vertices, object_texture_index } in snapped_quads {
		let Some(xnzn) = vertices.iter().position(|v| v.xz() == sec_pos) else {
			continue;
		};
		let xpzp = (xnzn + 2) % 4;
		if vertices[xpzp].xz() != sec_pos + 1 {
			continue;
		}
		let other_corners = [(xnzn + 1) % 4, (xnzn + 3) % 4];
		for &fc in fc_indices {
			let xnzp = other_corners[fc];
			let xpzn = other_corners[1 - fc];
			if {
				vertices[xnzp].xz() == i16vec2(sec_x, sec_z + 1) &&
				vertices[xpzn].xz() == i16vec2(sec_x + 1, sec_z)
			} {
				floor_ceiling[fc] = Some((
					[xnzp, xpzp, xpzn, xnzn].map(|i| vertices[i].y),
					Quad { object_texture_index },
				));
				fc_indices = &fc_indices[(1 - fc)..][..fc_indices.len() - 1];
				if fc_indices.is_empty() {
					break 'outer;
				} else {
					break;
				}
			}
		}
	}
	floor_ceiling
}

/// May be a diagonal wall sector, or space sector with the other tri being a door or transparent (no poly).
pub struct SingleTri {
	/// `0..3: [xnzp, xpzp, xpzn, xnzn]`.
	pub corner: u8,
	pub object_texture_index: u16,
}

pub struct ConnectedTris {
	/// Dividing line is parallel to the plane x = z.
	pub x_equals_z: bool,
	/// Negative X (left) tri. Either xnzp or xnzn corners.
	pub xn_object_texture_index: u16,
	/// Positive X (right) tri. Either xpzp or xpzn corners.
	pub xp_object_texture_index: u16,
}

/// Height of base corner is height for all three vertices of base tri.
pub struct SteppedTris {
	pub stepped_corner: u8,
	pub base_object_texture_index: u16,
	pub step_object_texture_index: u16,
}

pub enum Tris {
	SingleTri(SingleTri),
	ConnectedTris(ConnectedTris),
	SteppedTris(SteppedTris),
}

fn get_tris(
	snapped_tris: &[SnappedFace<3>], sec_x: i16, sec_z: i16, fc: usize,
) -> Option<([i16; 4], Tris)> {
	let coords = [
		i16vec2(sec_x, sec_z + 1),
		i16vec2(sec_x + 1, sec_z + 1),
		i16vec2(sec_x + 1, sec_z),
		i16vec2(sec_x, sec_z),
	];
	let mut surface = None;
	'outer: for &SnappedFace { vertices, object_texture_index } in snapped_tris {
		for corner in 0..4 {
			let Some((_offset, mut corner_heights)) = (0..3).find_map(|offset| {//TODO: offset needed?
				let mut heights = [i16::MAX; 4];//MAX: makes it obvious if it isn't initialized
				for i in 0..3 {
					let coord_index = (corner + ((i + 1) % 3) + 3) % 4;
					let v = vertices[(offset + i * (fc + 1)) % 3];
					if v.xz() != coords[coord_index] {
						return None;
					}
					heights[coord_index] = v.y;
				}
				Some((offset, heights))
			}) else {
				continue;
			};
			let opposite = (corner + 2) % 4;
			match surface {
				None => {
					corner_heights[opposite] = {
						(corner_heights[(corner + 1) % 4] + corner_heights[(corner + 3) % 4]) / 2
					};//set opposite corner to average of adjacent
					surface = Some((
						corner_heights,
						Tris::SingleTri(SingleTri {
							corner: corner as u8,
							object_texture_index,
						}),
					))
				},
				Some((tri_ch, Tris::SingleTri(tri))) => {
					assert!(opposite as u8 == tri.corner);//else they are overlapping
					//tris are connected
					if (0..2).map(|i| (corner + 1 + 2 * i) % 4).all(|i| tri_ch[i] == corner_heights[i]) {
						corner_heights[opposite] = tri_ch[opposite];
						let obj_tex_indices = [object_texture_index, tri.object_texture_index];
						let xn = ((corner + 1) / 2) % 2;
						surface = Some((
							corner_heights,
							Tris::ConnectedTris(ConnectedTris {
								x_equals_z: corner % 2 == 0,
								xn_object_texture_index: obj_tex_indices[xn],
								xp_object_texture_index: obj_tex_indices[1 - xn],
							}),
						));
					} else {
						let tris = [
							(corner_heights, 3, object_texture_index),
							(tri_ch, 1, tri.object_texture_index),
						];
						let [this_height, other_height] = tris.map(|(h, o, _)| {
							(0..3).map(|i| h[(corner + o + i) % 4]).sum::<i16>()
						});
						let base = (this_height < other_height) as usize ^ fc;//0 if this, 1 if other
						let base_corner = (corner + base * 2) % 4;
						let base_height = tris[base].0[base_corner];
						//base is flat
						assert!((0..2).all(|i| tris[base].0[(base_corner + 1 + 2 * i) % 4] == base_height));
						let mut corner_heights = tris[1 - base].0;
						corner_heights[base_corner] = base_height;
						surface = Some((
							corner_heights,
							Tris::SteppedTris(SteppedTris {
								stepped_corner: ((base_corner + 2) % 4) as u8,
								base_object_texture_index: tris[base].2,
								step_object_texture_index: tris[1 - base].2,
							}),
						));
					}
				},
				_ => panic!("three tris"),
			}
			continue 'outer;
		}
	}
	surface
}

pub enum SurfaceType {
	Quad(Quad),
	Tris(Tris),
}

pub struct SectorSurface {
	/// `[xnzp, xpzp, xpzn, xnzn]`
	pub corner_heights: [i16; 4],
	pub surface_type: SurfaceType,
}

pub fn get(
	snapped_quads: &[SnappedFace<4>], snapped_tris: &[SnappedFace<3>], sec_x: i16, sec_z: i16,
) -> [Option<SectorSurface>; 2] {
	let quads = get_quads(snapped_quads, sec_x, sec_z);
	let mut floor_ceiling = [const { None }; 2];
	for (fc, quad) in quads.into_iter().enumerate() {
		let (corner_heights, surface_type) = if let Some((corner_heights, quad)) = quad {
			(corner_heights, SurfaceType::Quad(quad))
		} else if let Some((corner_heights, tris)) = get_tris(snapped_tris, sec_x, sec_z, fc) {
			(corner_heights, SurfaceType::Tris(tris))
		} else {
			continue;
		};
		floor_ceiling[fc] = Some(SectorSurface { corner_heights, surface_type });
	}
	floor_ceiling
}
