use crate::{
    config::{Config, Kind, Target, library_artifact_name},
    convert_name_to_rust_name,
    ninja::utils::{Build, FilePath, NinjaFile, Rule, Variable},
    os_str_to_bytes,
};

mod utils;

pub fn generate(output: &mut Vec<u8>, config: &Config) {
    // Add the ninja build system variables separately.
    let mut builddir = Variable::new("builddir");
    builddir.push_escaped_path(&config.arguments.build_dir);
    builddir.write_out(output);

    let mut ninja_required_version = Variable::new("ninja_required_version");
    ninja_required_version.push_literal("1.3");
    ninja_required_version.write_out(output);
    output.push(b'\n');

    let mut file = NinjaFile::new();

    add_rust_binaries(&mut file, config);
    add_rust_rules(&mut file, config);
    add_misc_rules(&mut file, config);

    add_format_builds(&mut file, config);
    add_rustc_builds(&mut file, config);
    add_unit_test_builds(&mut file, config);
    add_miri_unit_test_builds(&mut file, config);
    add_reconfigure_build(&mut file, config);

    let mut always = Build::new("phony");
    always.add_output(FilePath::from_literal("ALWAYS"));
    file.add_build(always);

    file.write_out(output);
}

fn add_rust_binaries(file: &mut NinjaFile, config: &Config) {
    let rustc_sysroot_bin_path = config.rustc_sysroot.join("bin");

    let rustc_path = rustc_sysroot_bin_path.join("rustc");
    let clippy_path = rustc_sysroot_bin_path.join("clippy-driver");
    let miri_path = rustc_sysroot_bin_path.join("miri");
    let rustfmt_path = rustc_sysroot_bin_path.join("rustfmt");
    let rustdoc_path = rustc_sysroot_bin_path.join("rustdoc");

    let mut rustc = Variable::new("rustc");
    rustc.push_command_escaped_path(&rustc_path);
    file.add_variable(rustc);

    let mut clippy = Variable::new("clippy");
    clippy.push_command_escaped_path(&clippy_path);
    file.add_variable(clippy);

    let mut miri = Variable::new("miri");
    miri.push_command_escaped_path(&miri_path);
    file.add_variable(miri);

    let mut rustdoc = Variable::new("rustdoc");
    rustdoc.push_command_escaped_path(&rustdoc_path);
    file.add_variable(rustdoc);

    let mut rustfmt = Variable::new("rustfmt");
    rustfmt.push_command_escaped_path(&rustfmt_path);
    file.add_variable(rustfmt);
}

fn add_rust_rules(file: &mut NinjaFile, config: &Config) {
    let targets = [Target::Build, Target::Host, Target::Revm, Target::RevmStub];
    for target in targets {
        add_rustc_rule(file, config, target);
    }

    for target in targets {
        add_rustdoc_rule(file, config, target);
    }

    add_rustfmt_rule(file, config);
}

fn add_rustc_rule(file: &mut NinjaFile, config: &Config, target: Target) {
    let mut rule = Rule::new(target.rustc_rule());

    let mut command = Variable::new("command");
    command.push_literal("$env $driver");
    command.push_literal(" --color always");

    add_arguments(config, &mut command, target);
    command.push_literal(" --crate-name $crate_name");
    command.push_literal(" --crate-type $crate_type");
    command.push_literal(" --edition 2024");
    command.push_literal(" -L $search_dir");
    command.push_literal(" $extern");
    command.push_literal(" $extra_args");
    command.push_literal(" --emit link=$out_file");
    command.push_literal(" --emit dep-info=$out_file.dep");

    command.push_literal(" --out-dir");
    command.push_literal(" ");
    command.push_command_escaped_path(config.arguments.build_dir.join(target.folder()));

    command.push_literal(" $in_file");
    rule.add_variable(command);

    let mut depfile = Variable::new("depfile");
    depfile.push_literal("$out.dep");
    rule.add_variable(depfile);

    let mut deps = Variable::new("deps");
    deps.push_literal("gcc");
    rule.add_variable(deps);

    file.add_rule(rule);
}

fn add_rustdoc_rule(file: &mut NinjaFile, config: &Config, target: Target) {
    let mut rule = Rule::new(target.rustdoc_rule());

    let mut command = Variable::new("command");
    command.push_literal("$env $rustdoc");
    command.push_literal(" --color always");

    add_arguments(config, &mut command, target);
    command.push_literal(" --crate-name $crate_name");
    command.push_literal(" --crate-type $crate_type");
    command.push_literal(" --edition 2024");
    command.push_literal(" -L $search_dir");
    command.push_literal(" $extern");
    command.push_literal(" $extra_args");
    command.push_literal(" --emit html-static-files");
    command.push_literal(" --emit html-non-static-files");
    command.push_literal(" --emit dep-info=$out_file.dep");

    command.push_literal(" --out-dir");
    command.push_literal(" ");
    command.push_command_escaped_path(config.arguments.build_dir.join(target.folder()).join("doc"));

    command.push_literal(" $in_file");
    rule.add_variable(command);

    let mut depfile = Variable::new("depfile");
    depfile.push_literal("$out.dep");
    rule.add_variable(depfile);

    let mut deps = Variable::new("deps");
    deps.push_literal("gcc");
    rule.add_variable(deps);

    file.add_rule(rule);
}

fn add_arguments(config: &Config, command: &mut Variable, target: Target) {
    match target {
        Target::Build => {}
        Target::Host => {
            if let Some(triplet) = config.arguments.host_triplet.as_ref() {
                command.push_literal(" --target '");
                command.push_command_escaped(os_str_to_bytes(triplet));
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
            command.push_literal(" --target ");
            command.push_command_escaped(os_str_to_bytes(triplet.as_os_str()));

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

fn add_rustfmt_rule(file: &mut NinjaFile, _: &Config) {
    let mut format = Rule::new("format");
    let mut command = Variable::new("command");
    command.push_literal("$rustfmt");
    command.push_literal(" --edition 2024");
    command.push_literal(" --color always");
    command.push_literal(" $in_file");
    format.add_variable(command);
    file.add_rule(format);
}

fn add_misc_rules(file: &mut NinjaFile, config: &Config) {
    let mut copy = Rule::new("copy");
    let mut command = Variable::new("command");
    command.push_literal("cp $in_file $out_file");
    copy.add_variable(command);
    file.add_rule(copy);

    let mut execute = Rule::new("execute");
    let mut command = Variable::new("command");
    command.push_literal("$in $args");
    execute.add_variable(command);
    file.add_rule(execute);

    let mut configure_path = config.arguments.build_dir.join("build");
    configure_path.push("rustc");
    configure_path.push("configure");

    let mut configure = Rule::new("configure");
    let mut command = Variable::new("command");
    command.push_command_escaped_path(configure_path);
    for arg in std::env::args_os().skip(1) {
        command.push_literal(" ");
        command.push_command_escaped(os_str_to_bytes(arg));
    }
    configure.add_variable(command);
    file.add_rule(configure);
}

fn add_format_builds(file: &mut NinjaFile, config: &Config) {
    let mut format = Build::new("phony");
    format.add_output(FilePath::from_literal("format"));
    format.add_implicit_input(FilePath::from_literal("ALWAYS"));

    for subproject in &config.subprojects {
        if !subproject.is_workspace_member {
            continue;
        }

        let mut build = Build::new("format");

        let mut output = FilePath::from_literal("format-");
        output.push_escaped(&subproject.name);
        build.add_output(output.clone());

        build.add_input(FilePath::from_path(&subproject.root_module));
        build.add_implicit_input(FilePath::from_literal("ALWAYS"));

        let mut in_file = Variable::new("in_file");
        in_file.push_command_escaped_path(&subproject.root_module);
        build.add_variable(in_file);

        format.add_input(output);
        file.add_build(build);
    }

    file.add_build(format);
}

fn add_rustc_builds(file: &mut NinjaFile, config: &Config) {
    let mut tools = Build::new("phony");
    tools.add_output(FilePath::from_literal("tools"));

    let tools_dir = config.arguments.out_dir.join("tools");
    for subproject_info in &config.subproject_info {
        let mut build = Build::new(subproject_info.target.rustc_rule());

        let subproject = &config.subprojects[subproject_info.index];
        let mut artifacts_path = config
            .arguments
            .build_dir
            .join(subproject_info.target.folder());
        artifacts_path.push("rustc");

        let artifact_path = artifacts_path.join(subproject_info.artifact_name(config));
        build.add_output(FilePath::from_path(&artifact_path));
        build.add_input(FilePath::from_path(&subproject.root_module));

        let mut driver_var = Variable::new("driver");
        driver_var.push_literal("$");
        driver_var.push_literal("rustc");
        build.add_variable(driver_var);

        let mut crate_name = Variable::new("crate_name");
        crate_name.push_escaped(convert_name_to_rust_name(&subproject.name));
        build.add_variable(crate_name);

        let mut crate_type = Variable::new("crate_type");
        crate_type.push_escaped(subproject.kind.crate_type());
        build.add_variable(crate_type);

        let mut extern_var = Variable::new("extern");
        for dep in subproject
            .libraries
            .iter()
            .chain(subproject_info.extra_libraries.iter())
        {
            let dep_artifact_path = artifacts_path.join(library_artifact_name(dep));

            extern_var.push_literal(" --extern '");
            extern_var.push_literal(convert_name_to_rust_name(dep));
            extern_var.push_literal("=");
            extern_var.push_command_escaped_path(&dep_artifact_path);
            extern_var.push_literal("'");

            build.add_implicit_input(FilePath::from_path(dep_artifact_path));
        }

        if !subproject.libraries.is_empty() {
            build.add_variable(extern_var);
        }

        let mut extra_args = Variable::new("extra_args");
        if let Some(linker_script) = subproject.linker_script.as_ref() {
            extra_args.push_literal("-C link-arg=-T");
            extra_args.push_command_escaped_path(linker_script);
        }

        build.add_variable(extra_args);

        let mut search_dir = Variable::new("search_dir");
        search_dir.push_command_escaped_path(artifacts_path);
        build.add_variable(search_dir);

        let mut in_file = Variable::new("in_file");
        in_file.push_command_escaped_path(&subproject.root_module);
        build.add_variable(in_file);

        let mut out_file = Variable::new("out_file");
        out_file.push_command_escaped_path(&artifact_path);
        build.add_variable(out_file);

        file.add_build(build);

        if matches!(subproject.kind, Kind::Tool) && matches!(subproject_info.target, Target::Host) {
            let mut copy = Build::new("copy");

            let in_file_path = artifact_path;
            let out_file_path = tools_dir.join(&subproject.name);

            copy.add_input(FilePath::from_path(&in_file_path));
            copy.add_output(FilePath::from_path(&out_file_path));

            let mut in_file = Variable::new("in_file");
            in_file.push_command_escaped_path(in_file_path);
            copy.add_variable(in_file);

            let mut out_file = Variable::new("out_file");
            out_file.push_command_escaped_path(&out_file_path);
            copy.add_variable(out_file);

            tools.add_input(FilePath::from_path(&out_file_path));
            file.add_build(copy);
        }
    }

    file.add_build(tools);
}

fn add_unit_test_builds(file: &mut NinjaFile, config: &Config) {
    let mut unit_test = Build::new("phony");
    unit_test.add_output(FilePath::from_literal("unit-test"));

    for subproject in &config.subprojects {
        if !subproject.is_workspace_member {
            continue;
        } else if subproject.name == "core" || subproject.name == "compiler-builtins" {
            continue;
        }

        let mut build = Build::new(Target::Build.rustc_rule());

        let mut artifacts_path = config.arguments.build_dir.join(Target::Build.folder());
        artifacts_path.push("rustc");

        let artifact_path = artifacts_path.join(format!("{}-unit-test", subproject.name));
        build.add_output(FilePath::from_path(&artifact_path));

        let mut driver = Variable::new("driver");
        driver.push_literal("$rustc");
        build.add_variable(driver);

        let mut crate_name = Variable::new("crate_name");
        crate_name.push_escaped(convert_name_to_rust_name(&subproject.name));
        build.add_variable(crate_name);

        let mut crate_type = Variable::new("crate_type");
        crate_type.push_escaped(subproject.kind.crate_type());
        build.add_variable(crate_type);

        let mut extra_args = Variable::new("extra_args");
        extra_args.push_literal("--test");
        build.add_variable(extra_args);

        let mut search_dir = Variable::new("search_dir");
        search_dir.push_command_escaped_path(artifacts_path);
        build.add_variable(search_dir);

        let mut in_file = Variable::new("in_file");
        in_file.push_command_escaped_path(&subproject.root_module);
        build.add_variable(in_file);

        let mut out_file = Variable::new("out_file");
        out_file.push_command_escaped_path(&artifact_path);
        build.add_variable(out_file);

        file.add_build(build);

        let mut execute = Build::new("execute");
        execute.add_input(FilePath::from_path(artifact_path));

        let mut unit_test_path = FilePath::from_literal("execute-unit-test-");
        unit_test_path.push_literal(&subproject.name);
        execute.add_output(unit_test_path.clone());

        let mut args = Variable::new("args");
        args.push_literal(" --color always");
        execute.add_variable(args);

        unit_test.add_input(unit_test_path);
        file.add_build(execute);
    }

    file.add_build(unit_test);
}

fn add_miri_unit_test_builds(file: &mut NinjaFile, config: &Config) {
    let mut unit_test = Build::new("phony");
    unit_test.add_output(FilePath::from_path("miri-unit-test"));

    for subproject in &config.subprojects {
        if !subproject.is_workspace_member {
            continue;
        } else if subproject.name == "core" || subproject.name == "compiler-builtins" {
            continue;
        }

        let mut build = Build::new(Target::Build.rustc_rule());

        let mut artifacts_path = config.arguments.build_dir.join(Target::Build.folder());
        artifacts_path.push("rustc");

        let artifact_path = artifacts_path.join(format!("{}-miri-unit-test", subproject.name));
        build.add_output(FilePath::from_path(&artifact_path));

        let mut driver = Variable::new("driver");
        driver.push_literal("$miri");
        build.add_variable(driver);

        let mut crate_name = Variable::new("crate_name");
        crate_name.push_escaped(convert_name_to_rust_name(&subproject.name));
        build.add_variable(crate_name);

        let mut crate_type = Variable::new("crate_type");
        crate_type.push_escaped(subproject.kind.crate_type());
        build.add_variable(crate_type);

        let mut extra_args = Variable::new("extra_args");
        extra_args.push_literal("--test");
        extra_args.push_literal(" -C opt-level=0");
        build.add_variable(extra_args);

        let mut search_dir = Variable::new("search_dir");
        search_dir.push_command_escaped_path(artifacts_path);
        build.add_variable(search_dir);

        let mut in_file = Variable::new("in_file");
        in_file.push_command_escaped_path(&subproject.root_module);
        build.add_variable(in_file);

        let mut out_file = Variable::new("out_file");
        out_file.push_command_escaped_path(&artifact_path);
        build.add_variable(out_file);

        unit_test.add_input(FilePath::from_path(artifact_path));
        file.add_build(build);
    }

    file.add_build(unit_test);
}

fn add_reconfigure_build(file: &mut NinjaFile, config: &Config) {
    let mut reconfigure = Build::new("configure");

    reconfigure.add_output(FilePath::from_path(&config.arguments.ninja_path));
    let rust_project_path = config.arguments.source_dir.join("rust-project.json");
    reconfigure.add_output(FilePath::from_path(rust_project_path));

    let mut configure_path = config.arguments.build_dir.join("build");
    configure_path.push("rustc");
    configure_path.push("configure");

    reconfigure.add_implicit_input(FilePath::from_path(configure_path));
    reconfigure.add_implicit_input(FilePath::from_path(&config.arguments.config_path));

    let mut generator_var = Variable::new("generator");
    generator_var.push_literal("1");
    reconfigure.add_variable(generator_var);

    file.add_build(reconfigure);
}
