fn main() {
    println!("cargo:rerun-if-changed=src/vertex.glsl");
    println!("cargo:rerun-if-changed=src/fragment.glsl");
    println!("cargo:rerun-if-changed=src/background_shader.glsl");
}
