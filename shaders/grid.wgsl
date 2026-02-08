// Terminal grid cell rendering shader.
// Vertex shader expands vertex index (0-5) to quad corners.
// Fragment shader samples glyph atlas texture and blends fg/bg colors.
// Supports underline and strikethrough decorations via cell flags.

struct Uniforms {
    cell_size: vec2<f32>,    // width, height in NDC
    grid_size: vec2<f32>,    // columns, rows
    atlas_size: vec2<f32>,   // atlas texture dimensions in pixels
    _padding: vec2<f32>,
};

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var atlas_texture: texture_2d<f32>;

@group(0) @binding(2)
var atlas_sampler: sampler;

struct CellInstance {
    // Per-instance data from vertex buffer
    @location(0) position: vec2<f32>,   // cell position in grid (col, row)
    @location(1) atlas_uv: vec4<f32>,   // UV rect in atlas (u, v, width, height)
    @location(2) fg_color: vec4<f32>,   // foreground RGBA
    @location(3) bg_color: vec4<f32>,   // background RGBA
    @location(4) flags: u32,            // bit 0: has_glyph, bit 4: underline, bit 5: strikethrough
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) fg_color: vec4<f32>,
    @location(2) bg_color: vec4<f32>,
    @location(3) has_glyph: f32,
    @location(4) cell_y_frac: f32,      // 0.0 at top of cell, 1.0 at bottom
    @location(5) underline: f32,        // 1.0 if underline flag set
    @location(6) strikethrough: f32,    // 1.0 if strikethrough flag set
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    cell: CellInstance,
) -> VertexOutput {
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

    // Convert grid position to NDC (-1 to +1)
    // Origin is top-left, Y increases downward
    let x = -1.0 + (cell.position.x + corner.x) * uniforms.cell_size.x;
    let y = 1.0 - (cell.position.y + corner.y) * uniforms.cell_size.y;

    // Calculate UV coordinates within the glyph atlas
    let uv = cell.atlas_uv.xy + corner * cell.atlas_uv.zw;

    var out: VertexOutput;
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = uv;
    out.fg_color = cell.fg_color;
    out.bg_color = cell.bg_color;
    out.has_glyph = f32(cell.flags & 1u);
    out.cell_y_frac = corner.y;
    out.underline = f32((cell.flags >> 4u) & 1u);
    out.strikethrough = f32((cell.flags >> 5u) & 1u);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var color: vec3<f32>;

    if in.has_glyph < 0.5 {
        // No glyph — start with background color
        color = in.bg_color.rgb;
    } else {
        // Sample glyph alpha from atlas
        let glyph_alpha = textureSample(atlas_texture, atlas_sampler, in.uv).r;
        // Blend: background behind, foreground glyph on top
        color = mix(in.bg_color.rgb, in.fg_color.rgb, glyph_alpha);
    }

    // Underline: draw a line at the bottom ~7% of the cell (roughly 1-2px at typical sizes)
    if in.underline > 0.5 && in.cell_y_frac > 0.9 {
        color = in.fg_color.rgb;
    }

    // Strikethrough: draw a line at the vertical center ~7% band
    if in.strikethrough > 0.5 && in.cell_y_frac > 0.46 && in.cell_y_frac < 0.54 {
        color = in.fg_color.rgb;
    }

    return vec4<f32>(color, 1.0);
}
