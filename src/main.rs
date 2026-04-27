use ptip_ffi::{generate, initialize};

const EXAMPLE_TS_INPUT: &str = r#"
function greet(a: string) {
    console.log("Hello, world!");
}
"#;

fn main() {
    let registry = initialize();
    for lang in &registry {
        println!("Language found: {}", lang.name);
    }

    // TypeScript is the only language currently supported, so we use it for both input and output.
    let _ = generate(EXAMPLE_TS_INPUT, "TypeScript", "TypeScript", &registry);
}
