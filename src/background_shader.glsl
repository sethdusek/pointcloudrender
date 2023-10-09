#version 460
//#define FILTER(n) float sum ## n ## = 0.0; for (int i = 0; i < 9; i++) {sum ## n ## +=neighbors[i] * kernel ## n ## [i];}
layout(local_size_x = 1, local_size_y = 1, local_size_z = 1) in;

layout(binding=3, rgba8) uniform readonly image2D input_image;
layout(binding=4, r32f) uniform readonly image2D input_depth;
layout(binding=5, rgba8) uniform writeonly image2D output_image;
layout(binding=6, r32f) uniform writeonly image2D output_depth;
//layout(binding=5, r32f) readonly uniform image2D depth;
uniform uint uWidth;
uniform uint uHeight;
layout(binding=0) uniform atomic_uint converged;

// Clamped load, make sure nothing goes out of bounds
vec4 c_load(readonly image2D image, ivec2 coords) {
    ivec2 clamped = ivec2(clamp(coords.x, 0, uWidth), clamp(coords.y, 0, uHeight));
    return imageLoad(image, clamped);
}

float apply_kernel(float kernel[9], vec4 neighbors[9]) {
    float sum = 0.0;
    for (int i = 0; i < 9; i++) {
        sum+=neighbors[i].r * kernel[i];
    }
    return sum;
}

void main() {
    ivec2 i = ivec2(gl_GlobalInvocationID.x, gl_GlobalInvocationID.y);
    vec4 load = imageLoad(input_image, i);// / 255.0;
    vec4 depth = imageLoad(input_depth, i);

    ivec2 offsets[9] = {
        ivec2(-1, -1), ivec2(0, -1), ivec2(1, -1),
        ivec2(-1, 0), ivec2(0, 0), ivec2(1, 0),
        ivec2(-1, 1), ivec2(0, 1), ivec2(1, 1)
    };
    vec4 neighbors[9] = {
        c_load(input_depth, i + offsets[0]), c_load(input_depth, i + offsets[1]), c_load(input_depth, i + offsets[2]),
        c_load(input_depth, i + offsets[3]), c_load(input_depth, i + offsets[4]), c_load(input_depth, i + offsets[5]),
        c_load(input_depth, i + offsets[6]), c_load(input_depth, i + offsets[7]), c_load(input_depth, i + offsets[8])
    };

    if (abs(depth.r) > 1e-6) {
        imageStore(output_image, i, load);
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

        if (abs(prod) > 1e-6) {
            int min_idx = 0;
            float min_depth = 9999.0;
            for (int i = 0; i < 9; i++) {
                if (neighbors[i].r < min_depth) {
                    min_idx = i;
                    min_depth = min_depth;
                }
            }
            imageStore(output_image, i + offsets[min_idx], imageLoad(input_image, i + offsets[min_idx]));
            imageStore(output_depth, i + offsets[min_idx], imageLoad(input_depth, i + offsets[min_idx]));
        }
        else {
            imageStore(output_image, i, load);
            imageStore(output_depth, i, vec4(vec3(depth), 1.0));
        }
        // Mark background filling as incomplete
        atomicCounterMax(converged, uint(1));
    }

}
