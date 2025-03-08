use std::env;
use std::fs;
use std::path::Path;

fn main() {
    // Get the output directory from cargo
    let out_dir = env::var("OUT_DIR").unwrap();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    // Source and destination paths for the Metal shader
    let src_path = Path::new(&manifest_dir).join("aiSimulator/metal_location_search.metal");
    let dest_path = Path::new(&out_dir).join("../../../metal_location_search.metal");

    // Copy the Metal shader file
    println!("cargo:rerun-if-changed=src/metal_location_search.metal");
    if let Err(e) = fs::copy(&src_path, &dest_path) {
        println!("cargo:warning=Failed to copy Metal shader: {}", e);
    } else {
        println!("cargo:warning=Successfully copied Metal shader to: {}", dest_path.display());
    }
} 