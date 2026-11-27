//! The cabinet's CRT: the curved-glass, scanline and seal shaders (GLSL ES 1.00, for WebGL 1) that `Blip` draws the
//! finished frame through, and the CPU twin of the glass's curve.

use macroquad::math::{vec2, Vec2};
use macroquad::material::{load_material, Material, MaterialParams};
use macroquad::miniquad::{ShaderSource, UniformDesc, UniformType};

/// CPU twin of the shader's `curve()`: screen UV -> the picture UV drawn there.
pub(crate) fn curve(p: Vec2) -> Vec2 {
    let p = p * 2.0 - 1.0;
    let off = vec2(p.y.abs() / 12.0, p.x.abs() / 11.0);
    let p = (p + p * off * off) * 0.99;
    p * 0.5 + 0.5
}

pub(crate) const SEAL_FRAGMENT: &str = r#"#version 100
void main() { gl_FragColor = vec4(0.0, 0.0, 0.0, 1.0); }
"#;

pub(crate) const CRT_VERTEX: &str = r#"#version 100
attribute vec3 position;
attribute vec2 texcoord;
attribute vec4 color0;
varying highp vec2 uv;
varying lowp vec4 color;
uniform mat4 Model;
uniform mat4 Projection;
void main() {
    gl_Position = Projection * Model * vec4(position, 1);
    color = color0 / 255.0;
    uv = texcoord;
}
"#;

// Fragment floats are highp where the device has it: mediump can be 16 bits
// (NVIDIA's GLES is), too coarse for screen coordinates on a big canvas.
pub(crate) const CRT_FRAGMENT: &str = r#"#version 100
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif

varying vec2 uv;
varying lowp vec4 color;

uniform sampler2D Texture;
uniform vec4 _Time;
uniform vec2 ScreenSize;
uniform float Bloom;

// Bow 0..1 UVs into a convex tube (larger divisors = flatter); the <1 scale
// pulls the picture back so the pushed corners stay in range and the whole
// game, corners and HUD row included, stays visible.
vec2 curve(vec2 p) {
    p = p * 2.0 - 1.0;
    vec2 off = abs(p.yx) / vec2(12.0, 11.0);
    p = p + p * off * off;
    p *= 0.99;
    return p * 0.5 + 0.5;
}

// Cheap ordered dither against 8-bit banding in the dark gradients. The pixel
// is wrapped to a 64px cell: unwrapped, the dot product passes the 16-bit
// float limit toward the top right of a 1080p canvas and the result is NaN.
float dither(vec2 p) {
    return fract(sin(dot(mod(floor(p), 64.0), vec2(12.9898, 78.233))) * 43758.5453);
}

void main() {
    // The offscreen composite is stored y-flipped relative to the screen; undo
    // that here (a clean full flip, no half-texel offset from DrawTextureParams).
    vec2 fuv = vec2(uv.x, 1.0 - uv.y);

    vec2 cuv = curve(fuv);

    // The edge fade runs outward from the 0..1 boundary only, so every game
    // pixel is drawn at full weight.
    vec2 lo = smoothstep(vec2(-0.006), vec2(0.0), cuv);
    vec2 hi = smoothstep(vec2(-0.006), vec2(0.0), vec2(1.0) - cuv);
    float mask = lo.x * lo.y * hi.x * hi.y;

    vec3 col = texture2D(Texture, cuv).rgb;

    // ---- phosphor bloom: blur a ring of taps, keep the bright part, add back ----
    vec2 px = 1.0 / ScreenSize;
    vec3 b = vec3(0.0);
    b += texture2D(Texture, cuv + vec2( 1.5,  0.5) * px * 2.0).rgb;
    b += texture2D(Texture, cuv + vec2(-1.5, -0.5) * px * 2.0).rgb;
    b += texture2D(Texture, cuv + vec2( 0.5, -1.5) * px * 2.0).rgb;
    b += texture2D(Texture, cuv + vec2(-0.5,  1.5) * px * 2.0).rgb;
    b += texture2D(Texture, cuv + vec2( 2.5,  1.5) * px * 4.0).rgb;
    b += texture2D(Texture, cuv + vec2(-2.5, -1.5) * px * 4.0).rgb;
    b += texture2D(Texture, cuv + vec2( 1.5, -2.5) * px * 4.0).rgb;
    b += texture2D(Texture, cuv + vec2(-1.5,  2.5) * px * 4.0).rgb;
    b *= 0.125;
    col += max(b - 0.25, 0.0) * 1.4 * Bloom;

    // ---- tube vignette ----
    // Radial, easing only the last sliver near the glass edge, so corner HUD
    // text stays readable.
    float d = distance(cuv, vec2(0.5, 0.5));
    float vig = 1.0 - smoothstep(0.62, 1.02, d) * 0.42;
    col *= vig;

    // ---- glare on the glass (flat fuv, so it doesn't move with the curve) ----
    // Scaled by the pixel's own brightness so it only shows over lit phosphor —
    // over the black background it stays exactly black and can't band.
    float glare = smoothstep(0.7, 0.05,
        distance(fuv * vec2(1.0, 1.3), vec2(0.32, 0.30 * 1.3)));
    col += glare * 0.18 * dot(col, vec3(0.333));

    // gentle mains-hum brightness flicker + a touch of gain
    col *= 1.08 + 0.002 * sin(_Time.x * 10.7);

    // dither the near-black gradients so the vignette doesn't step
    col += (dither(gl_FragCoord.xy) - 0.5) * (1.0 / 255.0);

    gl_FragColor = vec4(col * color.rgb * mask, 1.0);
}
"#;

// Interlaced scanlines in the main blit's fragment shader, not a rectangle
// per screen row (hundreds of draw calls on a tall window). Reuses
// CRT_VERTEX.
pub(crate) const SCANLINE_FRAGMENT: &str = r#"#version 100
#ifdef GL_FRAGMENT_PRECISION_HIGH
precision highp float;
#else
precision mediump float;
#endif

varying vec2 uv;
varying lowp vec4 color;

uniform sampler2D Texture;
uniform float InterlaceField; // 0.0 or 1.0, flips every frame (Blip::interlace_field)
uniform float FieldDim;       // brightness of the fading field's rows (Blip::set_interlace)

void main() {
    vec3 col = texture2D(Texture, uv).rgb;

    // Alternate light and dark fields each frame; `composite` corrects parity for the target.
    float parity = mod(floor(gl_FragCoord.y), 2.0);
    float dim = (parity == InterlaceField) ? FieldDim : (1.0 - 60.0 / 255.0);
    col *= dim;

    gl_FragColor = vec4(col * color.rgb, 1.0);
}
"#;

/// The curved-glass post-process, or None if it does not compile.
pub(crate) fn glass() -> Option<Material> {
    load_material(
        ShaderSource::Glsl { vertex: CRT_VERTEX, fragment: CRT_FRAGMENT },
        MaterialParams {
            uniforms: vec![UniformDesc::new("ScreenSize", UniformType::Float2),
                           UniformDesc::new("Bloom", UniformType::Float1)],
            ..Default::default()
        },
    )
    .ok()
}

/// Seals the frame's alpha to 1 (writes alpha only): a translucent shape lowers the target's alpha where it
/// blends, and the frame is then blitted with alpha onto black, so glass and glows came out darkened.
pub(crate) fn seal() -> Option<Material> {
    load_material(
        ShaderSource::Glsl { vertex: CRT_VERTEX, fragment: SEAL_FRAGMENT },
        MaterialParams {
            pipeline_params: macroquad::miniquad::PipelineParams {
                color_write: (false, false, false, true),
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .ok()
}

/// The interlaced scanlines laid over the picture.
pub(crate) fn scanlines() -> Option<Material> {
    load_material(
        ShaderSource::Glsl { vertex: CRT_VERTEX, fragment: SCANLINE_FRAGMENT },
        MaterialParams {
            uniforms: vec![
                UniformDesc::new("InterlaceField", UniformType::Float1),
                UniformDesc::new("FieldDim", UniformType::Float1),
            ],
            ..Default::default()
        },
    )
    .ok()
}
