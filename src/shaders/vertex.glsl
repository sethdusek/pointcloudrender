#version 460 core
uniform mat4 projectionview;
//uniform mat4 projectionview;
layout(location=0) in vec3 position;
layout(location=1) in vec4 color;

layout(location=0) out vec4 fcolor;
layout(location=1) out float depth;

void main() {
    gl_Position = projectionview * vec4(position.x, position.y, position.z, 1.0);
    depth = gl_Position.z;
    fcolor = color;
}
