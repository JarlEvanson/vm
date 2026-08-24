use std::{
    collections::HashMap,
    error, fmt,
    path::{Path, PathBuf},
};

pub fn acquire_triplets(
    kconfig: &HashMap<String, String>,
    source_dir: &Path,
) -> Result<(PathBuf, PathBuf), AcquireTripletsError> {
    let target_specifications = source_dir.join("target-specifications");

    let Some(revm_triplet) = kconfig.get("CONFIG_REVM_ARCH_TARGET_TRIPLET").cloned() else {
        return Err(AcquireTripletsError::MissingRevmTriplet);
    };

    let mut revm_triplet = target_specifications.join(revm_triplet);
    revm_triplet.set_extension("json");

    let Some(revm_stub_triplet) = kconfig.get("CONFIG_STUB_ARCH_TARGET_TRIPLET").cloned() else {
        return Err(AcquireTripletsError::MissingRevmStubTriplet);
    };

    let mut revm_stub_triplet = target_specifications.join(revm_stub_triplet);
    revm_stub_triplet.set_extension("json");

    Ok((revm_triplet, revm_stub_triplet))
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
pub enum AcquireTripletsError {
    MissingRevmTriplet,
    MissingRevmStubTriplet,
}

impl fmt::Display for AcquireTripletsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingRevmTriplet => {
                write!(f, "CONFIG_REVM_ARCH_TARGET_TRIPLET could not be found")
            }
            Self::MissingRevmStubTriplet => {
                write!(f, "CONFIG_STUB_ARCH_TARGET_TRIPLET could not be found")
            }
        }
    }
}

impl error::Error for AcquireTripletsError {}
