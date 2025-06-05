use std::env;

fn main() {
    let version = env::var("VERSION").unwrap_or("unknown".to_owned());
    println!("cargo:rustc-env=TO_VERSION={version}");
}
