#version 330 core

in vec4 fcolor;
out vec4 fragColor;
void main() {
    fragColor = fcolor;
    //fragColor = vec4(vec3(abs(gl_FragCoord.z)), 1.0);
}
