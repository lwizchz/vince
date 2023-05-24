struct CustomMaterial {
    color: vec4<f32>,
};

@group(1) @binding(0)
var<uniform> material: CustomMaterial;
@group(1) @binding(1)
var base_color_texture: texture_2d<f32>;
@group(1) @binding(2)
var base_color_sampler: sampler;

@fragment
fn fragment(
    @location(0) coord: vec4<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>
) -> @location(0) vec4<f32> {
    var texel = textureSample(base_color_texture, base_color_sampler, uv);

    // var frag = texel * vec4<f32>(1.01, 1.01, 1.01, 1.0);
    var frag = texel + vec4<f32>(0.0, 0.0, 0.01, 1.0);
    if frag.r > 1.0 {
        frag.r -= 1.0;
    }
    if frag.g > 1.0 {
        frag.g -= 1.0;
    }
    if frag.b > 1.0 {
        frag.b -= 1.0;
    }
    if frag.a > 1.0 {
        frag.a = 1.0;
    }

    return frag;
}
