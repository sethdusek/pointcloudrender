#version 460
layout(local_size_x = 1, local_size_y = 1, local_size_z = 1) in;

layout(binding=3, rgba8) uniform readonly image2D input_image;
layout(binding=4, rgba8) writeonly uniform image2D output_image;
uniform int uWidth;
uniform int uHeight;
layout(binding=0) uniform atomic_uint converged;

void main() {
    ivec2 i = ivec2(gl_GlobalInvocationID.x, gl_GlobalInvocationID.y);
    vec4 load = imageLoad(input_image, i);// / 255.0;
    atomicCounterMax(converged, uint(load.g * 255));
    load.r = 1.0 - load.r;
    load.g = 1.0 - load.g;
    load.b = 1.0 - load.b;
    load.a = 1.0;
    imageStore(output_image, i, load);
}
