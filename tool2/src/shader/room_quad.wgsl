@group(0) @binding(0) var<uniform> transform: mat4x4f;
@group(0) @binding(1) var<uniform> room_xz: vec2i;

/*
2048 8-byte vertex structs:
{
	x: i16,
	y: i16,
	z: i16,
	light: i16,
}
x, z relative to room
*/
@group(0) @binding(2) var<uniform> vertices: array<vec4u, 1024>;

/*
816 20-byte object texture structs:
{
	blend_mode: u16,
	atlas_index: u16,
	atlas_vertices: [[u16; 2]; 4],
}
1632 total
*/
@group(0) @binding(3) var<uniform> object_textures1: array<vec4u, 1020>;
@group(0) @binding(4) var<uniform> object_textures2: array<vec4u, 1020>;

struct VertexOutput {
	@builtin(position) position: vec4f,
	@location(0) atlas_index: u32,
	@location(1) atlas_vertex: vec2f,
};

fn get_position(quad_index: u32, vertex_indices: vec4u) -> vec4f {
	var vertex_index = vertex_indices[quad_index];
	var packed_vertex_lower = vertices[vertex_index / 2][(vertex_index % 2) * 2];
	var packed_vertex_upper = vertices[vertex_index / 2][((vertex_index % 2) * 2) + 1];
	var x = i32(packed_vertex_lower >> 16);
	var y = i32(packed_vertex_lower & 0xFFFF);
	var z = i32(packed_vertex_upper >> 16);
	x += room_xz.x;
	z += room_xz.y;
	var vertex = vec3i(x, y, z);
	return transform * vec4f(vec3f(vertex), 1.0);
}

fn obj_tex_u32(obj_tex_u32_index: u32) -> u32 {
	if obj_tex_u32_index < 4080 {
		return object_textures1[obj_tex_u32_index / 4][obj_tex_u32_index % 4];
	} else {
		return object_textures2[obj_tex_u32_index / 4][obj_tex_u32_index % 4];
	}
}

fn get_atlas_pos(quad_index: u32, object_texture_index: u32) -> vec3u {
	var atlas_index = obj_tex_u32(object_texture_index * 5) & 0xFFFF;
	var packed_atlas_vertex = obj_tex_u32((object_texture_index * 5) + 1 + quad_index);
	var x = packed_atlas_vertex >> 16;
	var y = packed_atlas_vertex & 0xFFFF;
	var atlas_vertex_subpixel = vec2u(x, y);//units are 1/256 of a pixel
	var atlas_vertex = (atlas_vertex_subpixel + vec2u(128)) / 256;//round to nearest whole pixel
	return vec3u(atlas_index, atlas_vertex);//pack both values into vec3u for single return
}

@vertex
fn vs_main(
	@location(0) quad_index: u32,//0..4
	@location(1) vertex_indices: vec4u,
	@location(2) object_texture_index: vec2u,//second element is padding
) -> VertexOutput {
	var position = get_position(quad_index, vertex_indices);
	var atlas_pos = get_atlas_pos(quad_index, object_texture_index.x);
	var atlas_index = atlas_pos.x;
	var atlas_vertex = atlas_pos.yz;
	return VertexOutput(position, atlas_index, vec2f(atlas_vertex));
}

@group(0) @binding(5) var atlases: texture_2d_array<f32>;

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4f {
	var color = textureLoad(atlases, vec2<i32>(vertex.atlas_vertex), vertex.atlas_index, 0);
	// if color.a < 0.5 {
	// 	discard;
	// } else {
	// 	return color;
	// }
	return vec4f(1.0);
}
