use std::collections::HashSet;

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

    for target in targets {
        let mut doc = Rule::new(target.rustdoc_rule());
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
        command.push_command_escaped_path(
            config.arguments.build_dir.join(target.folder()).join("doc"),
        );

        command.push_literal(" $in_file");
        doc.add_variable(command);

        let mut depfile = Variable::new("depfile");
        depfile.push_literal("$out.dep");
        doc.add_variable(depfile);

        let mut deps = Variable::new("deps");
        deps.push_literal("gcc");
        doc.add_variable(deps);

        file.add_rule(doc);
    }

    let mut format = Rule::new("format");
    let mut command = Variable::new("command");
    command.push_command_escaped_path(&rustfmt_path);
    command.push_literal(" --edition 2024");
    command.push_literal(" --color always");
    command.push_literal(" $in_file");
    format.add_variable(command);
    file.add_rule(format);

    let mut copy = Rule::new("copy");
    let mut command = Variable::new("command");
    command.push_literal("cp '$in' '$out'");
    copy.add_variable(command);
    file.add_rule(copy);

    let mut configure_path = config.arguments.build_dir.join("build");
    configure_path.push("rustc");
    configure_path.push("configure");

    let mut configure = Rule::new("configure");
    let mut command = Variable::new("command");
    command.push_command_escaped_path(&configure_path);
    for arg in std::env::args_os().skip(1) {
        command.push_literal(" ");
        command.push_command_escaped_path(arg);
    }
    configure.add_variable(command);
    file.add_rule(configure);

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
            let output = FilePath::from_path(&artifact_path);
            if driver == "clippy" {
                clippy_paths.push(output.clone());
            } else if driver == "rustc"
                && matches!(subproject_info.kind, Kind::Tool)
                && matches!(subproject_info.target, Target::Host)
            {
                tool_paths.push((
                    FilePath::from_path(tools_dir.join(&subproject_info.name)),
                    output.clone(),
                ));
            }
            build.add_output(output);

            let explicit = FilePath::from_path(&subproject_info.root_module);
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
                extern_var.push_command_escaped(&dep_artifact_path);
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
                extra_args.push_literal("-C link-arg=-T");
                extra_args.push_command_escaped_path(linker_script);
            }

            build.add_variable(extra_args);

            let mut search_dir = Variable::new("search_dir");
            search_dir.push_command_escaped_path(artifacts_path);
            build.add_variable(search_dir);

            let mut in_file = Variable::new("in_file");
            in_file.push_command_escaped_path(&subproject_info.root_module);
            build.add_variable(in_file);

            let mut out_file = Variable::new("out_file");
            out_file.push_command_escaped_path(artifact_path);
            build.add_variable(out_file);

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

        let output = FilePath::from_path(&artifact_path);
        build.add_output(output);

        let mut extern_var = Variable::new("extern");
        for dep in &subproject_info.libraries {
            let mut dep_doc_path = doc_path.join(convert_name_to_rust_name(dep));
            dep_doc_path.push("index.html");

            let dep_doc_path = os_str_to_bytes(dep_doc_path.as_os_str());
            build.add_implicit(FilePath::from_escaped(dep_doc_path));

            let dep_artifact_path = artifacts_path.join(library_artifact_name(dep));
            let dep_artifact_path = os_str_to_bytes(dep_artifact_path.as_os_str());

            extern_var.push_literal(" --extern ");
            extern_var.push_literal(convert_name_to_rust_name(dep));
            extern_var.push_literal("=");
            extern_var.push_command_escaped(&dep_artifact_path);

            build.add_implicit(FilePath::from_escaped(dep_artifact_path));
        }

        let explicit = FilePath::from_path(&subproject_info.root_module);
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
        search_dir.push_command_escaped_path(artifacts_path);
        build.add_variable(search_dir);

        let mut out_dir = Variable::new("out_dir");
        out_dir.push_command_escaped_path(doc_path);
        build.add_variable(out_dir);

        let mut in_file = Variable::new("in_file");
        in_file.push_command_escaped_path(&subproject_info.root_module);
        build.add_variable(in_file);

        let mut out_file = Variable::new("out_file");
        out_file.push_command_escaped_path(artifact_path);
        build.add_variable(out_file);

        file.add_build(build);
    }

    let mut doc = Build::new("phony");
    doc.add_output(FilePath::from_literal("doc"));
    for doc_path in doc_paths {
        doc.add_explicit(FilePath::from_path(doc_path));
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

            let mut in_file = Variable::new("in_file");
            in_file.push_command_escaped_path(&subproject_info.root_module);
            format.add_variable(in_file);

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

    reconfigure.add_output(FilePath::from_path(&config.arguments.ninja_path));
    let rust_project_path = config.arguments.source_dir.join("rust-project.json");
    reconfigure.add_output(FilePath::from_path(rust_project_path));

    reconfigure.add_implicit(FilePath::from_path(configure_path));
    reconfigure.add_implicit(FilePath::from_path(&config.arguments.config_path));

    let mut generator_var = Variable::new("generator");
    generator_var.push_literal("1");
    reconfigure.add_variable(generator_var);

    file.add_build(reconfigure);

    let mut always = Build::new("phony");
    always.add_output(FilePath::from_literal("ALWAYS"));
    file.add_build(always);

    file.add_default(FilePath::from_literal("tools"));

    file.write_out(output);
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
