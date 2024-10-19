/*
struct Uv {
	x: u16,
	y: u16,
}
struct ObjectTexture {
	blend_mode: u16,
	atlas_index: u16,
	uvs: [Uv; 4],
}

struct RoomVertex {
	x: i16,
	y: i16,
	z: i16,
	light: u16,
}
struct ModelVertex {
	x: i16,
	y: i16,
	z: i16,
}
struct Vertex: either RoomVertex or ModelVertex;
struct VertexArray {
	vertex_size: u16,
	vertices: [Vertex],
}

struct Quad {
	vertex_indices: [u16; 4],
	texture_index: u16,
}
struct Tri {
	vert_indices: [u16; 3],
	texture_index: u16,
}
struct Face: either Quad or Tri;
struct FaceArray {
	vertex_array_offset: [u16; 2],
	face_size: u16,
	faces: [Face],
}

mapping:
{
	---align 4
	object_textures: [ObjectTexture],
	---align 2
	mix of VertexArrays, FaceArrays, and Mat4s
}
notes:
if textured, texture_index is an index into the ObjectTexture array
if solid, texture_index is an index into palette
*/
@group(0) @binding(0) var<storage> data: array<vec4u, 65536>;

@group(0) @binding(1) var<uniform> global_transform: mat4x4f;

/*
offset:
VVVVVVVVVVVVVVVVVVVVVVVVVVVVVVEE
V: vec4u in data
E: u32 in vec4u
*/
fn get_data_u32(offset: u32) -> u32 {
	return data[offset / 4][offset % 4];
}

/*
offset:
VVVVVVVVVVVVVVVVVVVVVVVVVVVVVEES
V: vec4u in data
E: u32 in vec4u
S: u16 in u32
*/
fn get_data_u16(offset: u32) -> u32 {
	return (get_data_u32(offset / 2) >> ((offset % 2) * 16)) & 0xFFFF;
}

struct PositionTexture {
	position: vec4f,
	texture_index: u32,
}

const TRANSFORMS_OFFSET: u32 = 8192;
const INDEX_OFFSET: u32 = 196608;

fn get_position_texture(face: u32, face_vertex_index: u32) -> PositionTexture {
	//unpack face instance
	let face_array_index = face & 1023;
	let face_index = (face >> 10) & 1023;
	let transform_index = (face >> 20) & 1023;
	//transform
	let transform_offset_v4 = (TRANSFORMS_OFFSET + transform_index) * 4;
	let local_transform = mat4x4f(
		bitcast<vec4f>(data[transform_offset_v4]),
		bitcast<vec4f>(data[transform_offset_v4 + 1]),
		bitcast<vec4f>(data[transform_offset_v4 + 2]),
		bitcast<vec4f>(data[transform_offset_v4 + 3]),
	);
	//position
	let face_array_offset = get_data_u32(INDEX_OFFSET + face_array_index);
	let vertex_array_offset_lower = get_data_u16(face_array_offset);
	let vertex_array_offset_upper = get_data_u16(face_array_offset + 1);
	let vertex_array_offset = vertex_array_offset_lower | (vertex_array_offset_upper << 16);
	let vertex_size = get_data_u16(vertex_array_offset);
	let face_size = get_data_u16(face_array_offset + 2);
	let face_offset = face_array_offset + 3 + (face_index * face_size);
	let vertex_index = get_data_u16(face_offset + face_vertex_index);
	let vertex_offset = vertex_array_offset + 1 + (vertex_index * vertex_size);
	let vertex_unsigned = vec3u(
		get_data_u16(vertex_offset),
		get_data_u16(vertex_offset + 1),
		get_data_u16(vertex_offset + 2),
	);
	let vertex_relative = vec3i(vertex_unsigned << vec3u(16)) >> vec3u(16);//interpret lower 16 as i16
	let vertex_absolute = local_transform * vec4f(vec3f(vertex_relative), 1.0);
	let position = global_transform * vertex_absolute;
	//texture
	let texture_index = get_data_u16(face_offset + face_size - 1);
	return PositionTexture(position, texture_index);
}

struct TexturedVTF {
	@builtin(position) position: vec4f,
	@location(0) blend_mode: u32,
	@location(1) atlas_index: u32,
	@location(2) uv: vec2f,
}

@vertex
fn textured_vs_main(
	@location(0) face_vertex_index: u32,//vertex
	@location(1) face: u32,//instance
) -> TexturedVTF {
	let position_texture = get_position_texture(face, face_vertex_index);
	let position = position_texture.position;
	let object_texture_index = position_texture.texture_index;
	let object_texture_offset = object_texture_index * 5;//5 = size of ObjectTexture in u32s
	let blend_atlas_packed = get_data_u32(object_texture_offset);
	let blend_mode = blend_atlas_packed & 0xFFFF;
	let atlas_index = blend_atlas_packed >> 16;
	let uv_packed = get_data_u32(object_texture_offset + 1 + face_vertex_index);
	let uv_subpixel = vec2u(uv_packed & 0xFFFF, uv_packed >> 16);
	let uv = vec2f((uv_subpixel + 128) / 256);//round to nearest whole pixel
	return TexturedVTF(position, blend_mode, atlas_index, uv);
}

struct SolidVTF {
	@builtin(position) position: vec4f,
	@location(0) color_index: u32,
}

@vertex
fn solid_vs_main(
	@location(0) face_vertex_index: u32,//vertex
	@location(1) face: u32,//instance
) -> SolidVTF {
	let position_texture = get_position_texture(face, face_vertex_index);
	let position = position_texture.position;
	let color_index = position_texture.texture_index;
	return SolidVTF(position, color_index);
}

/*
struct Color {
	r: u8,
	g: u8,
	b: u8,
}
mapping:
[Color; 256]
notes:
6-bit color
*/
@group(0) @binding(2) var<uniform> palette: array<vec4u, 48>;

/*
offset:
VVVVVVVVVVVVVVVVVVVVVVVVVVVVEEBB
V: vec4u in palette
E: u32 in vec4u
B: u8 in u32
*/
fn get_color_u8(offset: u32) -> u32 {
	return (palette[offset / 16][(offset / 4) % 4] >> ((offset % 4) * 8)) & 0xFF;
}

fn get_color(color_index: u32) -> vec4f {
	var offset = color_index * 3;
	var color_rgb_6bit = vec3u(
		get_color_u8(offset),
		get_color_u8(offset + 1),
		get_color_u8(offset + 2),
	);
	var color_rgb = vec3f(color_rgb_6bit) / 63.0;
	var color_rgba = vec4f(color_rgb, 1.0);
	return color_rgba;
}

@group(0) @binding(3) var atlases: texture_2d_array<u32>;

const BLEND_MODE_TEST: u32 = 1;

@fragment
fn textured_fs_main(vtf: TexturedVTF) -> @location(0) vec4f {
	var color_index = textureLoad(atlases, vec2i(vtf.uv), vtf.atlas_index, 0).x;
	if vtf.blend_mode == BLEND_MODE_TEST && color_index == 0 {
		discard;
	} else {
		return get_color(color_index);
	}
}

@fragment
fn solid_fs_main(vtf: SolidVTF) -> @location(0) vec4f {
	return get_color(vtf.color_index);
}
