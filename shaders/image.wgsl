// Image quad rendering shader for the Kitty Graphics Protocol.
// Renders textured quads for inline terminal images with alpha blending.
// Each image gets its own texture+sampler bind group; this shader just
// maps UV coordinates to the image texture.

struct ImageUniforms {
    // NDC position of the quad (x, y = top-left corner in NDC)
    position: vec2<f32>,
    // NDC size of the quad (width, height)
    size: vec2<f32>,
    // UV rect within the source texture (u, v, w, h) — for cropping
    source_uv: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: ImageUniforms;

@group(0) @binding(1)
var image_texture: texture_2d<f32>;

@group(0) @binding(2)
var image_sampler: sampler;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    // Expand vertex index to quad corners (two triangles: 0,1,2 and 2,1,3)
    let quad_index = array<vec2<f32>, 6>(
        vec2<f32>(0.0, 0.0),  // top-left
        vec2<f32>(1.0, 0.0),  // top-right
        vec2<f32>(0.0, 1.0),  // bottom-left
        vec2<f32>(0.0, 1.0),  // bottom-left
        vec2<f32>(1.0, 0.0),  // top-right
        vec2<f32>(1.0, 1.0),  // bottom-right
    );

    let corner = quad_index[vertex_index];

    // Position in NDC: origin at top-left of quad, Y goes down
    let x = uniforms.position.x + corner.x * uniforms.size.x;
    let y = uniforms.position.y - corner.y * uniforms.size.y;

    // UV: map corner to source rect within the texture
    let uv = vec2<f32>(
        uniforms.source_uv.x + corner.x * uniforms.source_uv.z,
        uniforms.source_uv.y + corner.y * uniforms.source_uv.w,
    );

    var out: VertexOutput;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = uv;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSampleLevel(image_texture, image_sampler, in.uv, 0.0);

    // Convert sRGB texture to linear for correct blending on sRGB surface
    let linear_rgb = vec3<f32>(
        srgb_to_linear(color.r),
        srgb_to_linear(color.g),
        srgb_to_linear(color.b),
    );

    return vec4<f32>(linear_rgb, color.a);
}

fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        return c / 12.92;
    }
    return pow((c + 0.055) / 1.055, 2.4);
}
