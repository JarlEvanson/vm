use std::collections::HashMap;

pub fn locate(kconfig: &HashMap<String, String>, prefix: &str) -> CompilerOptions {
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

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct CompilerOptions {
    pub opt_level: OptimizationLevel,
    pub incremental: bool,
    pub debug_assertions: bool,
    pub debug_info: bool,
    pub lto: Lto,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum OptimizationLevel {
    Performance,
    Size,
    Debugging,
}

impl OptimizationLevel {
    pub const fn option(&self) -> &'static str {
        match self {
            Self::Performance => "3",
            Self::Size => "s",
            Self::Debugging => "0",
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Lto {
    Disabled,
    Thin,
    Fat,
}

impl Lto {
    pub const fn option(&self) -> &'static str {
        match self {
            Self::Disabled => "off",
            Self::Thin => "thin",
            Self::Fat => "fat",
        }
    }
}
