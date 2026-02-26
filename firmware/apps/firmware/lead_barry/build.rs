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
// use std::fs::File;
use std::io::Write;
// use std::path::PathBuf;

use flate2::Compression;
use flate2::write::GzEncoder;
// use std::io;
// use std::io::prelude::*;

use build_log as log;
use cargo_command as cargo;
use file_operations::copy_memory_x;

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

fn compress_bin_to_file(input_data: &[u8], output_file: &str) -> Result<(), ()> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(&input_data).map_err(|e| {
        log::error!("Failed to write data to encoder for {}. Error: {}", output_file, e);
        ()
    })?;

    let compressed_data = encoder.finish().map_err(|e| {
        log::error!("Failed to compress to {}. Error: {}", output_file, e);
        ()
    })?;

    std::fs::write(&output_file, compressed_data).map_err(|e| {
        log::error!("Failed to write compressed file {}. Error: {}", output_file, e);
        ()
    })?;

    // Force rebuild when HTML files change

    Ok(())
}

use minify_html::{Cfg, minify};
use regex::Regex;
// Compress the files with gzip and include them in the binary
fn compress_html(file: &str, output_file: &str) -> Result<(), ()> {
    let html_bin = std::fs::read(&file).expect("Failed to read input file");
    let html_str = std::str::from_utf8(&html_bin).map_err(|e| {
        log::error!("Failed to parse HTML file as UTF-8: {}", e);
        ()
    })?;

    // Remove single-line JS comments using regex
    let js_comment = Regex::new(r"\s*\/\/\s[\w\s\'\.,:\(\)\[\]]*\n").unwrap();
    let html_str = js_comment.replace_all(html_str, "\n");

    let cfg = Cfg {
        minify_css: true,
        minify_js: true,
        keep_comments: false,
        minify_doctype: true,
        ..Cfg::default()
    };

    let minified_html = minify(html_str.as_bytes(), &cfg);
    compress_bin_to_file(&minified_html, &output_file)?;
    Ok(())
}

fn compress_file(file: &str, output_file: &str) -> Result<(), ()> {
    let input_data = std::fs::read(file).expect("Failed to read input file");

    compress_bin_to_file(&input_data, &output_file)?;
    Ok(())
}

// Compress the files with gzip and include them in the binary
fn compress(files: &[&str]) -> Result<(), ()> {
    for &file in files {
        let output_file = format!("{}.gz", file);
        if file.ends_with(".html") {
            log::info!("Compressed HTML");
            compress_html(&file, &output_file)?;
        } else {
            compress_file(&file, &output_file)?;
        }

        log::info!("Compressed {} to {}", file, output_file);
        // Force rebuild when HTML files change
        cargo::cmd!("rerun-if-changed={}", file);
        cargo::cmd!("rerun-if-changed={}", output_file);
        cargo::cmd!("rerun-if-not-exists={}", output_file);
    }
    Ok(())
}

fn main() {
    let files_to_compress = ["./src/web_server/web/main_configuration.html"];
    compress(&files_to_compress).expect("Failed to compress files");

    // Load .env file if it exists
    if dotenvy::dotenv().is_ok() {
        log::info!("Loaded .env file");
    } else {
        log::warning!("No .env file found, using defaults");
    }

    // Read environment variables and pass them to rustc
    if env::var("DBG_OVERWRITE_WITH_DEBUG_SETTINGS").is_ok() {
        cargo::cmd!("rustc-cfg=feature_overwrite_with_debug_settings");
    }
    cargo::cmd!("rustc-check-cfg=cfg(feature_overwrite_with_debug_settings)");

    forvard_dbg_var();

    // Rebuild if .env file changes
    cargo::cmd!("rerun-if-changed=.env");

    /**************************************************************************************
     *  Linker configuration
     **************************************************************************************/

    // Put `memory.x` in our output directory and ensure it's
    // on the linker search path.
    copy_memory_x().expect("Failed to copy memory.x");

    // Specify link flags for the linker script and other settings.
    cargo::cmd!("rustc-link-arg-bins=--nmagic");
    cargo::cmd!("rustc-link-arg-bins=-Tlink.x");
    cargo::cmd!("rustc-link-arg-bins=-Tlink-rp.x");
    if env::var("CARGO_FEATURE_DEFMT").is_ok() {
        cargo::cmd!("rustc-link-arg-bins=-Tdefmt.x");
        log::info!("defmt feature is enabled");
    }
}
