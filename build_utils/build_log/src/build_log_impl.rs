// ANSI color codes
pub const RED: &str = "\x1b[31m";
pub const YELLOW: &str = "\x1b[33m";
pub const GRAY: &str = "\x1b[90m";
pub const BLUE: &str = "\x1b[34m";
pub const GREEN: &str = "\x1b[32m";
pub const RESET: &str = "\x1b[0m";
pub const BOLD: &str = "\x1b[1m";

#[macro_export]
macro_rules! info {
        () => {{
            use build_log::build_log_impl::*;
            if std::env::var("DBG_PRINT").is_ok() {
            println!(
                "cargo:warning=[{}{}build.rs::INFO{}]", BLUE, BOLD, RESET
            );
            } else {
                println!("[{}{}build.rs::INFO{}]", BLUE, BOLD, RESET);
            }
        }};

        ($($arg:tt)*) => {{
            use build_log::build_log_impl::*;
            if std::env::var("DBG_PRINT").is_ok() {
            println!(
                "cargo:warning=[{}{}build.rs::INFO{}] {}", BLUE, BOLD, RESET, format!($($arg)*)
            );
            } else {
                println!("[{}{}build.rs::INFO{}] {}", BLUE, BOLD, RESET, format!($($arg)*));
            }
        }};
    }

#[macro_export]
macro_rules! debug {
        () => {{
            use build_log::build_log_impl::*;
            if std::env::var("DBG_PRINT").is_ok() {
            println!(
                "cargo:warning=[{}{}build.rs::DBG{}]", GRAY, BOLD, RESET
            );
            } else {
                println!("[{}{}build.rs::DBG{}]", GRAY, BOLD, RESET);
            }
        }};

        ($($arg:tt)*) => {{
            use build_log::build_log_impl::*;
            if std::env::var("DBG_PRINT").is_ok() {
            println!(
                "cargo:warning=[{}{}build.rs::DBG{}] {}",
                GRAY, BOLD, RESET, format!($($arg)*)
            );
            } else {
                println!("[{}{}build.rs::DBG{}] {}", GRAY, BOLD, RESET, format!($($arg)*));
            }
        }};
    }

#[macro_export]
macro_rules! warning {
        () => {{
            use build_log::build_log_impl::*;
            if std::env::var("DBG_PRINT").is_ok() {
            println!(
                "cargo:warning=[{}{}build.rs::WARNING{}]", YELLOW, BOLD, RESET
            );
            } else {
                println!("[{}{}build.rs::WARNING{}]", YELLOW, BOLD, RESET);
            }
        }};

        ($($arg:tt)*) => {{
            use build_log::build_log_impl::*;
            if std::env::var("DBG_PRINT").is_ok() {
            println!(
                "cargo:warning=[{}{}build.rs::WARNING{}] {}",
                YELLOW, BOLD, RESET, format!($($arg)*)
            );
            } else {
                println!("[{}{}build.rs::WARNING{}] {}", YELLOW, BOLD, RESET, format!($($arg)*));
            }
        }};
    }

#[macro_export]
macro_rules! error {
        () => {{
            use build_log::build_log_impl::*;
            if std::env::var("DBG_PRINT").is_ok() {
            println!(
                "cargo:warning=[{}{}build.rs::ERROR{}]", RED, BOLD, RESET
            );
            } else {
                println!("[{}{}build.rs::ERROR{}]", RED, BOLD, RESET);
            }
        }};

        ($($arg:tt)*) => {{
            use build_log::build_log_impl::*;
            if std::env::var("DBG_PRINT").is_ok() {
            println!(
                "cargo:warning=[{}{}build.rs::ERROR{}] {}",
                RED, BOLD, RESET, format!($($arg)*)
            );
            } else {
                println!("[{}{}build.rs::ERROR{}] {}", RED, BOLD, RESET, format!($($arg)*));
            }
        }};
    }

#[macro_export]
macro_rules! fatal {
        () => {{
            use build_log::build_log_impl::*;
            println!(
                "cargo:error=[{}{}build.rs::FATAL{}]", RED, BOLD, RESET
            );
        }};
        ($($arg:tt)*) => {{
            use build_log::build_log_impl::*;
            println!(
                "cargo:error=[{}{}build.rs::FATAL{}] {}",
                RED, BOLD, RESET, format!($($arg)*)
            );
        }};
    }
