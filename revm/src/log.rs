//! Implementation and defintions related to logging for `revm`.

/// Various levels to determine the priority of information.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    /// Designates very low priority information.
    Trace,
    /// Designates lower priority information.
    Debug,
    /// Designates informatory logs.
    Info,
    /// Designates hazardous logs.
    Warn,
    /// Designates very serious logs.
    Error,
}
