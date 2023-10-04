#version 330 core
uniform mat4 projectionview;
in vec3 position;
in vec4 color;

out vec4 fcolor;

void main() {
    gl_Position = projectionview * vec4(position.x, position.y, position.z, 1.0);
    gl_PointSize = 10; // gl_Position.w;
    fcolor = color;
}
