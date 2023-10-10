#version 460
//#define FILTER(n) float sum ## n ## = 0.0; for (int i = 0; i < 9; i++) {sum ## n ## +=neighbors[i] * kernel ## n ## [i];}
layout(local_size_x = 8, local_size_y = 8, local_size_z = 1) in;

layout(binding=3, rgba8) uniform readonly image2D input_image;
layout(binding=4, r32f) uniform readonly image2D input_depth;
layout(binding=5, rgba8) uniform writeonly image2D output_image;
layout(binding=6, r32f) uniform writeonly image2D output_depth;
//layout(binding=5, r32f) readonly uniform image2D depth;
uniform uint uWidth;
uniform uint uHeight;
layout(binding=0) uniform atomic_uint converged;


// For an 8x8 workgroup, we need 9x9 pixels of the region
shared float depths[81];
// Clamped load, make sure nothing goes out of bounds
vec4 c_load(readonly image2D image, ivec2 coords) {
    ivec2 clamped = ivec2(clamp(coords.x, 0, 8), clamp(coords.y, 0, 8));
    return vec4(vec3(depths[clamped.y * 9 + clamped.x]), 1.0);
    //return imageLoad(image, clamped);
}

float apply_kernel(float kernel[9], float neighbors[9]) {
    float sum = 0.0;
    for (int i = 0; i < 9; i++) {
        sum+=neighbors[i] * kernel[i];
    }
    return sum;
}


void main() {
    ivec2 i = ivec2(gl_GlobalInvocationID.x, gl_GlobalInvocationID.y);
    vec4 load = imageLoad(input_image, i);// / 255.0;
    vec4 depth = imageLoad(input_depth, i);
    ivec2 il = ivec2(gl_LocalInvocationID.x + 1, gl_LocalInvocationID.y + 1);
    if (il.x == 0 && il.y == 0) {

        for (int xx = -1; xx <= 1; xx++) {
            for (int yy = -1; yy <= 1; yy++) {
            }
        }
        depths[0] = imageLoad(input_depth, i + ivec2(-1, -1)).r;
        depths[1] = imageLoad(input_depth, i + ivec2(0, -1)).r;
        depths[2] = imageLoad(input_depth, i + ivec2(0, -1)).r;

    }
    depths[(gl_LocalInvocationID.y + 1) * 9 + gl_LocalInvocationID.x + 1] = depth.r;


    ivec2 offsets[9] = {
        ivec2(-1, -1), ivec2(0, -1), ivec2(1, -1),
        ivec2(-1, 0), ivec2(0, 0), ivec2(1, 0),
        ivec2(-1, 1), ivec2(0, 1), ivec2(1, 1)
    };

    barrier();
    float neighbors[9] = {
        c_load(input_depth, il + offsets[0]).r, c_load(input_depth, il + offsets[1]).r, c_load(input_depth, il + offsets[2]).r,
        c_load(input_depth, il + offsets[3]).r, c_load(input_depth, il + offsets[4]).r, c_load(input_depth, il + offsets[5]).r,
        c_load(input_depth, il + offsets[6]).r, c_load(input_depth, il + offsets[7]).r, c_load(input_depth, il + offsets[8]).r
    };

    if (abs(depth.r) > 1e-5) {
        imageStore(output_image, i, load);
        imageStore(output_depth, i, depth);
    }
    else {
        float kernel1[9] = {
        0.0, 1.0, 1.0,
        0.0, 1.0, 1.0,
        0.0, 1.0, 1.0
        };
        float kernel2[9] = {
        1.0, 1.0, 1.0,
        1.0, 1.0, 1.0,
        0.0, 0.0, 0.0
        };
        float kernel3[9] = {
        1.0, 1.0, 0.0,
        1.0, 1.0, 0.0,
        1.0, 1.0, 0.0
        };
        float kernel4[9] = {
        0.0, 0.0, 0.0,
        1.0, 1.0, 1.0,
        1.0, 1.0, 1.0,
        };
        float kernel5[9] = {
        1.0, 1.0, 1.0,
        0.0, 1.0, 1.0,
        0.0, 0.0, 1.0,
        };
        float kernel6[9] = {
        1.0, 1.0, 1.0,
        1.0, 1.0, 0.0,
        1.0, 0.0, 0.0,
        };
        float kernel7[9] = {
        1.0, 0.0, 0.0,
        1.0, 1.0, 0.0,
        1.0, 1.0, 1.0,
        };
        float kernel8[9] = {
        0.0, 0.0, 1.0,
        0.0, 1.0, 1.0,
        1.0, 1.0, 1.0,
        };

        float sum1 = apply_kernel(kernel1, neighbors);
        float sum2 = apply_kernel(kernel2, neighbors);
        float sum3 = apply_kernel(kernel3, neighbors);
        float sum4 = apply_kernel(kernel4, neighbors);
        float sum5 = apply_kernel(kernel5, neighbors);
        float sum6 = apply_kernel(kernel6, neighbors);
        float sum7 = apply_kernel(kernel7, neighbors);
        float sum8 = apply_kernel(kernel8, neighbors);

        float prod = sum1 * sum2 * sum3 * sum4 * sum5 * sum6 * sum7 * sum8;

        if (abs(prod) > 1e-5) {
            int min_idx = 0;
            float min_depth = 9999.0;
            for (int i = 0; i < 9; i++) {
                if (neighbors[i] < min_depth) {
                    min_idx = i;
                    min_depth = min_depth;
                }
            }
            imageStore(output_image, i, imageLoad(input_image, i + offsets[min_idx]));
            imageStore(output_depth, i, imageLoad(input_depth, i + offsets[min_idx]));
            // Mark background filling as incomplete
            // atomicOr isn't working for some reason so this will have to do
            atomicCounterMax(converged, uint(1));
        }
        else {
            imageStore(output_image, i, load);
            imageStore(output_depth, i, depth);
        }
    }

}
