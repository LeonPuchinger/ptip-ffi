use std::path::Path;

use ptip_ffi::{generate, initialize};

fn main() {
    let registry = initialize();
    for lang in &registry {
        println!("Language found: {}", lang.name);
    }

    // TypeScript is the only language currently supported, so we use it for both input and output.
    if let Err(e) = generate(
        &registry,
        Path::new("./in"),
        Path::new("./in/index.ts"),
        "TypeScript",
        "TypeScript",
        Path::new("./out/callee"),
        Path::new("./out/caller"),
    ) {
        eprintln!("Error: {:?}", e);
    }
}
