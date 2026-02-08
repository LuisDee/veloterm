// UI overlay shader for colored rectangles (dividers, focus overlays).
// Positions are in pixels; converted to NDC using surface_size uniform.
// Supports SDF-based rounded corners via extras.x (border_radius).

struct OverlayUniforms {
    surface_size: vec2<f32>,  // width, height in pixels
    _padding: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: OverlayUniforms;

struct OverlayInstance {
    @location(0) rect: vec4<f32>,   // x, y, width, height in pixels
    @location(1) color: vec4<f32>,  // RGBA
    @location(2) extras: vec4<f32>, // border_radius in .x
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) local_pos: vec2<f32>,    // pixel offset from rect origin
    @location(2) rect_size: vec2<f32>,    // width, height of the rect
    @location(3) border_radius: f32,
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
    out.local_pos = corner * vec2<f32>(instance.rect.z, instance.rect.w);
    out.rect_size = vec2<f32>(instance.rect.z, instance.rect.w);
    out.border_radius = instance.extras.x;
    return out;
}

// Convert a single sRGB component to linear.
fn srgb_to_linear(c: f32) -> f32 {
    if c <= 0.04045 {
        return c / 12.92;
    }
    return pow((c + 0.055) / 1.055, 2.4);
}

// Convert an RGB color from sRGB to linear, preserving alpha.
fn srgb_color_to_linear(c: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(srgb_to_linear(c.r), srgb_to_linear(c.g), srgb_to_linear(c.b), c.a);
}

// SDF for a rounded rectangle centered at origin.
fn sd_rounded_box(p: vec2<f32>, half_size: vec2<f32>, radius: f32) -> f32 {
    let r = min(radius, min(half_size.x, half_size.y));
    let q = abs(p) - half_size + vec2<f32>(r, r);
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Convert sRGB input color to linear for correct output on sRGB surface
    let linear_color = srgb_color_to_linear(in.color);

    if in.border_radius > 0.0 {
        let half_size = in.rect_size * 0.5;
        let center = half_size;
        let p = in.local_pos - center;
        let d = sd_rounded_box(p, half_size, in.border_radius);
        // Anti-alias: smooth transition over ~1px
        let alpha = 1.0 - smoothstep(-0.5, 0.5, d);
        if alpha < 0.001 {
            discard;
        }
        return vec4<f32>(linear_color.rgb, linear_color.a * alpha);
    }
    return linear_color;
}
