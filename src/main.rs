use ptip_ffi::{generate, initialize};

const EXAMPLE_TS_INPUT: &str = r#"
function greet() {
    console.log("Hello, world!");
}
"#;

fn main() {
    let registry = initialize();
    for lang in &registry {
        println!("Language found: {}", lang.name);
    }

    let _ = generate(EXAMPLE_TS_INPUT, "TypeScript", "Python", &registry);
}
