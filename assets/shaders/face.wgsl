#import bevy_pbr::forward_io::VertexOutput

@group(1) @binding(0)
var texture: texture_2d<f32>;

@group(1) @binding(1)
var our_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
	var color = textureSample(texture, our_sampler, in.uv);
	if color.a < 0.5 {
		discard;
	}
	return color;
}
