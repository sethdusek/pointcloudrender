@group(0) @binding(0)
var input_image: texture_storage_2d<bgra8unorm, read>;
@group(0) @binding(1)
var output_image: texture_storage_2d<bgra8unorm, write>;
@group(0) @binding(3)
var input_depth: texture_storage_2d<r32float, read>;
@group(0) @binding(4)
var output_depth: texture_storage_2d<r32float, write>;


// Return a clamped read into the texture so we don't go out of bounds
fn c_load(coords: vec2<i32>, dimensions: vec2<u32>) -> f32 {
   let clamped: vec2<i32> = vec2<i32>(clamp(coords.x, 0, i32(dimensions.x)), clamp(coords.y, 0, i32(dimensions.y)));
   return textureLoad(input_depth, clamped).r;
}

// apparently you can't index arrays by variable
fn apply_kernel(kernel: array<f32, 9>, neighbors: array<f32, 9>) -> f32 {
   return kernel[0] * neighbors[0] + kernel[1] * neighbors[1] + kernel[2] * neighbors[2]
      + kernel[3] * neighbors[3] + kernel[4] * neighbors[4] + kernel[5] * neighbors[5]
      + kernel[6] * neighbors[6] + kernel[7] * neighbors[7] + kernel[8] * neighbors[8];
}
//TODO: set to 8x8
@compute
@workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
   let size: vec2<u32> = textureDimensions(input_image);
   let clamped_id = vec2(clamp(global_id.x, u32(0), size.x), clamp(global_id.y, u32(0), size.y));
   let load: vec4<f32> = textureLoad(input_image, global_id.xy);
   textureStore(output_image, vec2(size.x, size.y) - clamped_id, load);
}
