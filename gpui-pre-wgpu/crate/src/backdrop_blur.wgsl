// Backdrop blur (`BackdropBlur` in gpui's scene): what was drawn within a
// region is copied out of the frame, scaled down, blurred with a separable
// Gaussian, and drawn back scaled up, clipped to the region's rounded
// rectangle. See `backdrop_blur.rs` for the passes.

struct BlurParams {
    // The sampled texture's size, in its pixels.
    source_size: vec2<f32>,
    // The part of the sampled texture holding valid content, in its pixels.
    // Samples are clamped to it.
    valid_min: vec2<f32>,
    valid_max: vec2<f32>,
    // Blur passes: one texel along x or y.
    direction: vec2<f32>,
    // Blur passes: the Gaussian's standard deviation, in source pixels.
    sigma: f32,
    // How many frame pixels one pixel of the scaled-down textures covers.
    factor: f32,
    // Composite: the rounded rectangle drawn into, in frame pixels.
    bounds_origin: vec2<f32>,
    bounds_size: vec2<f32>,
    // Composite: top left, top right, bottom right, bottom left.
    corner_radii: vec4<f32>,
}

@group(0) @binding(0) var t_source: texture_2d<f32>;
@group(0) @binding(1) var s_source: sampler;
@group(0) @binding(2) var<uniform> params: BlurParams;

// One triangle covering the whole target; the scissor rectangle limits it.
@vertex
fn vs_fullscreen(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0);
}

fn sample_clamped(coord: vec2<f32>) -> vec4<f32> {
    let lo = params.valid_min + vec2<f32>(0.5);
    let hi = max(params.valid_max - vec2<f32>(0.5), lo);
    let c = clamp(coord, lo, hi);
    return textureSampleLevel(t_source, s_source, c / params.source_size, 0.0);
}

// Frame copy → scaled-down texture: each output pixel averages the
// `factor`×`factor` frame pixels it covers, two by two with bilinear samples.
@fragment
fn fs_downsample(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let factor = params.factor;
    let origin = floor(position.xy) * factor;
    if (factor < 2.0) {
        return sample_clamped(origin + vec2<f32>(0.5));
    }
    let steps = i32(factor / 2.0);
    var sum = vec4<f32>(0.0);
    for (var y = 0; y < steps; y++) {
        for (var x = 0; x < steps; x++) {
            let at = origin + vec2<f32>(f32(x) * 2.0 + 1.0, f32(y) * 2.0 + 1.0);
            sum += sample_clamped(at);
        }
    }
    return sum / f32(steps * steps);
}

// One direction of the separable Gaussian.
@fragment
fn fs_blur(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let sigma = max(params.sigma, 0.0001);
    let reach = min(i32(ceil(sigma * 3.0)), 48);
    let center = position.xy;
    var sum = vec4<f32>(0.0);
    var weights = 0.0;
    for (var i = -reach; i <= reach; i++) {
        let offset = f32(i);
        let weight = exp(-(offset * offset) / (2.0 * sigma * sigma));
        sum += sample_clamped(center + params.direction * offset) * weight;
        weights += weight;
    }
    return sum / weights;
}

// The distance from `point` to the rounded rectangle's edge, negative inside.
fn rounded_rect_distance(point: vec2<f32>) -> f32 {
    let half_size = params.bounds_size * 0.5;
    let center = params.bounds_origin + half_size;
    let p = point - center;
    var radius: f32;
    if (p.x < 0.0) {
        if (p.y < 0.0) { radius = params.corner_radii.x; } else { radius = params.corner_radii.w; }
    } else {
        if (p.y < 0.0) { radius = params.corner_radii.y; } else { radius = params.corner_radii.z; }
    }
    radius = min(radius, min(half_size.x, half_size.y));
    let q = abs(p) - half_size + vec2<f32>(radius);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

// Blurred texture → frame, scaled up, within the rounded rectangle.
// Premultiplied: drawn with premultiplied alpha blending.
@fragment
fn fs_composite(@builtin(position) position: vec4<f32>) -> @location(0) vec4<f32> {
    let coverage = clamp(0.5 - rounded_rect_distance(position.xy), 0.0, 1.0);
    if (coverage <= 0.0) {
        discard;
    }
    let color = sample_clamped(position.xy / params.factor);
    return color * coverage;
}
