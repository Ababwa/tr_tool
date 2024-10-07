@group(0) @binding(0) var<uniform> transform: mat4x4f;

/*
struct Uv {
	x: u16,
	y: u16,
}
struct ObjectTexture {
	blend_mode: u16,
	atlas_idx: u16,
	uvs: [Uv; 4],
}
mapping:
[ObjectTexture; 1632]
notes:
split across two bindings since total size exceeds the max buffer size of 16384
uv units are 1/256 of a pixel
*/
@group(0) @binding(1) var<uniform> obj_texs1: array<vec4u, 1020>;
@group(0) @binding(2) var<uniform> obj_texs2: array<vec4u, 1020>;

/*
offset:
VVVVVVVVVVVVVVVVVVVVVVVVVVVVVVEE
V: vec4u in obj_texs
E: u32 in vec4u
*/
fn get_obj_tex_u32(offset: u32) -> u32 {
	var v4_idx = offset / 4;
	var v4: vec4u;
	if v4_idx < 1020 {
		v4 = obj_texs1[v4_idx];
	} else {
		v4 = obj_texs2[v4_idx - 1020];
	}
	return v4[offset % 4];
}

struct ObjTex {
	blend_mode: u32,
	atlas_idx: u32,
	uv: vec2f,
}

fn get_obj_tex(tex_idx: u32, face_vert_idx: u32) -> ObjTex {
	var offset = tex_idx * 5;//5 = size of ObjTex in u16s
	var blend_atlas_packed = get_obj_tex_u32(offset);
	var blend_mode = blend_atlas_packed & 0xFFFF;
	var atlas_idx = blend_atlas_packed >> 16;
	var uv_packed = get_obj_tex_u32(offset + 1 + face_vert_idx);
	var uv_subpixel = vec2u(uv_packed & 0xFFFF, uv_packed >> 16);
	var uv = vec2f((uv_subpixel + 128) / 256);//round to nearest whole pixel
	return ObjTex(blend_mode, atlas_idx, uv);
}

@group(1) @binding(0) var<uniform> mesh_transform: mat4x4f;

/*
size of struct in verts in u16s
if room, 4
if mesh, 3
*/
@group(2) @binding(0) var<uniform> vert_size: u32;

/*
struct RoomVert {
	x: i16,
	y: i16,
	z: i16,
	light: u16,
}
struct Vert {
	x: i16,
	y: i16,
	z: i16,
}
mapping:
if room, [RoomVert; 2048]
if mesh, [Vert; 2730]
*/
@group(2) @binding(1) var<uniform> verts: array<vec4u, 1024>;

/*
offset:
VVVVVVVVVVVVVVVVVVVVVVVVVVVVVEES
V: vec4u in verts
E: u32 in vec4u
S: u16 in u32
*/
fn get_vert_u16(offset: u32) -> u32 {
	return (verts[offset / 8][(offset / 2) % 4] >> ((offset % 2) * 16)) & 0xFFFF;
}

fn get_vert(vert_idx: u32) -> vec3f {
	var vert_offset = vert_idx * vert_size;
	var vert_u = vec3u(
		get_vert_u16(vert_offset),
		get_vert_u16(vert_offset + 1),
		get_vert_u16(vert_offset + 2),
	);
	var vert_r = vec3i(vert_u << vec3u(16)) >> vec3u(16);//interpret lower 16 as i16
	var vert = mesh_transform * vec4f(vec3f(vert_r), 1.0);
	return vert.xyz;
}

/*
size of struct in faces in u16s
if quad, 5
if tri, 4
*/
@group(3) @binding(0) var<uniform> face_size: u32;

/*
struct Quad {
	vert_indices: [u16; 4],
	tex_idx: u16,
}
struct Tri {
	vert_indices: [u16; 3],
	tex_idx: u16,
}
mapping:
if quad, [Quad; 1632]
if tri, [Tri; 2048]
notes:
if textured, tex_idx is an index into obj_texs
if solid, tex_idx is an index into palette
*/
@group(3) @binding(1) var<uniform> faces: array<vec4u, 1024>;

/*
offset:
VVVVVVVVVVVVVVVVVVVVVVVVVVVVVEES
V: vec4u in faces
E: u32 in vec4u
S: u16 in u32
*/
fn get_face_u16(offset: u32) -> u32 {
	return (faces[offset / 8][(offset / 2) % 4] >> ((offset % 2) * 16)) & 0xFFFF;
}

struct FaceVert {
	vert_idx: u32,
	tex_idx: u32,
}

fn get_face_vert(face_vert_idx: u32, face_idx: u32) -> FaceVert {
	var face_offset = face_idx * face_size;
	var vert_idx = get_face_u16(face_offset + face_vert_idx);
	var tex_idx = get_face_u16(face_offset + face_size - 1);
	return FaceVert(vert_idx, tex_idx);
}

struct TexturedVTF {
	@builtin(position) pos: vec4f,
	@location(0) blend_mode: u32,
	@location(1) atlas_idx: u32,
	@location(2) uv: vec2f,
}

@vertex
fn textured_vs_main(
	@location(0) face_vert_idx: u32,
	@builtin(instance_index) face_idx: u32,
) -> TexturedVTF {
	var face_vert = get_face_vert(face_vert_idx, face_idx);
	var vert = get_vert(face_vert.vert_idx);
	var pos = transform * vec4f(vert, 1.0);
	var obj_tex = get_obj_tex(face_vert.tex_idx, face_vert_idx);
	return TexturedVTF(pos, obj_tex.blend_mode, obj_tex.atlas_idx, obj_tex.uv);
}

struct SolidVTF {
	@builtin(position) pos: vec4f,
	@location(0) color_idx: u32,
}

@vertex
fn solid_vs_main(
	@location(0) face_vert_idx: u32,
	@builtin(instance_index) face_idx: u32,
) -> SolidVTF {
	var face_vert = get_face_vert(face_vert_idx, face_idx);
	var vert = get_vert(face_vert.vert_idx);
	var pos = transform * vec4f(vert, 1.0);
	return SolidVTF(pos, face_vert.tex_idx);
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
@group(0) @binding(3) var<uniform> palette: array<vec4u, 48>;

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

fn get_color(color_idx: u32) -> vec4f {
	var offset = color_idx * 3;
	var color_rgb_6bit = vec3u(
		get_color_u8(offset),
		get_color_u8(offset + 1),
		get_color_u8(offset + 2),
	);
	var color_rgb = vec3f(color_rgb_6bit) / 63.0;
	var color_rgba = vec4f(color_rgb, 1.0);
	return color_rgba;
}

@group(0) @binding(4) var atlases: texture_2d_array<u32>;

const BLEND_MODE_TEST: u32 = 1;

@fragment
fn textured_fs_main(vtf: TexturedVTF) -> @location(0) vec4f {
	var color_idx = textureLoad(atlases, vec2i(vtf.uv), vtf.atlas_idx, 0).x;
	if vtf.blend_mode == BLEND_MODE_TEST && color_idx == 0 {
		discard;
	} else {
		return get_color(color_idx);
	}
}

@fragment
fn solid_fs_main(vtf: SolidVTF) -> @location(0) vec4f {
	return get_color(vtf.color_idx);
}
