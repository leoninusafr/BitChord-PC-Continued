//! GPU Liquid Glass shader effect inspired by `MasterKPG/rust-glass`.
//!
//! Captures the live framebuffer region behind floating UI surfaces (top bar, search pill,
//! player bar, sidebar) and renders Apple-style squircle SDF refraction, edge chromatic
//! dispersion, blurred backdrop sampling, directional virtual lighting, and a specular rim.
//! Can be toggled on/off at runtime in Settings › Appearance to trade visual depth for GPU power.

use std::sync::{Arc, Mutex};

use eframe::glow::{self, HasContext};

#[derive(Clone, Copy)]
pub struct GlassStyle {
    /// Corner radius in logical points (converted to squircle curvature in shader).
    pub radius: f32,
    /// Backdrop blur radius in logical points.
    pub blur: f32,
    /// Tint color (RGB + tint strength alpha).
    pub tint: [f32; 4],
    /// Refraction depth and bend strength in points.
    pub refraction: f32,
    /// Chromatic dispersion intensity (0 = none).
    pub dispersion: f32,
    /// Specular rim highlight intensity.
    pub rim: f32,
    /// Dark edge contrast intensity.
    pub dark_edge: f32,
    /// Backdrop saturation multiplier.
    pub saturation: f32,
    /// Contrast floor/ceiling for face luminance mapping.
    pub face_lo: f32,
    pub face_hi: f32,
    /// Brightness lift / press glow.
    pub brightness: f32,
}

impl GlassStyle {
    pub fn bar(dark: bool) -> Self {
        if dark {
            Self {
                radius: 0.0,
                blur: 14.0,
                tint: [0.06, 0.07, 0.09, 0.52],
                refraction: 10.0,
                dispersion: 0.08,
                rim: 0.22,
                dark_edge: 0.14,
                saturation: 1.25,
                face_lo: 0.02,
                face_hi: 0.90,
                brightness: 0.0,
            }
        } else {
            Self {
                radius: 0.0,
                blur: 14.0,
                tint: [0.95, 0.96, 0.98, 0.62],
                refraction: 10.0,
                dispersion: 0.08,
                rim: 0.35,
                dark_edge: 0.06,
                saturation: 1.15,
                face_lo: 0.08,
                face_hi: 0.98,
                brightness: 0.0,
            }
        }
    }

    pub fn pill(dark: bool, focused: bool) -> Self {
        if dark {
            Self {
                radius: 17.0,
                blur: 10.0,
                tint: [0.11, 0.12, 0.15, if focused { 0.44 } else { 0.36 }],
                refraction: 13.0,
                dispersion: 0.16,
                rim: if focused { 0.55 } else { 0.34 },
                dark_edge: 0.18,
                saturation: 1.35,
                face_lo: 0.04,
                face_hi: 0.94,
                brightness: if focused { 0.04 } else { 0.0 },
            }
        } else {
            Self {
                radius: 17.0,
                blur: 10.0,
                tint: [0.97, 0.98, 1.0, if focused { 0.65 } else { 0.55 }],
                refraction: 12.0,
                dispersion: 0.14,
                rim: if focused { 0.72 } else { 0.45 },
                dark_edge: 0.06,
                saturation: 1.25,
                face_lo: 0.10,
                face_hi: 1.0,
                brightness: if focused { 0.03 } else { 0.0 },
            }
        }
    }

    pub fn panel(dark: bool, radius: f32) -> Self {
        let mut s = Self::bar(dark);
        s.radius = radius;
        s.rim = if dark { 0.26 } else { 0.40 };
        s.refraction = 11.0;
        s.dispersion = 0.10;
        s
    }
}

const VERTEX: &str = r#"
OUT vec2 v_px;
uniform vec4 u_rect;      // x0, y0, x1, y1 in bottom-left GL framebuffer pixels
uniform vec2 u_screen;    // framebuffer size in pixels

void main() {
  // 2 triangles covering u_rect
  int idx = gl_VertexID;
  vec2 c = vec2(
    (idx == 1 || idx == 2 || idx == 4) ? 1.0 : 0.0,
    (idx == 2 || idx == 3 || idx == 5) ? 1.0 : 0.0
  );
  vec2 px = mix(u_rect.xy, u_rect.zw, c);
  v_px = px;
  vec2 ndc = (px / u_screen) * 2.0 - 1.0;
  gl_Position = vec4(ndc, 0.0, 1.0);
}
"#;

const FRAGMENT: &str = r#"
precision highp float;
uniform sampler2D u_backdrop;
uniform vec4 u_rect;       // x0, y0, x1, y1 in GL pixels
uniform vec4 u_copy_uv;    // min_u, min_v, max_u, max_v valid in backdrop
uniform vec2 u_screen;
uniform float u_ppp;
uniform float u_radius;    // px
uniform float u_blur;      // px
uniform float u_refract;   // px
uniform float u_disperse;
uniform float u_rim;
uniform float u_dark_edge;
uniform float u_sat;
uniform float u_face_lo;
uniform float u_face_hi;
uniform float u_bright;
uniform vec4 u_tint;       // rgb, alpha
uniform float u_dark;      // 1.0 dark mode, 0.0 light mode

IN vec2 v_px;
#if NEW_SHADER_INTERFACE
out vec4 frag_color;
#define FRAG frag_color
#define TEX texture
#else
#define FRAG gl_FragColor
#define TEX texture2D
#endif

float luma(vec3 c) {
  return dot(c, vec3(0.299, 0.587, 0.114));
}

vec3 vibrance(vec3 c, float sat) {
  return clamp(mix(vec3(luma(c)), c, sat), vec3(0.0), vec3(1.0));
}

// Continuous curvature squircle SDF (from MasterKPG/rust-glass common.wgsl)
vec3 sd_squircle(vec2 p, vec2 half_sz, vec2 r_in, float n) {
  vec2 r = max(min(r_in, half_sz), vec2(0.001));
  vec2 s = vec2(p.x >= 0.0 ? 1.0 : -1.0, p.y >= 0.0 ? 1.0 : -1.0);
  vec2 q = abs(p) - (half_sz - r);
  if (q.x > 0.0 && q.y > 0.0) {
    vec2 u = q / r;
    float l = pow(pow(u.x, n) + pow(u.y, n), 1.0 / n);
    float k = pow(max(l, 1e-5), 1.0 - n);
    vec2 g = vec2(pow(u.x, n - 1.0), pow(u.y, n - 1.0)) * k / r;
    float gl = max(length(g), 1e-5);
    return vec3((l - 1.0) / gl, s * g / gl);
  }
  vec2 e = q - r;
  if (e.x > e.y) {
    return vec3(e.x, s.x, 0.0);
  }
  return vec3(e.y, 0.0, s.y);
}

float bevel(float t) {
  float u = 1.0 - clamp(t, 0.0, 1.0);
  return 1.0 - sqrt(max(1.0 - u * u, 0.0));
}

float gauss(float x) {
  return exp(-x * x);
}

vec3 sample_blur(vec2 px, float radius) {
  vec2 uv_center = clamp(px / u_screen, u_copy_uv.xy, u_copy_uv.zw);
  if (radius < 0.75) {
    return TEX(u_backdrop, uv_center).rgb;
  }
  vec2 step1 = vec2(radius * 0.45) / u_screen;
  vec2 step2 = vec2(radius * 0.95) / u_screen;
  vec3 acc = TEX(u_backdrop, uv_center).rgb * 0.20;
  acc += TEX(u_backdrop, clamp(uv_center + vec2( step1.x,  step1.y), u_copy_uv.xy, u_copy_uv.zw)).rgb * 0.10;
  acc += TEX(u_backdrop, clamp(uv_center + vec2(-step1.x,  step1.y), u_copy_uv.xy, u_copy_uv.zw)).rgb * 0.10;
  acc += TEX(u_backdrop, clamp(uv_center + vec2( step1.x, -step1.y), u_copy_uv.xy, u_copy_uv.zw)).rgb * 0.10;
  acc += TEX(u_backdrop, clamp(uv_center + vec2(-step1.x, -step1.y), u_copy_uv.xy, u_copy_uv.zw)).rgb * 0.10;
  acc += TEX(u_backdrop, clamp(uv_center + vec2( step2.x,  0.0),     u_copy_uv.xy, u_copy_uv.zw)).rgb * 0.075;
  acc += TEX(u_backdrop, clamp(uv_center + vec2(-step2.x,  0.0),     u_copy_uv.xy, u_copy_uv.zw)).rgb * 0.075;
  acc += TEX(u_backdrop, clamp(uv_center + vec2( 0.0,      step2.y), u_copy_uv.xy, u_copy_uv.zw)).rgb * 0.075;
  acc += TEX(u_backdrop, clamp(uv_center + vec2( 0.0,     -step2.y), u_copy_uv.xy, u_copy_uv.zw)).rgb * 0.075;
  acc += TEX(u_backdrop, clamp(uv_center + vec2( step2.x * 0.7,  step2.y * 0.7), u_copy_uv.xy, u_copy_uv.zw)).rgb * 0.025;
  acc += TEX(u_backdrop, clamp(uv_center + vec2(-step2.x * 0.7,  step2.y * 0.7), u_copy_uv.xy, u_copy_uv.zw)).rgb * 0.025;
  acc += TEX(u_backdrop, clamp(uv_center + vec2( step2.x * 0.7, -step2.y * 0.7), u_copy_uv.xy, u_copy_uv.zw)).rgb * 0.025;
  acc += TEX(u_backdrop, clamp(uv_center + vec2(-step2.x * 0.7, -step2.y * 0.7), u_copy_uv.xy, u_copy_uv.zw)).rgb * 0.025;
  return acc;
}

void main() {
  vec2 center = (u_rect.xy + u_rect.zw) * 0.5;
  vec2 half_sz = max((u_rect.zw - u_rect.xy) * 0.5, vec2(1.0));
  vec2 p = v_px - center;

  float r = min(u_radius, min(half_sz.x, half_sz.y));
  vec3 sdf = (r > 0.5)
    ? sd_squircle(p, half_sz, vec2(r), 2.65)
    : vec3(max(abs(p.x) - half_sz.x, abs(p.y) - half_sz.y), 0.0, p.y >= 0.0 ? 1.0 : -1.0);

  float d = sdf.x;
  vec2 n = sdf.yz;
  float cov = smoothstep(0.72, -0.72, d);
  if (cov <= 0.001) {
    discard;
  }

  float depth = max(-d, 0.0);
  float inner_h = max(min(u_refract * 1.4, min(half_sz.x, half_sz.y) * 0.7), 2.0);
  float shift = -u_refract * bevel(depth / inner_h);

  // Virtual top-left light vector in GL pixel coords (y up)
  vec2 light = normalize(vec2(-0.55, 0.83));
  float facing = dot(n, light);
  float near = max(facing, 0.0);
  float far = max(-facing, 0.0);

  float blur = u_blur * mix(0.42, 1.0, smoothstep(0.0, inner_h, depth));
  float disp = u_disperse * mix(1.0, -0.55, smoothstep(0.0, 0.5, far));

  vec3 c;
  if (abs(shift * disp) > 0.25 && r > 0.5) {
    c.r = sample_blur(v_px + n * shift * (1.0 + disp), blur).r;
    c.g = sample_blur(v_px + n * shift, blur).g;
    c.b = sample_blur(v_px + n * shift * (1.0 - disp), blur).b;
  } else {
    c = sample_blur(v_px + n * shift, blur);
  }

  c = vibrance(c, u_sat);
  float lum = luma(c);
  c = vec3(u_face_lo) + c * (u_face_hi - u_face_lo);

  float ta = u_tint.a;
  if (ta > 0.0) {
    vec3 toned = u_tint.rgb * (0.86 + 0.24 * lum) + vec3(0.04 * lum);
    c = mix(c, toned, ta);
  }

  float body = max(min(min(half_sz.x, half_sz.y) * 0.9, 32.0 * u_ppp), 1.0);
  float e = 1.0 - smoothstep(0.0, body, depth);
  c *= 1.0 - 0.14 * e * e * near;
  c = mix(c, vec3(1.0), 0.10 * e * e * far);

  // Dark inner outline for contrast on bright backgrounds
  float edge = gauss(max(depth - 0.18 * u_ppp, 0.0) / max(0.45 * u_ppp, 0.7));
  if (edge > 0.01 && u_dark_edge > 0.0) {
    float k = u_dark_edge * edge * (1.0 - 0.85 * smoothstep(0.1, 0.8, far));
    c = mix(c, c * 0.45, k);
  }

  // Specular Liquid Glass rim highlight
  if (u_rim > 0.0) {
    float rw = max(1.1 * u_ppp, 1.0);
    float line = gauss((depth - rw * 0.55) / (rw * 0.68));
    float soft = exp(-depth / (rw * 3.0));
    float profile = 0.58 * line + 0.42 * soft * (1.0 - 0.45 * u_dark);
    float dir = (1.0 - 0.75 * u_dark_edge) * pow(near, 1.5) + pow(far, 1.2);
    float key = 0.32 + 0.68 * min(dir, 1.0);
    vec3 rim_rgb = mix(vec3(1.0), min(vibrance(c, 1.35) * 1.45 + 0.15, vec3(1.0)), 0.25);
    float rim_a = clamp(u_rim * key * profile, 0.0, 1.0);
    c = mix(c, rim_rgb, rim_a);
  }

  c = mix(c, vec3(1.0), u_bright);
  c = clamp(c, vec3(0.0), vec3(1.0));
  FRAG = vec4(c * cov, cov);
}
"#;

struct GlGlass {
    program: glow::Program,
    vao: glow::VertexArray,
    backdrop: glow::Texture,
    tex_size: [i32; 2],
    loc_rect: Option<glow::UniformLocation>,
    loc_copy_uv: Option<glow::UniformLocation>,
    loc_screen: Option<glow::UniformLocation>,
    loc_ppp: Option<glow::UniformLocation>,
    loc_radius: Option<glow::UniformLocation>,
    loc_blur: Option<glow::UniformLocation>,
    loc_refract: Option<glow::UniformLocation>,
    loc_disperse: Option<glow::UniformLocation>,
    loc_rim: Option<glow::UniformLocation>,
    loc_dark_edge: Option<glow::UniformLocation>,
    loc_sat: Option<glow::UniformLocation>,
    loc_face_lo: Option<glow::UniformLocation>,
    loc_face_hi: Option<glow::UniformLocation>,
    loc_bright: Option<glow::UniformLocation>,
    loc_tint: Option<glow::UniformLocation>,
    loc_dark: Option<glow::UniformLocation>,
    loc_backdrop: Option<glow::UniformLocation>,
}

#[derive(Clone)]
pub struct LiquidGlass {
    gl: Arc<Mutex<Option<GlGlass>>>,
}

impl LiquidGlass {
    pub fn new(gl: &glow::Context) -> anyhow::Result<Self> {
        let version = eframe::egui_glow::ShaderVersion::get(gl);
        let header = format!(
            "{}\n#define NEW_SHADER_INTERFACE {}\n{}\n",
            version.version_declaration(),
            version.is_new_shader_interface() as i32,
            if version.is_new_shader_interface() {
                "#define IN in\n#define OUT out"
            } else {
                "#define IN varying\n#define OUT varying"
            }
        );
        unsafe {
            let program = gl.create_program().map_err(anyhow::Error::msg)?;
            let mut shaders = Vec::new();
            for (kind, src) in [(glow::VERTEX_SHADER, VERTEX), (glow::FRAGMENT_SHADER, FRAGMENT)] {
                let s = gl.create_shader(kind).map_err(anyhow::Error::msg)?;
                gl.shader_source(s, &format!("{header}{src}"));
                gl.compile_shader(s);
                anyhow::ensure!(gl.get_shader_compile_status(s), "glass shader: {}", gl.get_shader_info_log(s));
                gl.attach_shader(program, s);
                shaders.push(s);
            }
            gl.link_program(program);
            anyhow::ensure!(gl.get_program_link_status(program), "glass link: {}", gl.get_program_info_log(program));
            for s in shaders {
                gl.detach_shader(program, s);
                gl.delete_shader(s);
            }
            let vao = gl.create_vertex_array().map_err(anyhow::Error::msg)?;
            let backdrop = gl.create_texture().map_err(anyhow::Error::msg)?;
            gl.bind_texture(glow::TEXTURE_2D, Some(backdrop));
            for (p, v) in [
                (glow::TEXTURE_MIN_FILTER, glow::LINEAR),
                (glow::TEXTURE_MAG_FILTER, glow::LINEAR),
                (glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE),
                (glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE),
            ] {
                gl.tex_parameter_i32(glow::TEXTURE_2D, p, v as i32);
            }
            let g = GlGlass {
                loc_rect: gl.get_uniform_location(program, "u_rect"),
                loc_copy_uv: gl.get_uniform_location(program, "u_copy_uv"),
                loc_screen: gl.get_uniform_location(program, "u_screen"),
                loc_ppp: gl.get_uniform_location(program, "u_ppp"),
                loc_radius: gl.get_uniform_location(program, "u_radius"),
                loc_blur: gl.get_uniform_location(program, "u_blur"),
                loc_refract: gl.get_uniform_location(program, "u_refract"),
                loc_disperse: gl.get_uniform_location(program, "u_disperse"),
                loc_rim: gl.get_uniform_location(program, "u_rim"),
                loc_dark_edge: gl.get_uniform_location(program, "u_dark_edge"),
                loc_sat: gl.get_uniform_location(program, "u_sat"),
                loc_face_lo: gl.get_uniform_location(program, "u_face_lo"),
                loc_face_hi: gl.get_uniform_location(program, "u_face_hi"),
                loc_bright: gl.get_uniform_location(program, "u_bright"),
                loc_tint: gl.get_uniform_location(program, "u_tint"),
                loc_dark: gl.get_uniform_location(program, "u_dark"),
                loc_backdrop: gl.get_uniform_location(program, "u_backdrop"),
                program,
                vao,
                backdrop,
                tex_size: [0, 0],
            };
            Ok(Self { gl: Arc::new(Mutex::new(Some(g))) })
        }
    }

    pub fn paint(&self, painter: &egui::Painter, rect: egui::Rect, style: GlassStyle, dark: bool) {
        if rect.width() < 4.0 || rect.height() < 4.0 {
            return;
        }
        let state = self.gl.clone();
        let cb = eframe::egui_glow::CallbackFn::new(move |info, painter| {
            let mut guard = state.lock().unwrap();
            let Some(g) = guard.as_mut() else { return };
            let screen_w = info.screen_size_px[0] as i32;
            let screen_h = info.screen_size_px[1] as i32;
            if screen_w < 4 || screen_h < 4 {
                return;
            }
            let ppp = info.pixels_per_point;
            let x0 = (rect.left() * ppp).floor().max(0.0);
            let x1 = (rect.right() * ppp).ceil().min(screen_w as f32);
            // egui y=0 is top, GL y=0 is bottom
            let y1 = (screen_h as f32 - rect.top() * ppp).ceil().min(screen_h as f32);
            let y0 = (screen_h as f32 - rect.bottom() * ppp).floor().max(0.0);
            if x1 <= x0 || y1 <= y0 {
                return;
            }

            let pad = ((style.blur + style.refraction + 4.0) * ppp).ceil() as i32;
            let cx0 = (x0 as i32 - pad).max(0);
            let cy0 = (y0 as i32 - pad).max(0);
            let cx1 = (x1 as i32 + pad).min(screen_w);
            let cy1 = (y1 as i32 + pad).min(screen_h);
            let cw = (cx1 - cx0).max(1);
            let ch = (cy1 - cy0).max(1);

            let gl = painter.gl();
            unsafe {
                gl.active_texture(glow::TEXTURE0);
                gl.bind_texture(glow::TEXTURE_2D, Some(g.backdrop));
                if g.tex_size != [screen_w, screen_h] {
                    gl.tex_image_2d(
                        glow::TEXTURE_2D,
                        0,
                        glow::RGBA8 as i32,
                        screen_w,
                        screen_h,
                        0,
                        glow::RGBA,
                        glow::UNSIGNED_BYTE,
                        glow::PixelUnpackData::Slice(None),
                    );
                    g.tex_size = [screen_w, screen_h];
                }
                gl.copy_tex_sub_image_2d(glow::TEXTURE_2D, 0, cx0, cy0, cx0, cy0, cw, ch);

                gl.use_program(Some(g.program));
                gl.bind_vertex_array(Some(g.vao));
                gl.uniform_1_i32(g.loc_backdrop.as_ref(), 0);
                gl.uniform_4_f32(g.loc_rect.as_ref(), x0, y0, x1, y1);
                let min_u = (cx0 as f32 + 1.0) / screen_w as f32;
                let min_v = (cy0 as f32 + 1.0) / screen_h as f32;
                let max_u = (cx1 as f32 - 1.0).max(cx0 as f32 + 1.0) / screen_w as f32;
                let max_v = (cy1 as f32 - 1.0).max(cy0 as f32 + 1.0) / screen_h as f32;
                gl.uniform_4_f32(g.loc_copy_uv.as_ref(), min_u, min_v, max_u, max_v);
                gl.uniform_2_f32(g.loc_screen.as_ref(), screen_w as f32, screen_h as f32);
                gl.uniform_1_f32(g.loc_ppp.as_ref(), ppp);
                gl.uniform_1_f32(g.loc_radius.as_ref(), style.radius * ppp);
                gl.uniform_1_f32(g.loc_blur.as_ref(), style.blur * ppp);
                gl.uniform_1_f32(g.loc_refract.as_ref(), style.refraction * ppp);
                gl.uniform_1_f32(g.loc_disperse.as_ref(), style.dispersion);
                gl.uniform_1_f32(g.loc_rim.as_ref(), style.rim);
                gl.uniform_1_f32(g.loc_dark_edge.as_ref(), style.dark_edge);
                gl.uniform_1_f32(g.loc_sat.as_ref(), style.saturation);
                gl.uniform_1_f32(g.loc_face_lo.as_ref(), style.face_lo);
                gl.uniform_1_f32(g.loc_face_hi.as_ref(), style.face_hi);
                gl.uniform_1_f32(g.loc_bright.as_ref(), style.brightness);
                gl.uniform_4_f32(g.loc_tint.as_ref(), style.tint[0], style.tint[1], style.tint[2], style.tint[3]);
                gl.uniform_1_f32(g.loc_dark.as_ref(), if dark { 1.0 } else { 0.0 });

                gl.draw_arrays(glow::TRIANGLES, 0, 6);
            }
        });
        painter.add(egui::PaintCallback { rect, callback: Arc::new(cb) });
    }

    pub fn destroy(&self, gl: &glow::Context) {
        if let Some(g) = self.gl.lock().unwrap().take() {
            unsafe {
                gl.delete_program(g.program);
                gl.delete_vertex_array(g.vao);
                gl.delete_texture(g.backdrop);
            }
        }
    }
}
