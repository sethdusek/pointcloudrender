#version 330 core

in vec4 fcolor;
//in float depth;
out vec4 color_out;
out float depth_out;

void main() {
    depth_out = abs(gl_FragCoord.z);
    color_out = fcolor;
    //fragColor = vec4(vec3(abs(gl_FragCoord.z)), 1.0);
}
