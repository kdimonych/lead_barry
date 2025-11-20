//! This build script copies the `memory.x` file from the crate root into
//! a directory where the linker can always find it at build time.
//! For many projects this is optional, as the linker always searches the
//! project root directory -- wherever `Cargo.toml` is. However, if you
//! are using a workspace or have a more complicated build setup, this
//! build script becomes required. Additionally, by requesting that
//! Cargo re-run the build script whenever `memory.x` is changed,
//! updating `memory.x` ensures a rebuild of the application with the
//! new memory settings.

use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use flate2::Compression;
use flate2::write::GzEncoder;
use std::io;
use std::io::prelude::*;

fn forvard_dbg_var() {
    let forward_list = [
        "DBG_WIFI_SSID",
        "DBG_WIFI_PASSWORD",
        "DBG_WIFI_AP_SSID",
        "DBG_WIFI_AP_PASSWORD",
        "DBG_WIFI_AP_CHANNEL",
        "DBG_WIFI_AP_IP",
        "DBG_WIFI_AP_PREFIX_LEN",
        "DBG_USE_STATIC_IP_CONFIG",
        "DBG_STATIC_IP_ADDRESS",
        "DBG_STATIC_IP_GATEWAY",
        "DBG_STATIC_IP_PREFIX_LEN",
        "DBG_STATIC_IP_DNS_1",
        "DBG_STATIC_IP_DNS_2",
        "DBG_STATIC_IP_DNS_3",
    ];

    for var_name in forward_list {
        if let Ok(var_value) = env::var(var_name) {
            println!("cargo:rustc-env={}={}", var_name, var_value);
        }
    }
}

// Compress the files with gzip and include them in the binary
fn compress(files: &[&str]) {
    for &file in files {
        let output_file = format!("{}.gz", file);
        let input_data = std::fs::read(file).expect("Failed to read input file");

        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder
            .write_all(&input_data)
            .expect("Failed to write data to encoder");
        let compressed_data = encoder.finish().expect("Failed to finish compression");

        std::fs::write(&output_file, compressed_data).expect("Failed to write compressed file");
        println!("cargo:warning=Compressed {} to {}", file, output_file);
    }
}

fn main() {
    let files_to_compress = ["./src/config_server/web/main_configuration.html"];
    compress(&files_to_compress);

    // Put `memory.x` in our output directory and ensure it's
    // on the linker search path.
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());

    // By default, Cargo will re-run a build script whenever
    // any file in the project changes. By specifying `memory.x`
    // here, we ensure the build script is only re-run when
    // `memory.x` is changed.
    println!("cargo:rerun-if-changed=memory.x");

    // Load .env file if it exists
    if dotenvy::dotenv().is_ok() {
        println!("cargo:warning=Loaded .env file");
    } else {
        println!("cargo:warning=No .env file found, using defaults");
    }

    // Read environment variables and pass them to rustc

    if env::var("DBG_OVERWRITE_WITH_DEBUG_SETTINGS").is_ok() {
        println!("cargo:rustc-cfg=feature_overwrite_with_debug_settings");
    }
    println!("cargo:rustc-check-cfg=cfg(feature_overwrite_with_debug_settings)");

    forvard_dbg_var();

    // Rebuild if .env file changes
    println!("cargo:rerun-if-changed=.env");
}
