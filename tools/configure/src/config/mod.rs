use std::{
    borrow::Cow,
    cmp::Ordering,
    collections::HashMap,
    error,
    ffi::OsString,
    fmt, io, iter,
    path::{Path, PathBuf},
    process::Command,
    string::FromUtf8Error,
};

use crate::{
    bytes_to_os_str,
    config::{
        cli::{Arguments, ParseArgumentsAction, ParseArgumentsError},
        compiler_options::CompilerOptions,
        kconfig::ParseKconfigError,
        targets::AcquireTripletsError,
    },
    convert_name_to_rust_name,
};

pub mod cli;
mod compiler_options;
mod kconfig;
mod targets;

#[path = "../../../../lib/configure.rs"]
mod configure_lib;
#[path = "../../../../revm/configure.rs"]
mod configure_revm;
#[path = "../../../../stub/configure.rs"]
mod configure_stub;
#[path = "../../../../tools/configure.rs"]
mod configure_tools;

#[expect(clippy::result_large_err)]
pub fn load() -> Result<LoadConfigAction, LoadConfigError> {
    let arguments = match cli::parse()? {
        ParseArgumentsAction::Help => return Ok(LoadConfigAction::Help),
        ParseArgumentsAction::Run(arguments) => arguments,
    };

    let rustc_sysroot = acquire_sysroot(&arguments)?;

    let kconfig = kconfig::parse(&arguments.config_path)?;
    let (revm_triplet, revm_stub_triplet) =
        targets::acquire_triplets(&kconfig, &arguments.source_dir)?;

    let general_opts = compiler_options::locate(&kconfig, "GENERAL");
    let revm_opts = compiler_options::locate(&kconfig, "REVM");
    let revm_stub_opts = compiler_options::locate(&kconfig, "STUB");

    let mut config = Config {
        arguments,
        rustc_sysroot,

        kconfig,

        revm_triplet,
        revm_stub_triplet,

        general_opts,
        revm_opts,
        revm_stub_opts,

        build_cfgs: Vec::new(),
        host_cfgs: Vec::new(),
        revm_cfgs: Vec::new(),
        revm_stub_cfgs: Vec::new(),

        subprojects: Vec::new(),

        graph: HashMap::new(),
        subproject_info: Vec::new(),
    };

    config.build_cfgs = acquire_cfgs(&config, Target::Build)?;
    config.host_cfgs = acquire_cfgs(&config, Target::Host)?;
    config.revm_cfgs = acquire_cfgs(&config, Target::Revm)?;
    config.revm_stub_cfgs = acquire_cfgs(&config, Target::RevmStub)?;

    configure_lib::configure(&mut config);
    configure_revm::configure(&mut config);
    configure_stub::configure(&mut config);
    configure_tools::configure(&mut config);

    let mut subproject_info = Vec::new();
    for (index, subproject) in config.subprojects.iter().enumerate() {
        if subproject.build {
            let info = generate_subproject_info(index, Target::Build, iter::empty());

            subproject_info.push(info);
        }

        if subproject.host {
            let info = generate_subproject_info(index, Target::Host, iter::empty());

            subproject_info.push(info);
        }

        let core_iter = if subproject.name != "core" {
            Some("core")
        } else {
            None
        };

        let compiler_builtins_iter =
            if core_iter.is_some() && subproject.name != "compiler-builtins" {
                Some("compiler-builtins")
            } else {
                None
            };

        let extra_deps = core_iter.into_iter().chain(compiler_builtins_iter);

        if subproject.revm {
            let info = generate_subproject_info(index, Target::Revm, extra_deps.clone());

            subproject_info.push(info);
        }

        if subproject.revm_stub {
            let info = generate_subproject_info(index, Target::RevmStub, extra_deps);

            subproject_info.push(info);
        }
    }

    subproject_info.sort_unstable_by(|a, b| {
        if a.target.cmp(&b.target) != Ordering::Equal {
            return a.target.cmp(&b.target);
        }

        config.subprojects[a.index]
            .name
            .cmp(&config.subprojects[b.index].name)
    });

    let mut graph = HashMap::new();
    for (index, info) in subproject_info.iter_mut().enumerate() {
        graph.insert(
            (config.subprojects[info.index].name.clone(), info.target),
            index,
        );
    }

    config.graph = graph;
    config.subproject_info = subproject_info;
    Ok(LoadConfigAction::Run(Box::new(config)))
}

#[expect(clippy::result_large_err)]
fn acquire_sysroot(arguments: &Arguments) -> Result<PathBuf, LoadConfigError> {
    let mut command = Command::new(&arguments.rustc);
    command.args(["--print", "sysroot"]);

    let output = match command.output() {
        Ok(output) => output,
        Err(error) => {
            return Err(LoadConfigError::CommandExecutionFailed { command, error });
        }
    };

    if !output.status.success() {
        match output.status.code() {
            Some(code) => return Err(LoadConfigError::CommandFailedCode { command, code }),
            None => return Err(LoadConfigError::CommandFailedWithoutCode { command }),
        }
    }

    let rustc_sysroot = PathBuf::from(bytes_to_os_str(output.stdout.trim_ascii()));
    Ok(rustc_sysroot)
}

#[expect(clippy::result_large_err)]
fn acquire_cfgs(config: &Config, target: Target) -> Result<Vec<String>, LoadConfigError> {
    let mut command = Command::new(&config.arguments.rustc);
    for (name, value) in &config.kconfig {
        command.arg("--cfg");
        command.arg(name);

        command.arg("--cfg");
        command.arg(format!("{name}=\"{value}\""));
    }

    match target {
        Target::Build => {}
        Target::Host => {
            if let Some(triplet) = config.arguments.host_triplet.as_ref() {
                command.arg("--target");
                command.arg(triplet);
            }
        }
        Target::Revm | Target::RevmStub => {
            let triplet = if target == Target::Revm {
                &config.revm_triplet
            } else {
                &config.revm_stub_triplet
            };

            command.arg("-Zunstable-options");
            command.arg("--target");
            command.arg(triplet);
        }
    }

    let opts = match target {
        Target::Build | Target::Host => &config.general_opts,
        Target::Revm => &config.revm_opts,
        Target::RevmStub => &config.revm_stub_opts,
    };

    command.arg("-C");
    command.arg(format!("opt-level={}", opts.opt_level.option()));

    if opts.incremental {
        let mut incremental_dir = config.arguments.build_dir.join(target.folder());
        incremental_dir.push("cache");

        command.arg("-C");

        let mut incremental = OsString::from("incremental=");
        incremental.push(incremental_dir.as_os_str());
        command.arg(incremental);
    }

    command.arg("-C");
    command.arg(format!("debug-assertions={}", opts.debug_assertions));

    command.arg("-C");
    let debuginfo = if opts.debug_info { "full" } else { "none" };
    command.arg(format!("debuginfo={debuginfo}"));

    command.arg("-C");
    command.arg(format!("lto={}", opts.lto.option()));

    command.args(["--print", "cfg"]);

    let output = match command.output() {
        Ok(output) => output,
        Err(error) => return Err(LoadConfigError::CommandExecutionFailed { command, error }),
    };

    if !output.status.success() {
        match output.status.code() {
            Some(code) => return Err(LoadConfigError::CommandFailedCode { command, code }),
            None => return Err(LoadConfigError::CommandFailedWithoutCode { command }),
        }
    }

    let stdout = match String::from_utf8(output.stdout) {
        Ok(stdout) => stdout,
        Err(error) => return Err(LoadConfigError::Utf8Command { command, error }),
    };
    let cfgs = stdout.lines().map(String::from).collect::<Vec<_>>();
    Ok(cfgs)
}

fn generate_subproject_info<'a, I: Iterator<Item = &'a str>>(
    index: usize,
    target: Target,
    extra_deps: I,
) -> SubprojectInfo {
    SubprojectInfo {
        index,

        extra_libraries: extra_deps.map(String::from).collect::<Vec<_>>(),

        target,
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub arguments: Arguments,
    pub rustc_sysroot: PathBuf,

    pub kconfig: HashMap<String, String>,

    pub revm_triplet: PathBuf,
    pub revm_stub_triplet: PathBuf,

    pub general_opts: CompilerOptions,
    pub revm_opts: CompilerOptions,
    pub revm_stub_opts: CompilerOptions,

    pub build_cfgs: Vec<String>,
    pub host_cfgs: Vec<String>,
    pub revm_cfgs: Vec<String>,
    pub revm_stub_cfgs: Vec<String>,

    pub subprojects: Vec<Subproject>,

    pub graph: HashMap<(String, Target), usize>,
    pub subproject_info: Vec<SubprojectInfo>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubprojectInfo {
    pub index: usize,

    pub extra_libraries: Vec<String>,

    pub target: Target,
}

impl SubprojectInfo {
    pub fn artifact_name(&self, config: &Config) -> String {
        config.subprojects[self.index].artifact_name()
    }
}

pub fn library_artifact_name(name: &str) -> String {
    format!("lib{}.rlib", convert_name_to_rust_name(name))
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Kind {
    Binary,
    Tool,
    Library,
}

impl Kind {
    pub fn crate_type(&self) -> &'static str {
        match self {
            Self::Binary | Self::Tool => "bin",
            Self::Library => "lib",
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub enum Target {
    Build,
    Host,
    Revm,
    RevmStub,
}

impl Target {
    pub const fn rustc_rule(&self) -> &'static str {
        match self {
            Self::Build => "rustc_build",
            Self::Host => "rustc_host",
            Self::Revm => "rustc_revm",
            Self::RevmStub => "rustc_stub",
        }
    }

    pub const fn rustdoc_rule(&self) -> &'static str {
        match self {
            Self::Build => "doc_build",
            Self::Host => "doc_host",
            Self::Revm => "doc_revm",
            Self::RevmStub => "doc_stub",
        }
    }

    pub const fn folder(&self) -> &'static str {
        match self {
            Self::Build => "build",
            Self::Host => "host",
            Self::Revm => "revm",
            Self::RevmStub => "stub",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Subproject {
    pub name: String,

    pub root_module: PathBuf,
    pub libraries: Vec<String>,

    pub linker_script: Option<PathBuf>,

    pub kind: Kind,

    build: bool,
    host: bool,
    revm: bool,
    revm_stub: bool,

    pub is_workspace_member: bool,
}

impl Subproject {
    fn new<'a, 'b, S: Into<Cow<'a, str>>, P: Into<Cow<'b, Path>>>(name: S, root_module: P) -> Self {
        Self {
            name: name.into().into_owned(),

            root_module: root_module.into().into_owned(),
            libraries: Vec::new(),

            linker_script: None,

            kind: Kind::Library,

            build: true,
            host: true,
            revm: true,
            revm_stub: true,

            is_workspace_member: true,
        }
    }

    fn add_libraries(&mut self, library: impl Into<Cow<'static, str>>) {
        self.libraries.push(library.into().into_owned());
    }

    fn set_linker_script(&mut self, linker_script: PathBuf) {
        self.linker_script = Some(linker_script);
    }

    const fn set_binary(&mut self, tool: bool) {
        if tool {
            self.kind = Kind::Tool;
        } else {
            self.kind = Kind::Binary;
        }
    }

    const fn disable_build(&mut self) {
        self.build = false;
    }

    const fn disable_host(&mut self) {
        self.host = false;
    }

    const fn disable_revm(&mut self) {
        self.revm = false;
    }

    const fn disable_revm_stub(&mut self) {
        self.revm_stub = false;
    }

    const fn set_external(&mut self) {
        self.is_workspace_member = false;
    }

    pub fn artifact_name(&self) -> String {
        match self.kind {
            Kind::Binary | Kind::Tool => self.name.clone(),
            Kind::Library => library_artifact_name(&self.name),
        }
    }
}

#[derive(Clone, Debug)]
pub enum LoadConfigAction {
    Help,
    Run(Box<Config>),
}

#[derive(Debug)]
pub enum LoadConfigError {
    ParseArguments(ParseArgumentsError),
    CommandExecutionFailed {
        command: Command,
        error: io::Error,
    },
    CommandFailedCode {
        command: Command,
        code: i32,
    },
    CommandFailedWithoutCode {
        command: Command,
    },
    Utf8Command {
        command: Command,
        error: FromUtf8Error,
    },
    ParseKconfig(ParseKconfigError),
    AcquireTriplets(AcquireTripletsError),
}

impl From<ParseArgumentsError> for LoadConfigError {
    fn from(error: ParseArgumentsError) -> Self {
        Self::ParseArguments(error)
    }
}

impl From<ParseKconfigError> for LoadConfigError {
    fn from(error: ParseKconfigError) -> Self {
        Self::ParseKconfig(error)
    }
}

impl From<AcquireTripletsError> for LoadConfigError {
    fn from(error: AcquireTripletsError) -> Self {
        Self::AcquireTriplets(error)
    }
}

impl fmt::Display for LoadConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseArguments(error) => write!(f, "failed to parse CLI arguments: {error}"),
            Self::CommandExecutionFailed { command, error } => {
                write!(f, "failed to execute {command:?}: {error}")
            }
            Self::CommandFailedCode { command, code } => {
                write!(f, "{command:?} failed with status code: {code}")
            }
            Self::CommandFailedWithoutCode { command } => {
                write!(f, "{command:?} failed without a status code")
            }
            Self::Utf8Command { command, error } => {
                write!(f, "{command:?} returned a non UTF-8 output: {error}")
            }
            Self::ParseKconfig(error) => {
                write!(f, "failed to parse build configuration: {error}")
            }
            Self::AcquireTriplets(error) => {
                write!(f, "failed to acquire target triplets: {error}")
            }
        }
    }
}

impl error::Error for LoadConfigError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::ParseArguments(error) => Some(error),
            Self::CommandExecutionFailed { command: _, error } => Some(error),
            Self::Utf8Command { command: _, error } => Some(error),
            Self::ParseKconfig(error) => Some(error),
            Self::AcquireTriplets(error) => Some(error),
            _ => None,
        }
    }
}
