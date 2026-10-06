//! Functionality to locate [`CompilerOptions`] for a a specified set of crates.

use std::collections::HashMap;

/// Locates the [`CompilerOptions`] with the provided `prefix`.
pub(super) fn locate(kconfig: &HashMap<String, String>, prefix: &str) -> CompilerOptions {
    let opt_level_options = [
        (OptimizationLevel::Performance, "PERFORMANCE"),
        (OptimizationLevel::Size, "SIZE"),
        (OptimizationLevel::Debugging, "DEBUGGING"),
    ];

    let opt_level = lookup_list(
        prefix,
        "OPTIMIZE",
        kconfig,
        opt_level_options.into_iter().map(|(_, suffix)| suffix),
    );
    let opt_level = match opt_level {
        Some(index) => opt_level_options[index].0,
        None => {
            eprintln!("warning: missing or invalid configuration for '{prefix}_OPTIMIZE_*'");
            OptimizationLevel::Debugging
        }
    };

    let incremental = kconfig
        .get(&format!("CONFIG_{prefix}_INCREMENTAL"))
        .is_some_and(|value| value == "y");
    let debug_assertions = kconfig
        .get(&format!("CONFIG_{prefix}_DEBUG_ASSERTIONS"))
        .is_some_and(|value| value == "y");
    let debug_info = kconfig
        .get(&format!("CONFIG_{prefix}_DEBUG_INFO"))
        .is_some_and(|value| value == "y");

    let lto_options = [
        (Lto::Disabled, "DISABLED"),
        (Lto::Thin, "THIN"),
        (Lto::Fat, "FAT"),
    ];

    let lto = lookup_list(
        prefix,
        "LTO",
        kconfig,
        lto_options.into_iter().map(|(_, suffix)| suffix),
    );
    let lto = match lto {
        Some(index) => lto_options[index].0,
        None => {
            eprintln!("warning: missing or invalid configuration for '{prefix}_LTO_*'");
            Lto::Disabled
        }
    };

    CompilerOptions {
        opt_level,
        incremental,
        debug_assertions,
        debug_info,
        lto,
    }
}

/// Looks for the index of the requested item.
fn lookup_list<'a, I: Iterator<Item = &'a str>>(
    prefix: &str,
    base: &str,
    kconfig: &HashMap<String, String>,
    mut list: I,
) -> Option<usize> {
    list.position(|suffix| {
        kconfig
            .get(&format!("CONFIG_{prefix}_{base}_{suffix}"))
            .is_some()
    })
}

/// The values of the CLI options.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct CompilerOptions {
    /// The optimization level.
    pub opt_level: OptimizationLevel,
    /// Whether incremental compilation should be performed.
    pub incremental: bool,
    /// Whether debug assertions should be enabled.
    pub debug_assertions: bool,
    /// Whether debug info should be generated.
    pub debug_info: bool,
    /// Whether LTO should be performed and what type.
    pub lto: Lto,
}

/// The value of the CLI option for `opt-level`.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum OptimizationLevel {
    /// Optimize for performance.
    Performance,
    /// Optimize for size.
    Size,
    /// Optimize for debugging.
    Debugging,
}

impl OptimizationLevel {
    /// Returns the [`str`] of the value of the CLI option for `-C opt-level=<VALUE>`.
    pub const fn option(&self) -> &'static str {
        match self {
            Self::Performance => "3",
            Self::Size => "s",
            Self::Debugging => "0",
        }
    }
}

/// The value of the CLI option for LTO.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Lto {
    /// LTO should not be performed.
    Disabled,
    /// Thin LTO should be performed.
    Thin,
    /// Fat LTO should be performed.
    Fat,
}

impl Lto {
    /// Returns the [`str`] of value of the CLI option for `-C lto=<VALUE>`.
    pub const fn option(&self) -> &'static str {
        match self {
            Self::Disabled => "off",
            Self::Thin => "thin",
            Self::Fat => "fat",
        }
    }
}
