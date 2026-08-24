use std::{borrow::Cow, collections::HashSet};

use crate::{
    config::{Config, Kind, Target, library_artifact_name},
    convert_name_to_rust_name, os_str_to_bytes,
};

pub fn generate(output: &mut Vec<u8>, config: &Config) {
    let mut builddir = Variable::new("builddir");
    builddir.push_escaped(os_str_to_bytes(config.arguments.build_dir.as_os_str()));
    builddir.write_out(output);

    let mut ninja_required_version = Variable::new("ninja_required_version");
    ninja_required_version.push_literal("1.3");
    ninja_required_version.write_out(output);
    output.push(b'\n');

    let mut file = NinjaFile::new();

    let mut rustc_path = config.rustc_sysroot.join("bin");
    rustc_path.push("rustc");

    let mut rustc = Variable::new("rustc");
    rustc.push_literal("\'");
    rustc.push_escaped(os_str_to_bytes(rustc_path.as_os_str()));
    rustc.push_literal("\'");
    file.add_variable(rustc);

    let mut clippy_path = config.rustc_sysroot.join("bin");
    clippy_path.push("clippy-driver");

    let mut clippy = Variable::new("clippy");
    clippy.push_literal("\'");
    clippy.push_escaped(os_str_to_bytes(clippy_path.as_os_str()));
    clippy.push_literal("\'");
    file.add_variable(clippy);

    let mut rustdoc_path = config.rustc_sysroot.join("bin");
    rustdoc_path.push("rustdoc");

    let mut rustdoc = Variable::new("rustdoc");
    rustdoc.push_literal("\'");
    rustdoc.push_escaped(os_str_to_bytes(rustdoc_path.as_os_str()));
    rustdoc.push_literal("\'");
    file.add_variable(rustdoc);

    let targets = [Target::Build, Target::Host, Target::Revm, Target::RevmStub];
    for target in targets {
        let mut rule = Rule::new(target.rustc_rule());

        let mut command = Variable::new("command");
        command.push_literal("$env $driver");
        command.push_literal(" --color always");

        add_arguments(config, &mut command, target);
        command.push_literal(" --crate-name $crate_name");
        command.push_literal(" --crate-type $crate_type");
        command.push_literal(" --edition 2024");
        command.push_literal(" -L '$search_dir'");
        command.push_literal(" $extern");
        command.push_literal(" $extra_args");
        command.push_literal(" --emit 'link=$out'");
        command.push_literal(" --emit 'dep-info=$out.dep'");
        command.push_literal(" --out-dir");
        command.push_literal(" \'");
        command.push_escaped(os_str_to_bytes(
            config.arguments.build_dir.join(target.folder()).as_os_str(),
        ));
        command.push_literal("\'");
        command.push_literal(" '$in'");
        rule.add_variable(command);

        let mut depfile = Variable::new("depfile");
        depfile.push_literal("$out.dep");
        rule.add_variable(depfile);

        let mut deps = Variable::new("deps");
        deps.push_literal("gcc");
        rule.add_variable(deps);

        file.add_rules(rule);
    }

    for target in targets {
        let mut doc = Rule::new(target.rustdoc_rule());
        let mut command = Variable::new("command");
        command.push_literal("$env $rustdoc");
        command.push_literal(" --color always");

        add_arguments(config, &mut command, target);
        command.push_literal(" --crate-name $crate_name");
        command.push_literal(" --crate-type $crate_type");
        command.push_literal(" --edition 2024");
        command.push_literal(" -L '$search_dir'");
        command.push_literal(" $extern");
        command.push_literal(" --out-dir '$out_dir'");
        command.push_literal(" $in");
        doc.add_variable(command);
        file.add_rules(doc);
    }

    let mut rustfmt_path = config.rustc_sysroot.join("bin");
    rustfmt_path.push("rustfmt");

    let mut format = Rule::new("format");
    let mut command = Variable::new("command");
    command.push_literal("\'");
    command.push_escaped(os_str_to_bytes(rustfmt_path.as_os_str()));
    command.push_literal("\'");
    command.push_literal(" --edition 2024");
    command.push_literal(" --color always");
    command.push_literal(" $in");
    format.add_variable(command);
    file.add_rules(format);

    let mut copy = Rule::new("copy");
    let mut command = Variable::new("command");
    command.push_literal("cp '$in' '$out'");
    copy.add_variable(command);
    file.add_rules(copy);

    let mut configure_path = config.arguments.build_dir.join("build");
    configure_path.push("rustc");
    configure_path.push("configure");

    let mut configure = Rule::new("configure");
    let mut command = Variable::new("command");
    command.push_literal("'");
    command.push_escaped(os_str_to_bytes(configure_path.as_os_str()));
    command.push_literal("'");
    for arg in std::env::args_os().skip(1) {
        command.push_literal(" '");
        command.push_escaped(os_str_to_bytes(arg.as_os_str()));
        command.push_literal("'");
    }
    configure.add_variable(command);
    file.add_rules(configure);

    let mut package = Rule::new("package");
    let mut command = Variable::new("command");
    command.push_literal("'$packager' $in '$out'");
    package.add_variable(command);
    file.add_rules(package);

    let tools_dir = config.arguments.out_dir.join("tools");

    let mut clippy_paths = Vec::new();
    let mut tool_paths = Vec::new();
    for driver in ["rustc", "clippy"] {
        for subproject_info in &config.subproject_info {
            if driver == "clippy" && subproject_info.name == "core" {
                continue;
            }

            let mut build = Build::new(subproject_info.target.rustc_rule());

            let mut artifacts_path = config
                .arguments
                .build_dir
                .join(subproject_info.target.folder());
            artifacts_path.push(driver);

            let artifact_path = artifacts_path.join(subproject_info.artifact_name());
            let output = FilePath::from_escaped(os_str_to_bytes(artifact_path.as_os_str()));
            if driver == "clippy" {
                clippy_paths.push(output.clone());
            } else if driver == "rustc"
                && matches!(subproject_info.kind, Kind::Tool)
                && matches!(subproject_info.target, Target::Host)
            {
                tool_paths.push((
                    FilePath::from_escaped(os_str_to_bytes(
                        tools_dir.join(&subproject_info.name).as_os_str(),
                    )),
                    output.clone(),
                ));
            }
            build.add_output(output);

            let explicit =
                FilePath::from_escaped(os_str_to_bytes(subproject_info.root_module.as_os_str()));
            build.add_explicit(explicit);

            let mut extern_var = Variable::new("extern");
            for dep in &subproject_info.libraries {
                let dep_artifact_path = if driver == "clippy" && dep == "core" {
                    let mut tmp = artifacts_path.clone();
                    tmp.pop();
                    tmp.push("rustc");
                    tmp.push(library_artifact_name(dep));
                    tmp
                } else {
                    artifacts_path.join(library_artifact_name(dep))
                };

                let dep_artifact_path = os_str_to_bytes(dep_artifact_path.as_os_str());

                extern_var.push_literal(" --extern '");
                extern_var.push_literal(convert_name_to_rust_name(dep));
                extern_var.push_literal("=");
                extern_var.push_escaped(&dep_artifact_path);
                extern_var.push_literal("'");

                build.add_implicit(FilePath::from_escaped(dep_artifact_path));
            }

            if driver == "clippy" {
                build.add_implicit(FilePath::from_literal("ALWAYS"));
            }

            let mut driver_var = Variable::new("driver");
            driver_var.push_literal("$");
            driver_var.push_literal(driver);
            build.add_variable(driver_var);

            let mut crate_name = Variable::new("crate_name");
            crate_name.push_escaped(convert_name_to_rust_name(&subproject_info.name));
            build.add_variable(crate_name);

            let mut crate_type = Variable::new("crate_type");
            crate_type.push_escaped(subproject_info.kind.crate_type());
            build.add_variable(crate_type);

            if !subproject_info.libraries.is_empty() {
                build.add_variable(extern_var);
            }

            let mut extra_args = Variable::new("extra_args");
            if let Some(linker_script) = subproject_info.linker_script.as_ref() {
                extra_args.push_literal("-C 'link-arg=-T");
                extra_args.push_escaped(os_str_to_bytes(linker_script.as_os_str()));
                extra_args.push_literal("'");
            }

            build.add_variable(extra_args);

            let mut search_dir = Variable::new("search_dir");
            search_dir.push_escaped(os_str_to_bytes(artifacts_path.as_os_str()));
            build.add_variable(search_dir);

            file.add_build(build);
        }
    }

    let mut clippy = Build::new("phony");
    clippy.add_output(FilePath::from_literal("clippy"));
    for clippy_path in clippy_paths {
        clippy.add_explicit(clippy_path);
    }
    file.add_build(clippy);

    let mut doc_paths = Vec::new();
    for subproject_info in &config.subproject_info {
        let mut build = Build::new(subproject_info.target.rustdoc_rule());

        let mut artifacts_path = config
            .arguments
            .build_dir
            .join(subproject_info.target.folder());
        artifacts_path.push("rustc");

        let mut doc_path = config
            .arguments
            .build_dir
            .join(subproject_info.target.folder());
        doc_path.push("doc");

        let mut artifact_path = doc_path.join(convert_name_to_rust_name(&subproject_info.name));
        artifact_path.push("index.html");
        doc_paths.push(artifact_path.clone());

        let output = FilePath::from_escaped(os_str_to_bytes(artifact_path.as_os_str()));
        build.add_output(output);

        let mut extern_var = Variable::new("extern");
        for dep in &subproject_info.libraries {
            let mut dep_doc_path = doc_path.join(convert_name_to_rust_name(dep));
            dep_doc_path.push("index.html");

            let dep_doc_path = os_str_to_bytes(dep_doc_path.as_os_str());
            build.add_implicit(FilePath::from_escaped(dep_doc_path));

            let dep_artifact_path = artifacts_path.join(library_artifact_name(dep));
            let dep_artifact_path = os_str_to_bytes(dep_artifact_path.as_os_str());

            extern_var.push_literal(" --extern '");
            extern_var.push_literal(convert_name_to_rust_name(dep));
            extern_var.push_literal("=");
            extern_var.push_escaped(&dep_artifact_path);
            extern_var.push_literal("'");

            build.add_implicit(FilePath::from_escaped(dep_artifact_path));
        }

        let explicit =
            FilePath::from_escaped(os_str_to_bytes(subproject_info.root_module.as_os_str()));
        build.add_explicit(explicit);

        let mut crate_name = Variable::new("crate_name");
        crate_name.push_escaped(convert_name_to_rust_name(&subproject_info.name));
        build.add_variable(crate_name);

        let mut crate_type = Variable::new("crate_type");
        crate_type.push_escaped(subproject_info.kind.crate_type());
        build.add_variable(crate_type);

        if !subproject_info.libraries.is_empty() {
            build.add_variable(extern_var);
        }

        let mut search_dir = Variable::new("search_dir");
        search_dir.push_escaped(os_str_to_bytes(artifacts_path.as_os_str()));
        build.add_variable(search_dir);

        let mut out_dir = Variable::new("out_dir");
        out_dir.push_escaped(os_str_to_bytes(doc_path.as_os_str()));
        build.add_variable(out_dir);

        file.add_build(build);
    }

    let mut doc = Build::new("phony");
    doc.add_output(FilePath::from_literal("doc"));
    for doc_path in doc_paths {
        doc.add_explicit(FilePath::from_escaped(os_str_to_bytes(
            doc_path.as_os_str(),
        )));
    }
    file.add_build(doc);

    let mut root_modules = HashSet::new();
    for subproject_info in &config.subproject_info {
        if !subproject_info.is_workspace_member {
            continue;
        }

        if root_modules.insert((&subproject_info.name, subproject_info.root_module.clone())) {
            let format_path = os_str_to_bytes(subproject_info.root_module.as_os_str());

            let mut format = Build::new("format");

            let mut format_output = FilePath::from_literal("format-");
            format_output.push_escaped(&subproject_info.name);
            format.add_output(format_output);

            format.add_explicit(FilePath::from_escaped(format_path));
            format.add_implicit(FilePath::from_literal("ALWAYS"));

            file.add_build(format);
        }
    }

    let mut format = Build::new("phony");
    format.add_output(FilePath::from_literal("format"));
    for (name, _) in root_modules {
        let mut format_output = FilePath::from_literal("format-");
        format_output.push_escaped(name);

        format.add_explicit(format_output);
    }
    file.add_build(format);

    for (tool_path, build_tool_path) in tool_paths.clone() {
        let mut copy = Build::new("copy");
        copy.add_output(tool_path);
        copy.add_explicit(build_tool_path);

        file.add_build(copy);
    }

    let mut tools = Build::new("phony");
    tools.add_output(FilePath::from_literal("tools"));
    for (tool_path, _) in tool_paths {
        tools.add_explicit(tool_path);
    }
    file.add_build(tools);

    let mut reconfigure = Build::new("configure");

    reconfigure.add_output(FilePath::from_escaped(os_str_to_bytes(
        config.arguments.ninja_path.as_os_str(),
    )));
    let rust_project_path = config.arguments.source_dir.join("rust-project.json");
    reconfigure.add_output(FilePath::from_escaped(os_str_to_bytes(
        rust_project_path.as_os_str(),
    )));

    reconfigure.add_implicit(FilePath::from_escaped(os_str_to_bytes(
        configure_path.as_os_str(),
    )));
    reconfigure.add_implicit(FilePath::from_escaped(os_str_to_bytes(
        config.arguments.config_path.as_os_str(),
    )));

    let mut generator_var = Variable::new("generator");
    generator_var.push_literal("1");
    reconfigure.add_variable(generator_var);

    file.add_build(reconfigure);

    let mut packager_path = config.arguments.build_dir.join(Target::Build.folder());
    packager_path.push("rustc");
    packager_path.push("package");

    let mut revm_path = config.arguments.build_dir.join(Target::Revm.folder());
    revm_path.push("rustc");
    revm_path.push("revm");

    let mut revm_stub_path = config.arguments.build_dir.join(Target::RevmStub.folder());
    revm_stub_path.push("rustc");
    revm_stub_path.push("revm-stub");

    let packaged_path = config.arguments.out_dir.join("revm");

    let mut package = Build::new("package");
    package.add_output(FilePath::from_escaped(os_str_to_bytes(
        packaged_path.as_os_str(),
    )));
    package.add_implicit(FilePath::from_escaped(os_str_to_bytes(
        packager_path.as_os_str(),
    )));
    package.add_explicit(FilePath::from_escaped(os_str_to_bytes(
        revm_stub_path.as_os_str(),
    )));
    package.add_explicit(FilePath::from_escaped(os_str_to_bytes(
        revm_path.as_os_str(),
    )));

    let mut packager = Variable::new("packager");
    packager.push_escaped(os_str_to_bytes(packager_path.as_os_str()));
    package.add_variable(packager);

    file.add_build(package);

    let mut package = Build::new("phony");
    package.add_output(FilePath::from_literal("package"));
    package.add_explicit(FilePath::from_escaped(os_str_to_bytes(
        packaged_path.as_os_str(),
    )));
    file.add_build(package);

    let mut always = Build::new("phony");
    always.add_output(FilePath::from_literal("ALWAYS"));
    file.add_build(always);

    file.add_default(FilePath::from_literal("tools"));
    file.add_default(FilePath::from_literal("package"));

    file.write_out(output);
}

fn add_arguments(config: &Config, command: &mut Variable, target: Target) {
    match target {
        Target::Build => {}
        Target::Host => {
            if let Some(triplet) = config.arguments.host_triplet.as_ref() {
                command.push_literal(" --target '");
                command.push_escaped(os_str_to_bytes(triplet));
                command.push_literal("'");
            }
        }
        Target::Revm | Target::RevmStub => {
            let triplet = if target == Target::Revm {
                &config.revm_triplet
            } else {
                &config.revm_stub_triplet
            };

            command.push_literal(" -Zunstable-options");
            command.push_literal(" --target '");
            command.push_escaped(os_str_to_bytes(triplet.as_os_str()));
            command.push_literal("'");

            command.push_escaped(" -C relocation-model=pie");
            command.push_escaped(" -C target-feature=+crt-static");

            command.push_escaped(" -Z direct-access-external-data=yes");
        }
    }

    let mut kconfig = config.kconfig.iter().collect::<Vec<_>>();
    kconfig.sort_unstable();

    for (name, value) in kconfig {
        command.push_literal(" --cfg");

        command.push_literal(" '");
        command.push_escaped(name);
        command.push_literal("'");

        command.push_literal(" --cfg");

        command.push_literal(" '");
        command.push_escaped(name);
        command.push_literal("=\"");
        command.push_escaped(value);
        command.push_literal("\"'");
    }

    let opts = match target {
        Target::Build | Target::Host => &config.general_opts,
        Target::Revm => &config.revm_opts,
        Target::RevmStub => &config.revm_stub_opts,
    };

    command.push_literal(" -C opt-level=");
    command.push_literal(opts.opt_level.option());

    if opts.incremental {
        let mut incremental_dir = config.arguments.build_dir.join(target.folder());
        incremental_dir.push("cache");

        command.push_literal(" -C incremental='");
        command.push_escaped(os_str_to_bytes(incremental_dir.as_os_str()));
        command.push_literal("'");
    }

    command.push_literal(" -C debug-assertions=");
    command.push_literal(format!("{}", opts.debug_assertions));

    command.push_literal(" -C debuginfo=");
    if opts.debug_info {
        command.push_literal("full");
    } else {
        command.push_literal("none");
    }

    command.push_literal(" -C lto=");
    command.push_literal(opts.lto.option());
}

#[derive(Clone, Debug)]
pub struct NinjaFile<'a> {
    variables: Vec<Variable<'a>>,
    rules: Vec<Rule<'a>>,
    build: Vec<Build<'a>>,
    default: Vec<FilePath>,
}

impl<'a> NinjaFile<'a> {
    fn new() -> Self {
        Self {
            variables: Vec::new(),
            rules: Vec::new(),
            build: Vec::new(),
            default: Vec::new(),
        }
    }

    fn add_variable(&mut self, variable: Variable<'a>) {
        self.variables.push(variable)
    }

    fn add_rules(&mut self, rule: Rule<'a>) {
        self.rules.push(rule)
    }

    fn add_build(&mut self, build: Build<'a>) {
        self.build.push(build)
    }

    fn add_default(&mut self, path: FilePath) {
        self.default.push(path)
    }

    fn write_out(&self, output: &mut Vec<u8>) {
        let mut needs_newline = false;

        for variable in &self.variables {
            variable.write_out(output);
            needs_newline = true;
        }

        if needs_newline && !self.rules.is_empty() {
            output.push(b'\n');
            needs_newline = false;
        }

        for (index, rule) in self.rules.iter().enumerate() {
            rule.write_out(output);
            if index != self.rules.len() - 1 {
                output.push(b'\n');
            }

            needs_newline = true;
        }

        if needs_newline && !self.build.is_empty() {
            output.push(b'\n');
            needs_newline = false;
        }

        for (index, build) in self.build.iter().enumerate() {
            build.write_out(output);
            if index != self.build.len() - 1 {
                output.push(b'\n');
            }

            needs_newline = true;
        }

        if needs_newline && !self.default.is_empty() {
            output.push(b'\n');
        }

        if !self.default.is_empty() {
            output.extend_from_slice("default".as_bytes());
            for default in &self.default {
                output.push(b' ');
                default.write_out(output);
            }

            output.push(b'\n');
        }
    }
}

#[derive(Clone, Debug)]
struct Variable<'a> {
    name: Cow<'a, str>,
    value: Vec<u8>,
}

impl<'a> Variable<'a> {
    pub fn new(name: impl Into<Cow<'a, str>>) -> Self {
        Self::new_inner(name.into())
    }

    fn new_inner(name: Cow<'a, str>) -> Self {
        Self {
            name,
            value: Vec::new(),
        }
    }

    fn push_literal(&mut self, bytes: impl AsRef<[u8]>) {
        self.value.extend_from_slice(bytes.as_ref())
    }

    fn push_escaped(&mut self, bytes: impl AsRef<[u8]>) {
        for &byte in bytes.as_ref() {
            match byte {
                b'\n' => {
                    self.value.push(b'$');
                    self.value.push(b'\n');
                }
                b'$' => {
                    self.value.push(b'$');
                    self.value.push(b'$');
                }
                _ => self.value.push(byte),
            }
        }
    }

    fn write_out(&self, output: &mut Vec<u8>) {
        output.extend_from_slice(self.name.as_bytes());
        output.extend_from_slice(" = ".as_bytes());
        output.extend_from_slice(&self.value);
        output.push(b'\n');
    }
}

#[derive(Clone, Debug)]
struct Rule<'a> {
    name: Cow<'a, str>,
    variables: Vec<Variable<'a>>,
}

impl<'a> Rule<'a> {
    pub fn new(name: impl Into<Cow<'a, str>>) -> Self {
        Self::new_inner(name.into())
    }

    fn new_inner(name: Cow<'a, str>) -> Self {
        Self {
            name,
            variables: Vec::new(),
        }
    }

    fn add_variable(&mut self, variable: Variable<'a>) {
        self.variables.push(variable)
    }

    fn write_out(&self, output: &mut Vec<u8>) {
        output.extend_from_slice("rule ".as_bytes());
        output.extend_from_slice(self.name.as_bytes());
        output.push(b'\n');

        for variable in &self.variables {
            output.extend_from_slice("  ".as_bytes());
            variable.write_out(output);
        }
    }
}

#[derive(Clone, Debug)]
struct Build<'a> {
    rule_name: Cow<'a, str>,

    outputs: Vec<FilePath>,

    explicit: Vec<FilePath>,
    implicit: Vec<FilePath>,

    variables: Vec<Variable<'a>>,
}

impl<'a> Build<'a> {
    pub fn new(rule_name: impl Into<Cow<'a, str>>) -> Self {
        Self::new_inner(rule_name.into())
    }

    fn new_inner(rule_name: Cow<'a, str>) -> Self {
        Self {
            rule_name,

            outputs: Vec::new(),

            explicit: Vec::new(),
            implicit: Vec::new(),

            variables: Vec::new(),
        }
    }

    fn add_output(&mut self, path: FilePath) {
        self.outputs.push(path);
    }

    fn add_explicit(&mut self, path: FilePath) {
        self.explicit.push(path)
    }

    fn add_implicit(&mut self, path: FilePath) {
        self.implicit.push(path)
    }

    fn add_variable(&mut self, variable: Variable<'a>) {
        self.variables.push(variable)
    }

    fn write_out(&self, output: &mut Vec<u8>) {
        assert!(!self.outputs.is_empty());

        output.extend_from_slice("build".as_bytes());

        for output_path in &self.outputs {
            output.push(b' ');
            output_path.write_out(output);
        }

        output.push(b':');
        output.push(b' ');

        output.extend_from_slice(self.rule_name.as_bytes());

        for explicit in &self.explicit {
            output.push(b' ');
            explicit.write_out(output);
        }

        if !self.implicit.is_empty() {
            output.push(b' ');
            output.push(b'|');

            for implicit in &self.implicit {
                output.push(b' ');
                implicit.write_out(output);
            }
        }

        output.push(b'\n');
        for variable in &self.variables {
            output.extend_from_slice("  ".as_bytes());
            variable.write_out(output);
        }
    }
}

#[derive(Clone, Debug)]
struct FilePath(Vec<u8>);

impl FilePath {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    fn from_literal(bytes: impl AsRef<[u8]>) -> Self {
        let mut path = Self::new();
        path.push_literal(bytes);

        path
    }

    pub fn from_escaped(bytes: impl AsRef<[u8]>) -> Self {
        let mut path = Self::new();
        path.push_escaped(bytes);

        path
    }

    fn push_literal(&mut self, bytes: impl AsRef<[u8]>) {
        self.0.extend_from_slice(bytes.as_ref());
    }

    fn push_escaped(&mut self, bytes: impl AsRef<[u8]>) {
        for &byte in bytes.as_ref() {
            match byte {
                b'\n' => {
                    self.0.push(b'$');
                    self.0.push(b'\n');
                }
                b' ' => {
                    self.0.push(b'$');
                    self.0.push(b' ');
                }
                b':' => {
                    self.0.push(b'$');
                    self.0.push(b':');
                }
                b'$' => {
                    self.0.push(b'$');
                    self.0.push(b'$');
                }
                _ => self.0.push(byte),
            }
        }
    }

    fn write_out(&self, output: &mut Vec<u8>) {
        output.extend_from_slice(&self.0);
    }
}
