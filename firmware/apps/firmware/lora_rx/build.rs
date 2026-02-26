//! This build script copies the `memory.x` file from the crate root into
//! a directory where the linker can always find it at build time.
//! For many projects this is optional, as the linker always searches the
//! project root directory -- wherever `Cargo.toml` is. However, if you
//! are using a workspace or have a more complicated build setup, this
//! build script becomes required. Additionally, by requesting that
//! Cargo re-run the build script whenever `memory.x` is changed,
//! updating `memory.x` ensures a rebuild of the application with the
//! new memory settings.

use cargo_command as cargo;
use file_operations::copy_memory_x;

fn main() {
    // Put `memory.x` in our output directory and ensure it's
    // on the linker search path.
    copy_memory_x().expect("Failed to copy memory.x");

    // By default, Cargo will re-run a build script whenever
    // any file in the project changes. By specifying `memory.x`
    // here, we ensure the build script is only re-run when
    // `memory.x` is changed.
    cargo::cmd!("rerun-if-changed=memory.x");

    cargo::cmd!("rustc-link-arg-bins=--nmagic");
    cargo::cmd!("rustc-link-arg-bins=-Tlink.x");
    cargo::cmd!("rustc-link-arg-bins=-Tlink-rp.x");
    cargo::cmd!("rustc-link-arg-bins=-Tdefmt.x");
}
