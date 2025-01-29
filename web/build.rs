use build_target::{Arch, Env, Os};
use copy_to_output::copy_to_output_path;
use std::{ffi::OsStr, process::Command, time::SystemTime};

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

    println!("cargo:rustc-env=PROFILE={profile}");

    // Skip building the connector if we're running in rust-analyzer or anywhere unnecessary
    let is_redundant = cfg!(clippy) || cfg!(miri) || cfg!(doc) || cfg!(test) || cfg!(rustfmt);
    let is_rust_analyzer = if cfg!(windows) {
        std::env::var("_NT_SYMBOL_PATH").is_ok_and(|v| v.contains("rust-analyzer"))
    } else if cfg!(unix) {
        std::env::current_exe().is_ok_and(|v| {
            v.ancestors()
                .into_iter()
                .any(|p| p == OsStr::new("rust-analyzer"))
        })
    } else {
        false
    };

    if !is_redundant && !is_rust_analyzer {
        println!("cargo:rerun-if-changed=TemporalStasis");
        build_connector();
    }
}

fn build_connector() {
    let connector_result_path = format!("{}/connector", std::env::var("OUT_DIR").unwrap());

    let target = build_target::target().unwrap();
    let mut rid_arch = match target.arch {
        Arch::AARCH64 => "arm64",
        Arch::ARM => "arm",
        Arch::MIPS64 => "mips64",
        Arch::RISCV => "riscv64",
        Arch::WASM32 => "wasm",
        Arch::X86 => "x86",
        Arch::X86_64 => "x64",
        Arch::S390X => "s390x",
        Arch::POWERPC64 => "ppc64le",
        _ => panic!("Unsupported architecture: {:?}", target.arch),
    }
    .to_string();
    if target.env == Env::Musl {
        rid_arch = format!("musl-{rid_arch}");
    }

    let rid_os = match target.os {
        Os::Android => "android",
        Os::Emscripten => "browser",
        Os::Linux => "linux",
        Os::MacOs => "osx",
        Os::FreeBSD => "freebsd",
        Os::Solaris => "solaris",
        Os::Windows => "win",
        _ if target.family == build_target::Family::Unix => "unix",
        _ => panic!("Unsupported os: {:?}", target.arch),
    };

    let rid = format!("{rid_os}-{rid_arch}");

    let connector_result = Command::new("dotnet")
        .arg("publish")
        .arg("--nologo")
        .arg("TemporalStasis/TemporalStasis.Connector")
        .args(["-r", &rid])
        .args(["-o", &connector_result_path])
        .status()
        .unwrap();
    assert!(connector_result.success());

    for entry in std::fs::read_dir(&connector_result_path).unwrap() {
        let path = entry.unwrap().path();
        let path = path.as_path();
        if path.is_file() {
            copy_to_output_path(path, &std::env::var("PROFILE").unwrap())
                .expect("Could not copy connector artifact");
        }
    }
}
