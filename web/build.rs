use std::time::SystemTime;

fn main() {
    println!("cargo:rerun-if-changed=migrations");

    println!(
        "cargo::rustc-env=BUILD_TIMESTAMP={}",
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
    );

    println!(
        "cargo:rustc-env=PROFILE={}",
        std::env::var("PROFILE").unwrap()
    );
}
