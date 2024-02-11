struct UpscalingUniform {
    width: u32,
    height: u32,
}

@group(0) @binding(0) var<uniform> upscaling_uniform: UpscalingUniform;
@group(0) @binding(1) var gi_texture: texture_2d<f32>;
@group(0) @binding(2) var shadow_texture: texture_2d<f32>;
@group(0) @binding(3) var s: sampler;

struct UpscalingVertexOutput {
    @builtin(position)
    position: vec4<f32>,
    @location(0)
    uv: vec2<f32>,
    @location(1)
    texture_coordinates: vec2<f32>,
};


// Taken from https://github.com/bevyengine/bevy/blob/main/crates/bevy_core_pipeline/src/fullscreen_vertex_shader/fullscreen.wgsl
// This vertex shader produces the following, when drawn using indices 0..3:
//
//  1 |  0-----x.....2
//  0 |  |  s  |  . ´
// -1 |  x_____x´
// -2 |  :  .´
// -3 |  1´
//    +---------------
//      -1  0  1  2  3
//
// The axes are clip-space x and y. The region marked s is the visible region.
// The digits in the corners of the right-angled triangle are the vertex
// indices.
//
// The top-left has UV 0,0, the bottom-left has 0,2, and the top-right has 2,0.
// This means that the UV gets interpolated to 1,1 at the bottom-right corner
// of the clip-space rectangle that is at 1,-1 in clip space.
@vertex
fn upscaling_vertex_shader(@builtin(vertex_index) vertex_index: u32) -> UpscalingVertexOutput {
    var uv = vec2<f32>(f32(vertex_index >> 1u), f32(vertex_index & 1u)) * 2.0;
    var clip_position: vec4<f32> = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);

    // Subtract 1 from width and height because storage textures use 0-based indexing.
    let texture_coordinates = vec2<f32>(uv.x * f32(upscaling_uniform.width), uv.y * f32(upscaling_uniform.height));

    return UpscalingVertexOutput(clip_position, uv, texture_coordinates);
}

@fragment
fn upscaling_fragment_shader(vertex_output: UpscalingVertexOutput) -> @location(0) vec4<f32> {
    // return textureLoad(gi_texture, vec2u(vertex_output.texture_coordinates));
    return textureSample(gi_texture, s, vertex_output.uv);
    // return textureSample(gi_texture, s, vec2f(0.0, 0.0));
}
