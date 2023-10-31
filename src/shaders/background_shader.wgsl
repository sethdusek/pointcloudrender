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
      + kernel[6] * neighbors[6] + kernel[7] * neighbors[7] + kernel[7] * neighbors[7];
}
//TODO: set to 8x8
@compute
@workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
   let size: vec2<u32> = textureDimensions(input_image);

   let load: vec4<f32> = textureLoad(input_image, global_id.xy);
   let i: vec2<i32> = vec2<i32>(i32(global_id.x), i32(global_id.y));
   var offsets = array<vec2<i32>, 9>(
        vec2(-1, -1), vec2(0, -1), vec2(1, -1),
        vec2(-1, 0), vec2(0, 0), vec2(1, 0),
        vec2(-1, 1), vec2(0, 1), vec2(1, 1)
   );
   var neighbors = array<f32, 9>(
      c_load(i + offsets[0], size), c_load(i + offsets[1], size), c_load(i + offsets[2], size),
      c_load(i + offsets[3], size), c_load(i + offsets[4], size), c_load(i + offsets[5], size),
      c_load(i + offsets[6], size), c_load(i + offsets[7], size), c_load(i + offsets[8], size)
   );

   if (abs(neighbors[4]) > 1e-8) {
      textureStore(output_image, global_id.xy, load);
      textureStore(output_depth, global_id.xy, vec4(neighbors[4]));
   }
   else {
      let kernel1 = array<f32, 9>(
        0.0, 1.0, 1.0,
        0.0, 1.0, 1.0,
        0.0, 1.0, 1.0
      );
      let kernel2 = array<f32, 9>(
        1.0, 1.0, 1.0,
        1.0, 1.0, 1.0,
        0.0, 0.0, 0.0
      );
      let kernel3 = array<f32, 9>(
        1.0, 1.0, 0.0,
        1.0, 1.0, 0.0,
        1.0, 1.0, 0.0
      );
      let kernel4 = array<f32, 9>(
        0.0, 0.0, 0.0,
        1.0, 1.0, 1.0,
        1.0, 1.0, 1.0,
      );
      let kernel5 = array<f32, 9>(
        1.0, 1.0, 1.0,
        0.0, 1.0, 1.0,
        0.0, 0.0, 1.0,
      );
      let kernel6 = array<f32, 9>(
        1.0, 1.0, 1.0,
        1.0, 1.0, 0.0,
        1.0, 0.0, 0.0,
      );
      let kernel7 = array<f32, 9>(
        1.0, 0.0, 0.0,
        1.0, 1.0, 0.0,
        1.0, 1.0, 1.0,
      );
      let kernel8 = array<f32, 9>(
        0.0, 0.0, 1.0,
        0.0, 1.0, 1.0,
        1.0, 1.0, 1.0,
      );

      let sum1 = apply_kernel(kernel1, neighbors);
      let sum2 = apply_kernel(kernel2, neighbors);
      let sum3 = apply_kernel(kernel3, neighbors);
      let sum4 = apply_kernel(kernel4, neighbors);
      let sum5 = apply_kernel(kernel5, neighbors);
      let sum6 = apply_kernel(kernel6, neighbors);
      let sum7 = apply_kernel(kernel7, neighbors);
      let sum8 = apply_kernel(kernel8, neighbors);

      let prod = sum1 * sum2 * sum3 * sum4 * sum5 * sum6 * sum7 * sum8;

      var min_idx = 4;

      if abs(prod) > 1e-7 {
            // TODO: remove this monstrosity. turns out you *can* index arrays by index if you declare it with var instead of let
            var min_depth = 9999.0;
            if neighbors[0] < min_depth { min_idx = 0; min_depth = neighbors[0]; }
            if neighbors[1] < min_depth { min_idx = 1; min_depth = neighbors[1]; }
            if neighbors[2] < min_depth { min_idx = 2; min_depth = neighbors[2]; }
            if neighbors[3] < min_depth { min_idx = 3; min_depth = neighbors[3]; }
            if neighbors[4] < min_depth { min_idx = 4; min_depth = neighbors[4]; }
            if neighbors[5] < min_depth { min_idx = 5; min_depth = neighbors[5]; }
            if neighbors[6] < min_depth { min_idx = 6; min_depth = neighbors[6]; }
            if neighbors[7] < min_depth { min_idx = 7; min_depth = neighbors[7]; }
            if neighbors[8] < min_depth { min_idx = 8; min_depth = neighbors[8]; }
      }



      textureStore(output_image, global_id.xy, textureLoad(input_image, i + offsets[min_idx]));
      textureStore(output_depth, global_id.xy, vec4(neighbors[min_idx]));
   }
}
