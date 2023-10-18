#version 330 core
uniform mat4 projectionview;
in vec3 position;
in vec4 color;

out vec4 fcolor;
out float depth;

void main() {
    gl_Position = projectionview * vec4(position.x, position.y, position.z, 1.0);
    depth = gl_Position.z;
    fcolor = color;
}
