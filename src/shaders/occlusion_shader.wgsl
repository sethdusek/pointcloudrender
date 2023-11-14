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
   // Missing pixels have an alpha of 1. To avoid them being considered for occlusion shading we return an absurdly high depth value
   return textureLoad(input_depth, clamped).r + 10000.0 * textureLoad(input_image, clamped).a;
}

fn apply_kernel(kernel: array<f32, 9>, neighbors: array<f32, 9>) -> f32 {
   return kernel[0] * step(neighbors[0], neighbors[4]) + kernel[1] * step(neighbors[1], neighbors[4]) + kernel[2] * step(neighbors[2], neighbors[4])
      + kernel[3] * step(neighbors[3], neighbors[4]) + kernel[4] * step(neighbors[4], neighbors[4]) + kernel[5] * step(neighbors[5], neighbors[4])
      + kernel[6] * step(neighbors[6], neighbors[4]) + kernel[7] * step(neighbors[7], neighbors[4]) + kernel[8] * step(neighbors[8], neighbors[4]);
}

//TODO: set to 8x8
@compute
@workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
   let size: vec2<u32> = textureDimensions(input_image);

   let load: vec4<f32> = textureLoad(input_image, global_id.xy);
   let id: vec2<i32> = vec2<i32>(i32(global_id.x), i32(global_id.y));
   var offsets = array<vec2<i32>, 9>(
      vec2(-1, -1), vec2(0, -1), vec2(1, -1),
      vec2(-1, 0), vec2(0, 0), vec2(1, 0),
      vec2(-1, 1), vec2(0, 1), vec2(1, 1)
   );
   var neighbors = array<f32, 9>(
      c_load(id + offsets[0], size), c_load(id + offsets[1], size), c_load(id + offsets[2], size),
      c_load(id + offsets[3], size), c_load(id + offsets[4], size), c_load(id + offsets[5], size),
      c_load(id + offsets[6], size), c_load(id + offsets[7], size), c_load(id + offsets[8], size)
   );

   if (abs(neighbors[4]) < 1e-9 || load.a == 1.0) {
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


      // 0 Should be good to use here as all our kernel results will be 1 or 0
      if abs(prod) > 0.0 {
         var min_idx = 4;
         var min_depth = 0.0;
         var i = 0;
         loop {
            if i == 9 { break; }
            // If diff is positive, that means the pixel we're on should be occluded by a neighboring pixel that's closer
            let diff = neighbors[4] - neighbors[i];
            if diff > 1e-2 && diff > min_depth {
                  min_idx = i;
                  min_depth = diff;
            }
            i++;
         }
         textureStore(output_image, global_id.xy, textureLoad(input_image, id + offsets[min_idx]));
         textureStore(output_depth, global_id.xy, vec4(neighbors[min_idx]));
      }
      else {
         textureStore(output_image, global_id.xy, load);
         textureStore(output_depth, global_id.xy, vec4(neighbors[4]));
      }

   }
}
