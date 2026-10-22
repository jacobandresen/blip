use std::{env, fs, path::PathBuf};

#[path = "src/gfx.rs"]
mod gfx;

fn main() {
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("OUT_DIR"));
    let mut registry = String::new();
    for (path, bytes) in gfx::generate() {
        if path.contains("_LOW_") {
            continue;
        }
        let name = path.rsplit('/').next().expect("sprite name");
        let dest = out.join(name);
        fs::write(&dest, bytes).expect("write Bubbler sprite");
        let constant = name.trim_end_matches(".png");
        registry.push_str(&format!(
            "pub const {constant}: &[u8] = include_bytes!({:?});\n",
            dest
        ));
    }
    fs::write(out.join("gfx.rs"), registry).expect("write sprite registry");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/gfx.rs");
}
