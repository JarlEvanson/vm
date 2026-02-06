//! Various items that are common between [`Action`][a] parsing and execution.
//!
//! [a]: crate::cli::Action

/// The architectures supported by `tvm`.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum Arch {
    /// The `aarch64` architecture.
    Aarch64,
    /// The `x86_32` architecture.
    X86_32,
    /// The `x86_64` architecture.
    X86_64,
}

impl Arch {
    /// Returns the textual representation of the [`Arch`].
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Aarch64 => "aarch64",
            Self::X86_32 => "x86_32",
            Self::X86_64 => "x86_64",
        }
    }

    /// Returns the path to the target specification associated with [`Arch`].
    pub fn as_target_spec(&self) -> &'static str {
        match self {
            Self::Aarch64 => "targets/aarch64-unknown-none.json",
            Self::X86_32 => "targets/x86_32-unknown-none.json",
            Self::X86_64 => "targets/x86_64-unknown-none.json",
        }
    }

    /// Returns the name of the subfolder inside which the OVMF files for [`Arch`] are stored.
    pub fn as_ovmf_folder(&self) -> &'static str {
        match self {
            Self::Aarch64 => "aarch64",
            Self::X86_32 => "ia32",
            Self::X86_64 => "x64",
        }
    }

    /// Returns the name of the Limine binary associated with [`Arch`].
    pub fn as_limine_binary(&self) -> &'static str {
        match self {
            Self::Aarch64 => "BOOTAA64.EFI",
            Self::X86_32 => "BOOTIA32.EFI",
            Self::X86_64 => "BOOTX64.EFI",
        }
    }

    /// Returns the name of the `QEMU` executable associated with [`Arch`].
    pub fn as_qemu_executable(&self) -> &'static str {
        match self {
            Self::Aarch64 => "qemu-system-aarch64",
            Self::X86_32 => "qemu-system-i386",
            Self::X86_64 => "qemu-system-x86_64",
        }
    }
}

impl clap::ValueEnum for Arch {
    fn value_variants<'a>() -> &'a [Self] {
        static ARCHITECTURES: &[Arch] = &[Arch::Aarch64, Arch::X86_32, Arch::X86_64];

        ARCHITECTURES
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(self.as_str()))
    }
}

/// A `cargo` profile.
#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub enum Profile {
    /// The `dev` cargo profile.
    #[default]
    Dev,
    /// The `release` cargo profile.
    Release,
}

impl Profile {
    /// Returns the textual representation of the [`Profile`].
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Dev => "dev",
            Self::Release => "release",
        }
    }

    /// Returns the textual representation of the [`Profile`]'s target path component.
    pub fn target_string(&self) -> &'static str {
        match self {
            Self::Dev => "debug",
            Self::Release => "release",
        }
    }
}

impl clap::ValueEnum for Profile {
    fn value_variants<'a>() -> &'a [Self] {
        static PROFILES: &[Profile] = &[Profile::Dev, Profile::Release];

        PROFILES
    }

    fn to_possible_value(&self) -> Option<clap::builder::PossibleValue> {
        Some(clap::builder::PossibleValue::new(self.as_str()))
    }
}
