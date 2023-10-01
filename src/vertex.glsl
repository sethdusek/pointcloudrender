#version 330 core
uniform mat4 view;
uniform mat4 projection;
in vec3 position;
in vec4 color;

out vec4 fcolor;

void main() {
    gl_Position = projection * view * vec4(position.x, position.y, position.z, 1.0);
    fcolor = color;
}
