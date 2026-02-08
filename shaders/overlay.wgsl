// UI overlay shader for colored rectangles (dividers, focus overlays).
// Positions are in pixels; converted to NDC using surface_size uniform.

struct OverlayUniforms {
    surface_size: vec2<f32>,  // width, height in pixels
    _padding: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: OverlayUniforms;

struct OverlayInstance {
    @location(0) rect: vec4<f32>,   // x, y, width, height in pixels
    @location(1) color: vec4<f32>,  // RGBA
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: OverlayInstance,
) -> VertexOutput {
    let quad_index = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(0.0, 1.0),
        vec2<f32>(1.0, 0.0),
        vec2<f32>(1.0, 1.0),
    );

    let corner = quad_index[vertex_index];

    // Convert pixel rect to NDC (-1..+1, Y flipped)
    let px = instance.rect.x + corner.x * instance.rect.z;
    let py = instance.rect.y + corner.y * instance.rect.w;
    let ndc_x = (px / uniforms.surface_size.x) * 2.0 - 1.0;
    let ndc_y = 1.0 - (py / uniforms.surface_size.y) * 2.0;

    var out: VertexOutput;
    out.position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
    out.color = instance.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return in.color;
}
