use std::process::Command;
use std::process::Stdio;
fn main() {
    println!("cargo:rerun-if-changed=src/vertex.glsl");
    println!("cargo:rerun-if-changed=src/fragment.glsl");
    println!("cargo:rerun-if-changed=src/background_shader.glsl");
    for shader in std::fs::read_dir("src/shaders")
        .unwrap()
        .map(Result::unwrap)
    {
        if shader
            .file_type()
            .as_ref()
            .map(std::fs::FileType::is_file)
            .unwrap_or(false)
        {
            let path = shader.path();
            let file_name = path.to_str().unwrap();
            println!("cargo:rerun-if-changed={}", file_name);
            if file_name.split_once('.').unwrap_or(("", "")).1 == "wgsl" {
                let naga_result = Command::new("naga")
                    .arg(file_name)
                    .stdout(Stdio::inherit())
                    .stderr(Stdio::inherit())
                    .status();
                if let Ok(res) = naga_result {
                    if !res.success() {
                        std::process::exit(1);
                    }
                }
                else {
                    println!("cargo:warning=Naga could not be called for shader validation");
                }
            }
        }
    }
}
