use std::env;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    println!("cargo:rustc-env=OUT_DIR={}", out_dir);
    println!("cargo:rustc-env=RUSTFLAGS=--cfg procmacro2_semver_exempt");
}
