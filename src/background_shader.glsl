#version 430
layout(local_size_x = 1, local_size_y = 1, local_size_z = 1) in;

//uniform layout(binding=3, rgba8) readonly image2D input_image;
uniform layout(binding=0, rgba8) writeonly image2D output_image;
uniform int uWidth;
uniform int uHeight;
//uniform layout(binding=1) atomic_uint converged;

void main() {
    ivec2 i = ivec2(gl_GlobalInvocationID.x, gl_GlobalInvocationID.y);
    // vec4 load = imageLoad(input_image, i);// / 255.0;
    vec4 load = vec4(0.0);
    load.r = 1.0; //- load.r;
    load.g = 0.0;// - load.g;
    load.b = 0.0;// - load.b;
    load.a = 1.0;
    imageStore(output_image, i, load);
    if (i.x == 0 && i.y == 0) {
        //atomicCounterIncrement(converged);
    }
}
