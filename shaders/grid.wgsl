// Terminal grid cell rendering shader.
// Vertex shader expands vertex index (0-5) to quad corners.
// Fragment shader samples glyph atlas texture and blends fg/bg colors.
// Supports underline, strikethrough decorations, and cursor rendering via cell flags.

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
    @location(4) flags: u32,            // bit 0: has_glyph, bit 1: is_cursor,
                                        // bits 2-3: cursor shape (00=block, 01=beam, 10=underline, 11=hollow)
                                        // bit 4: underline, bit 5: strikethrough, bit 6: selected
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
    @location(7) is_cursor: f32,        // 1.0 if cursor flag set
    @location(8) cursor_shape: f32,     // 0=block, 1=beam, 2=underline, 3=hollow
    @location(9) cell_x_frac: f32,      // 0.0 at left, 1.0 at right
    @location(10) is_selected: f32,     // 1.0 if selected flag set
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
    out.cell_x_frac = corner.x;
    out.underline = f32((cell.flags >> 4u) & 1u);
    out.strikethrough = f32((cell.flags >> 5u) & 1u);
    out.is_cursor = f32((cell.flags >> 1u) & 1u);
    out.cursor_shape = f32((cell.flags >> 2u) & 3u);
    out.is_selected = f32((cell.flags >> 6u) & 1u);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Cursor rendering — draw cursor shape using bg_color as cursor color
    if in.is_cursor > 0.5 {
        let cursor_color = in.bg_color.rgb;
        let shape = u32(in.cursor_shape + 0.5);

        if shape == 0u {
            // Block cursor: fill entire cell
            return vec4<f32>(cursor_color, 1.0);
        } else if shape == 1u {
            // Beam cursor: thin vertical line on left (~10% of cell width)
            if in.cell_x_frac < 0.12 {
                return vec4<f32>(cursor_color, 1.0);
            }
            discard;
        } else if shape == 2u {
            // Underline cursor: thin horizontal line at bottom (~10% of cell height)
            if in.cell_y_frac > 0.88 {
                return vec4<f32>(cursor_color, 1.0);
            }
            discard;
        } else {
            // Hollow block cursor: outline border (~8% on each edge)
            let border = 0.08;
            if in.cell_x_frac < border || in.cell_x_frac > (1.0 - border) ||
               in.cell_y_frac < border || in.cell_y_frac > (1.0 - border) {
                return vec4<f32>(cursor_color, 1.0);
            }
            discard;
        }
    }

    // Selection: swap fg and bg colors
    var fg = in.fg_color.rgb;
    var bg = in.bg_color.rgb;
    if in.is_selected > 0.5 {
        fg = in.bg_color.rgb;
        bg = in.fg_color.rgb;
    }

    var color: vec3<f32>;

    if in.has_glyph < 0.5 {
        // No glyph — start with background color
        color = bg;
    } else {
        // Sample glyph alpha from atlas
        let glyph_alpha = textureSample(atlas_texture, atlas_sampler, in.uv).r;
        // Blend: background behind, foreground glyph on top
        color = mix(bg, fg, glyph_alpha);
    }

    // Underline: draw a line at the bottom ~7% of the cell (roughly 1-2px at typical sizes)
    if in.underline > 0.5 && in.cell_y_frac > 0.9 {
        color = fg;
    }

    // Strikethrough: draw a line at the vertical center ~7% band
    if in.strikethrough > 0.5 && in.cell_y_frac > 0.46 && in.cell_y_frac < 0.54 {
        color = fg;
    }

    return vec4<f32>(color, 1.0);
}
