use copy_to_output::copy_to_output;
use std::{fmt::format, process::Command, time::SystemTime};

fn main() {
    println!("cargo:rerun-if-changed=migrations");

    let profile = std::env::var("PROFILE").unwrap();
    if profile == "release" {
        println!(
            "cargo::rustc-env=BUILD_TIMESTAMP={}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs()
        );
    } else {
        println!("cargo::rustc-env=BUILD_TIMESTAMP=0");
    }

    println!("cargo:rustc-env=PROFILE={}", profile);

    println!("cargo:rerun-if-changed=../TemporalStasis/TemporalStasis.Connector");
    let connector_result_path = format!("{}/connector", std::env::var("OUT_DIR").unwrap());
    let connector_result = Command::new("dotnet")
        .arg("publish")
        .arg("--nologo")
        .arg("../TemporalStasis/TemporalStasis.Connector")
        .arg("--ucr")
        .args(["-o", &connector_result_path])
        .status()
        .unwrap();
    assert!(connector_result.success());

    copy_to_output(&connector_result_path, &std::env::var("PROFILE").unwrap())
        .expect("Could not connector artifact");
}
