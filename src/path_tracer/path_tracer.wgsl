#import bevy_render::view::View

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var output_texture: texture_storage_2d<rgba16float, read_write>;
// @group(0) @binding(2) var output_texture_sampler: sampler;

@compute @workgroup_size(8, 8, 1)
fn path_trace(@builtin(global_invocation_id) id: vec3<u32>) {
    textureStore(output_texture, id.xy, vec4<f32>(1.0, 0.0, 0.0, 1.0));
}

// struct FullscreenVertexOutput {
//     @builtin(position)
//     position: vec4<f32>,
//     @location(0)
//     tex_coords: vec2<u32>,
// };

// @vertex
// fn fullscreen_vertex(@builtin(vertex_index) vertex_index: u32) -> FullscreenVertexOutput {
//     let uv = vec2<f32>(f32(vertex_index >> 1u), f32(vertex_index & 1u)) * 2.0;
//     let clip_position = vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);

//     // NOTE: view.viewport is vec4<f32>(x_orig, y_orig, width, height)
//     let tex_coords = vec2<u32>(u32(uv.x * view.viewport.z), u32(uv.y * view.viewport.w));


//     return FullscreenVertexOutput(clip_position, tex_coords);
// }

// @vertex
// fn fullscreen_vertex(@location(0) position: vec2<f32>) -> @builtin(position) vec4<f32> {
//     return view.view_proj * vec4<f32>(position.xy, 1.0, 1.0);
// }

// @fragment
// fn fullscreen_fragment(@builtin(position) clip_position: vec4<f32>) -> @location(0) vec4<f32> {
//     // return textureLoad(output_texture, input.uv);
//     return vec4<f32>(0.0, 0.0, 1.0, 1.0);
// }







struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn fullscreen_vertex(
    model: VertexInput,
) -> VertexOutput {
    var out: VertexOutput;
    out.color = model.color;
    out.clip_position = vec4<f32>(model.position, 1.0);
    return out;
}

// Fragment shader

@fragment
fn fullscreen_fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // return vec4<f32>(in.color, 1.0);
    return textureLoad(output_texture, vec2<u32>(1u, 1u));
}