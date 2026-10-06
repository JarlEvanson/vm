//! Subproject functionality.

use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

/// A complete description of a [`Subproject`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subproject {
    /// The name of the [`Subproject`].
    name: String,

    /// The configuration for the [`Library`] this [`Subproject`] provides.
    library: Option<Library>,

    /// Configurations for the [`Binary`]s this [`Subproject`] provides.
    binaries: Vec<Binary>,

    /// If true, this [`Subproject`] is a workspace member.
    ///
    /// This indicates that linting and compilation should be highly permissive and enables several
    /// LSP optimizations.
    is_workspace_member: bool,
}

impl Subproject {
    /// Creates a new [`Subproject`] that defines a [`Library`].
    pub(super) fn new_library<'a, S: Into<Cow<'a, str>>>(name: S, library: Library) -> Self {
        Self::new_inner(name.into().into_owned(), Some(library), Vec::new())
    }
    /// Creates a new [`Subproject`] that defines a [`Binary`].
    pub(super) fn new_binary<'a, S: Into<Cow<'a, str>>>(name: S, binary: Binary) -> Self {
        Self::new_inner(name.into().into_owned(), None, vec![binary])
    }

    /// Creates a new [`Subproject`] with a shared [`Library`] and multiple dependent [`Binary`]s.
    #[expect(dead_code)]
    pub(super) fn new_multi_binary<'a, S: Into<Cow<'a, str>>>(
        name: S,
        library: Library,
        binaries: Vec<Binary>,
    ) -> Self {
        Self::new_inner(name.into().into_owned(), Some(library), binaries)
    }

    /// Internal [`Subproject`] creation helper.
    fn new_inner(name: String, library: Option<Library>, binaries: Vec<Binary>) -> Self {
        Self {
            name,

            library,

            binaries,

            is_workspace_member: true,
        }
    }

    /// Calling this function indicates that the [`Subproject`] is not a workspace member.
    ///
    /// This indicates that linting and compilation should be highly permissive and enables several
    /// LSP optimizations.
    pub(super) fn set_external(&mut self) {
        self.is_workspace_member = false;
    }

    /// Returns the name of the [`Subproject`].
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the [`Subproject`]'s library, if it exists.
    pub fn library(&self) -> Option<&Library> {
        self.library.as_ref()
    }

    /// Returns the [`Subproject`]'s binaries.
    pub fn binaries(&self) -> &[Binary] {
        &self.binaries
    }

    /// Returns `true` if the [`Subproject`] is a member of the workspace.
    pub fn is_workspace_member(&self) -> bool {
        self.is_workspace_member
    }
}

/// The configuration for a [`Library`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Library {
    /// The [`Path`] to the root module of this [`Library`].
    root_module: PathBuf,

    /// The dependencies for this [`Library`] that are shared across all [`Target`]s.
    dependencies: Vec<String>,

    /// The [`LibraryTarget`] associated with [`Target::Build`].
    build: Option<LibraryTarget>,
    /// The [`LibraryTarget`] associated with [`Target::Host`].
    host: Option<LibraryTarget>,
    /// The [`LibraryTarget`] associated with [`Target::Revm`].
    revm: Option<LibraryTarget>,
    /// The [`LibraryTarget`] associated with [`Target::RevmStub`].
    revm_stub: Option<LibraryTarget>,
}

impl Library {
    /// Creates a new [`Library`].
    pub(super) fn new<'a, P: Into<Cow<'a, Path>>>(root_module: P) -> Self {
        Self {
            root_module: root_module.into().into_owned(),
            dependencies: Vec::new(),

            build: Some(LibraryTarget::new()),
            host: Some(LibraryTarget::new()),
            revm: Some(LibraryTarget::new()),
            revm_stub: Some(LibraryTarget::new()),
        }
    }

    /// Adds the provided `dependency` to the list of dependencies that are shared across all
    /// [`Target`]s.
    pub(super) fn add_dependency<'a, S: Into<Cow<'a, str>>>(&mut self, dependency: S) {
        self.dependencies.push(dependency.into().into_owned())
    }

    /// Disables the [`LibraryTarget`] for the provided [`Target`].
    pub(super) fn library_target_disable(&mut self, target: Target) {
        match target {
            Target::Build => self.build = None,
            Target::Host => self.host = None,
            Target::Revm => self.revm = None,
            Target::RevmStub => self.revm_stub = None,
        }
    }

    /// Returns a mutable reference to the [`Target`]-specific portion of the associated
    /// [`Library`] configuration.
    pub(super) fn library_target_mut(&mut self, target: Target) -> &mut LibraryTarget {
        match target {
            Target::Build => self
                .build
                .as_mut()
                .expect("subproject configurers must know what has been disabled"),
            Target::Host => self
                .host
                .as_mut()
                .expect("subproject configurers must know what has been disabled"),
            Target::Revm => self
                .revm
                .as_mut()
                .expect("subproject configurers must know what has been disabled"),
            Target::RevmStub => self
                .revm_stub
                .as_mut()
                .expect("subproject configurers must know what has been disabled"),
        }
    }

    /// Returns the [`Path`] to the root module of this [`Library`].
    pub fn root_module(&self) -> &Path {
        &self.root_module
    }

    /// Returns the dependencies for this [`Library`] that are shared across all [`Target`]s.
    pub fn dependencies(&self) -> DependencyIter<'_> {
        DependencyIter(&self.dependencies)
    }

    /// Returns an immutable reference to the [`Target`]-specific portion of the associated
    /// [`Library`] configuration.
    pub fn library_target(&self, target: Target) -> Option<&LibraryTarget> {
        match target {
            Target::Build => self.build.as_ref(),
            Target::Host => self.host.as_ref(),
            Target::Revm => self.revm.as_ref(),
            Target::RevmStub => self.revm_stub.as_ref(),
        }
    }
}

/// The [`Target`]-specific portion of the associated [`Library`] configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LibraryTarget {
    /// The dependencies for this [`Library`] that are specific to the associated [`Target`].
    dependencies: Vec<String>,
}

impl LibraryTarget {
    /// Creates a new [`LibraryTarget`].
    pub(crate) fn new() -> Self {
        Self {
            dependencies: Vec::new(),
        }
    }

    /// Adds the provided `dependency` to the list of dependencies that are specific to the
    /// associated [`Target`].
    pub(super) fn add_dependency<'a, S: Into<Cow<'a, str>>>(&mut self, dependency: S) {
        self.dependencies.push(dependency.into().into_owned())
    }

    /// Returns the dependencies for this [`Library`] that are specific to the associated
    /// [`Target`].
    pub fn dependencies(&self) -> DependencyIter<'_> {
        DependencyIter(&self.dependencies)
    }
}

/// A binary's associated metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binary {
    /// If [`Some`], this is the name of the produced executable.
    name: String,

    /// The path to the root module of this [`Binary`].
    root_module: PathBuf,

    /// The dependencies for the [`Binary`] that are shared across all [`Target`]s.
    dependencies: Vec<String>,

    /// The [`BinaryTarget`] associated with [`Target::Build`].
    build: Option<BinaryTarget>,
    /// The [`BinaryTarget`] associated with [`Target::Host`].
    host: Option<BinaryTarget>,
    /// The [`BinaryTarget`] associated with [`Target::Revm`].
    revm: Option<BinaryTarget>,
    /// The [`BinaryTarget`] associated with [`Target::RevmStub`].
    revm_stub: Option<BinaryTarget>,
}

impl Binary {
    /// Creates a new [`Binary`].
    pub(super) fn new<'a, 'b, S: Into<Cow<'a, str>>, P: Into<Cow<'b, Path>>>(
        name: S,
        root_module: P,
    ) -> Self {
        Self {
            name: name.into().into_owned(),
            root_module: root_module.into().into_owned(),
            dependencies: Vec::new(),

            build: Some(BinaryTarget::new()),
            host: Some(BinaryTarget::new()),
            revm: Some(BinaryTarget::new()),
            revm_stub: Some(BinaryTarget::new()),
        }
    }

    /// Adds the provided `dependency` to the list of dependencies that are shared across all
    /// [`Target`]s.
    pub(super) fn add_dependency<'a, S: Into<Cow<'a, str>>>(&mut self, dependency: S) {
        self.dependencies.push(dependency.into().into_owned())
    }

    /// Disables the [`BinaryTarget`] for the provided [`Target`].
    pub(super) fn binary_target_disable(&mut self, target: Target) {
        match target {
            Target::Build => self.build = None,
            Target::Host => self.host = None,
            Target::Revm => self.revm = None,
            Target::RevmStub => self.revm_stub = None,
        }
    }

    /// Returns a mutable reference to the [`Target`]-specific portion of the associated
    /// [`Binary`] configuration.
    pub(super) fn binary_target_mut(&mut self, target: Target) -> &mut BinaryTarget {
        match target {
            Target::Build => self
                .build
                .as_mut()
                .expect("subproject configurers must know what has been disabled"),
            Target::Host => self
                .host
                .as_mut()
                .expect("subproject configurers must know what has been disabled"),
            Target::Revm => self
                .revm
                .as_mut()
                .expect("subproject configurers must know what has been disabled"),
            Target::RevmStub => self
                .revm_stub
                .as_mut()
                .expect("subproject configurers must know what has been disabled"),
        }
    }

    /// Returns the name of the produced executable.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the [`Path`] to the root module of this [`Binary`].
    pub fn root_module(&self) -> &Path {
        &self.root_module
    }

    /// Returns the dependencies for this [`Binary`] that are shared across all [`Target`]s.
    pub fn dependencies(&self) -> DependencyIter<'_> {
        DependencyIter(&self.dependencies)
    }

    /// Returns an immutable reference to the [`Target`]-specific portion of the associated
    /// [`Binary`] configuration.
    pub fn binary_target(&self, target: Target) -> Option<&BinaryTarget> {
        match target {
            Target::Build => self.build.as_ref(),
            Target::Host => self.host.as_ref(),
            Target::Revm => self.revm.as_ref(),
            Target::RevmStub => self.revm_stub.as_ref(),
        }
    }
}

/// The target-specific portion of the associated [`Binary`] configuration.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BinaryTarget {
    /// The dependencies for this [`Binary`] that are specific to the associated [`Target`].
    dependencies: Vec<String>,

    /// The path to a linker script required for compilation on the associated [`Target`].
    linker_script: Option<PathBuf>,
}

impl BinaryTarget {
    /// Creates a new [`BinaryTarget`].
    pub fn new() -> Self {
        Self {
            dependencies: Vec::new(),
            linker_script: None,
        }
    }

    /// Adds the provided `dependency` to the list of dependencies that are specific to the
    /// associated [`Target`].
    pub(super) fn add_dependency<'a, S: Into<Cow<'a, str>>>(&mut self, dependency: S) {
        self.dependencies.push(dependency.into().into_owned())
    }

    /// Indicates the linker script that is required for compilation on the associated [`Target`].
    pub(super) fn set_linker_script<'a, P: Into<Cow<'a, Path>>>(&mut self, path: P) {
        self.linker_script = Some(path.into().into_owned());
    }

    /// Returns the dependencies for this [`Binary`] that are specific to the associated
    /// [`Target`].
    pub fn dependencies(&self) -> DependencyIter<'_> {
        DependencyIter(&self.dependencies)
    }

    /// Returns the [`Path`] to the linker script required by this [`Binary`] for compilation on the
    /// associated [`Target`].
    pub fn linker_script(&self) -> Option<&Path> {
        self.linker_script.as_deref()
    }
}

/// An [`Iterator`] over a list of dependencies.
#[derive(Clone, Debug)]
pub struct DependencyIter<'a>(&'a [String]);

impl<'a> Iterator for DependencyIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        let (next, remainder) = self.0.split_first()?;
        self.0 = remainder;
        Some(next.as_str())
    }
}

/// Assocations with a particular target.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    /// Indicates an association with the build target (i.e., the target `rustc` and the remainder
    /// of the build system is utilizing).
    Build,
    /// Indicates an association with the host target (i.e., where the `revm` connection/management
    /// tools will be run).
    Host,
    /// Indicates an association with the `revm` target (i.e., the target on which `revm` will run).
    Revm,
    /// Indicates an association with the `revm` target (i.e., the target on which `revm-stub` will
    /// run).
    ///
    /// This can differ from [`Target::Revm`] since `i686` and `x86_64` can transition between each
    /// other at runtime.
    RevmStub,
}

impl Target {
    /// A list of all [`Target`] values.
    pub const ALL_TARGETS: [Target; 4] =
        [Target::Build, Target::Host, Target::Revm, Target::RevmStub];

    /// Returns the display string for this [`Target`].
    pub fn display(&self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::Host => "host",
            Self::Revm => "revm",
            Self::RevmStub => "revm-stub",
        }
    }

    /// Returns the name of the folder in the build directory for this [`Target`].
    pub fn folder(&self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::Host => "host",
            Self::Revm => "revm",
            Self::RevmStub => "stub",
        }
    }

    /// Returns the name of the `rustc` rule for this [`Target`].
    pub fn rustc_rule(&self) -> &'static str {
        match self {
            Self::Build => "rustc_build",
            Self::Host => "rustc_host",
            Self::Revm => "rustc_revm",
            Self::RevmStub => "rustc_stub",
        }
    }

    /// Returns the name of the `rustdoc` rule for this [`Target`].
    pub fn rustdoc_rule(&self) -> &'static str {
        match self {
            Self::Build => "rustdoc_build",
            Self::Host => "rustdoc_host",
            Self::Revm => "rustdoc_revm",
            Self::RevmStub => "rustdoc_stub",
        }
    }
}
