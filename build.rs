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

fn main() {
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

    if env::var("USE_DEBUG_SETTINGS").is_ok() {
        println!("cargo:rustc-cfg=feature_use_debug_settings");
    }
    println!("cargo:rustc-check-cfg=cfg(feature_use_debug_settings)");

    if env::var("OVERWRITE_WITH_DEBUG_SETTINGS").is_ok() {
        println!("cargo:rustc-cfg=feature_overwrite_with_debug_settings");
    }
    println!("cargo:rustc-check-cfg=cfg(feature_overwrite_with_debug_settings)");

    if let Ok(wifi_ssid) = env::var("DBG_WIFI_SSID") {
        println!("cargo:rustc-env=DBG_WIFI_SSID={}", wifi_ssid);
    }
    if let Ok(wifi_password) = env::var("DBG_WIFI_PASSWORD") {
        println!("cargo:rustc-env=DBG_WIFI_PASSWORD={}", wifi_password);
    }

    if env::var("DBG_USE_STATIC_IP_CONFIG").is_ok() {
        println!("cargo:rustc-cfg=feature_use_static_ip_config");
    }
    println!("cargo:rustc-check-cfg=cfg(feature_use_static_ip_config)");

    if let Ok(static_ip_address) = env::var("DBG_STATIC_IP_ADDRESS") {
        println!(
            "cargo:rustc-env=DBG_STATIC_IP_ADDRESS={}",
            static_ip_address
        );
    }

    if let Ok(static_ip_gateway) = env::var("DBG_STATIC_IP_GATEWAY") {
        println!(
            "cargo:rustc-env=DBG_STATIC_IP_GATEWAY={}",
            static_ip_gateway
        );
    }

    if let Ok(static_ip_prefix_len) = env::var("DBG_STATIC_IP_PREFIX_LEN") {
        println!(
            "cargo:rustc-env=DBG_STATIC_IP_PREFIX_LEN={}",
            static_ip_prefix_len
        );
    }

    if let Ok(static_ip_dns) = env::var("DBG_STATIC_IP_DNS_1") {
        println!("cargo:rustc-env=DBG_STATIC_IP_DNS_1={}", static_ip_dns);
    }
    if let Ok(static_ip_dns) = env::var("DBG_STATIC_IP_DNS_2") {
        println!("cargo:rustc-env=DBG_STATIC_IP_DNS_2={}", static_ip_dns);
    }
    if let Ok(static_ip_dns) = env::var("DBG_STATIC_IP_DNS_3") {
        println!("cargo:rustc-env=DBG_STATIC_IP_DNS_3={}", static_ip_dns);
    }

    // Rebuild if .env file changes
    println!("cargo:rerun-if-changed=.env");
}
