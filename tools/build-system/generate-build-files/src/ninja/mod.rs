//! `build.ninja` file generation functionality.

use std::path::{Path, PathBuf};

use crate::{
    config::{
        Config,
        subproject::{
            Binary, BinaryTarget, LibraryTarget, Subproject,
            Target::{self},
        },
    },
    convert_name_to_rust_name,
    ninja::utils::{Argument, Build, FilePath, NinjaFile, Rule, Variable},
};

mod utils;

/// Generates the `build.ninja` file contents.
pub fn generate(output: &mut Vec<u8>, config: &Config) {
    // Add the ninja build system variables separately.
    let mut builddir = Variable::new("builddir");
    builddir.push_escaped_path(&config.cli.build_dir_path);
    builddir.write_out(output);

    let mut ninja_required_version = Variable::new("ninja_required_version");
    ninja_required_version.push_escaped_str("1.3");
    ninja_required_version.write_out(output);
    output.push(b'\n');

    let mut file = NinjaFile::new();

    add_rust_binaries(&mut file, config);
    add_rust_cfgs(&mut file, config);
    add_rust_lints(&mut file);

    add_rust_rules(&mut file, config);
    add_misc_rules(&mut file, config);

    add_rustfmt_builds(&mut file, config);
    add_rustc_builds(&mut file, config);
    add_clippy_builds(&mut file, config);
    add_rustdoc_builds(&mut file, config, false);
    add_rustdoc_builds(&mut file, config, true);
    add_unit_test_builds(&mut file, config);

    let miri_sysroot = add_miri_sysroot(&mut file, config);
    add_miri_builds(&mut file, config, &miri_sysroot);
    add_miri_unit_test_builds(&mut file, config, &miri_sysroot);

    add_regenerate_build(&mut file, config);

    // Add the `ALWAYS` build to enable executing something whenever it is part of the execution
    // graph.
    let mut always = Build::new("phony");
    always.add_output(FilePath::from_literal("ALWAYS"));
    file.add_build(always);

    file.add_default(FilePath::from_literal("build-tools"));

    file.write_out(output);
}

/// Adds basic [`Variable`]s for easy access to various rust binaries.
fn add_rust_binaries(file: &mut NinjaFile, config: &Config) {
    let rustc_sysroot_bin_path = config.rustc_sysroot.join("bin");

    let cargo_path = rustc_sysroot_bin_path.join("cargo");
    let rustc_path = rustc_sysroot_bin_path.join("rustc");
    let clippy_path = rustc_sysroot_bin_path.join("clippy-driver");
    let miri_path = rustc_sysroot_bin_path.join("miri");
    let rustfmt_path = rustc_sysroot_bin_path.join("rustfmt");
    let rustdoc_path = rustc_sysroot_bin_path.join("rustdoc");

    let mut cargo = Variable::new("cargo");
    let mut cargo_arg = Argument::new();
    cargo_arg.push_path(cargo_path);
    cargo.push_escaped_argument(&cargo_arg);
    file.add_variable(cargo);

    let mut rustc = Variable::new("rustc");
    let mut rustc_arg = Argument::new();
    rustc_arg.push_path(rustc_path);
    rustc.push_escaped_argument(&rustc_arg);
    file.add_variable(rustc);

    let mut clippy = Variable::new("clippy");
    let mut clippy_arg = Argument::new();
    clippy_arg.push_path(clippy_path);
    clippy.push_escaped_argument(&clippy_arg);
    file.add_variable(clippy);

    let mut miri = Variable::new("miri");
    let mut miri_arg = Argument::new();
    miri_arg.push_path(miri_path);
    miri.push_escaped_argument(&miri_arg);
    file.add_variable(miri);

    let mut rustdoc = Variable::new("rustdoc");
    let mut rustdoc_arg = Argument::new();
    rustdoc_arg.push_path(rustdoc_path);
    rustdoc.push_escaped_argument(&rustdoc_arg);
    file.add_variable(rustdoc);

    let mut rustfmt = Variable::new("rustfmt");
    let mut rustfmt_arg = Argument::new();
    rustfmt_arg.push_path(rustfmt_path);
    rustfmt.push_escaped_argument(&rustfmt_arg);
    file.add_variable(rustfmt);
}

/// Adds a [`Variable`] for `cfg`s.
fn add_rust_cfgs(file: &mut NinjaFile, config: &Config) {
    let mut rust_cfgs = Variable::new("rust_cfgs");

    let mut kconfig = config.kconfig.iter().collect::<Vec<_>>();
    kconfig.sort_unstable();

    for (name, value) in kconfig {
        rust_cfgs.push_escaped_str(" --cfg");
        rust_cfgs.push_escaped_str(" ");
        rust_cfgs.push_escaped_argument(&Argument::new_str(name));

        rust_cfgs.push_escaped_str(" --cfg");
        rust_cfgs.push_escaped_str(" ");
        let mut cfg_arg = Argument::new();
        cfg_arg.push_str(name);
        cfg_arg.push_str("=\"");
        for c in value.chars() {
            if c == '\"' {
                cfg_arg.push_str("\\");
                cfg_arg.push_str("\"");
            } else {
                let mut buffer = [0; 4];
                cfg_arg.push_str(c.encode_utf8(&mut buffer));
            }
        }
        cfg_arg.push_str("\"");
        rust_cfgs.push_escaped_argument(&cfg_arg);
    }

    file.add_variable(rust_cfgs);
}

/// Adds pre-configured [`Variable`]s representing the global lints for this project.
fn add_rust_lints(file: &mut NinjaFile) {
    let mut rust_lints = Variable::new("rust_lints");
    rust_lints.push_escaped_str("-Wunsafe-op-in-unsafe-fn");
    rust_lints.push_escaped_str(" -Wmissing-docs");
    file.add_variable(rust_lints);

    let mut clippy_lints = Variable::new("clippy_lints");
    clippy_lints.push_escaped_str("-Wclippy::undocumented_unsafe_blocks");
    clippy_lints.push_escaped_str(" -Wclippy::multiple_unsafe_ops_per_block");
    clippy_lints.push_escaped_str(" -Wclippy::missing_safety_doc");
    clippy_lints.push_escaped_str(" -Wclippy::missing_errors_doc");
    clippy_lints.push_escaped_str(" -Wclippy::missing_panics_doc");
    clippy_lints.push_escaped_str(" -Wclippy::missing_docs_in_private_items");
    clippy_lints.push_escaped_str(" -Wclippy::cast_possible_truncation");
    clippy_lints.push_escaped_str(" -Wclippy::cast_possible_wrap");
    clippy_lints.push_escaped_str(" -Wclippy::cast_precision_loss");
    clippy_lints.push_escaped_str(" -Wclippy::cast_sign_loss");
    clippy_lints.push_escaped_str(" -Wclippy::ptr_as_ptr");
    file.add_variable(clippy_lints);
}

/// Adds the various rules relevant to rust to the [`NinjaFile`].
fn add_rust_rules(file: &mut NinjaFile, config: &Config) {
    add_rustfmt_rule(file, config);

    for target in Target::ALL_TARGETS {
        add_rustc_rule(file, config, target);
    }

    for target in Target::ALL_TARGETS {
        add_rustdoc_rule(file, config, target);
    }
}
/// Adds the [`Rule`] to call `rustdoc` on a file for the provided [`Target`].
fn add_rustc_rule(file: &mut NinjaFile, config: &Config, target: Target) {
    let mut rule = Rule::new(target.rustc_rule());

    let mut command = Variable::new("command");
    command.push_literal_str("$driver");
    command.push_escaped_str(" --color always");

    add_arguments(config, &mut command, target);
    command.push_escaped_str(" --crate-name");
    command.push_literal_str(" $crate_name");
    command.push_escaped_str(" --crate-type");
    command.push_literal_str(" $crate_type");
    command.push_escaped_str(" --edition 2024");
    command.push_escaped_str(" -L");
    command.push_literal_str(" $search_dir");
    command.push_literal_str(" $extern");
    command.push_literal_str(" $lints");
    command.push_literal_str(" $extra_args");
    command.push_escaped_str(" --emit");
    command.push_literal_str(" link=$out_file");
    command.push_escaped_str(" --emit");
    command.push_literal_str(" dep-info=$out_file.dep");

    command.push_escaped_str(" --out-dir");
    command.push_literal_str(" ");
    let out_dir_path = config.cli.build_dir_path.join(target.folder());
    command.push_escaped_argument(&Argument::new_path(out_dir_path));

    command.push_literal_str(" $in_file");
    rule.add_variable(command);

    let mut depfile = Variable::new("depfile");
    depfile.push_literal_str("$out.dep");
    rule.add_variable(depfile);

    let mut deps = Variable::new("deps");
    deps.push_escaped_str("gcc");
    rule.add_variable(deps);

    file.add_rule(rule);
}

/// Adds the [`Rule`] to call `rustdoc` on a file for the provided [`Target`].
fn add_rustdoc_rule(file: &mut NinjaFile, config: &Config, target: Target) {
    let mut rule = Rule::new(target.rustdoc_rule());

    let mut command = Variable::new("command");
    command.push_literal_str("$rustdoc");
    command.push_escaped_str(" --color always");

    add_arguments(config, &mut command, target);
    command.push_escaped_str(" --crate-name");
    command.push_literal_str(" $crate_name");
    command.push_escaped_str(" --crate-type");
    command.push_literal_str(" $crate_type");
    command.push_escaped_str(" --edition 2024");
    command.push_escaped_str(" -L");
    command.push_literal_str(" $search_dir");
    command.push_literal_str(" $extern");
    command.push_literal_str(" $extra_args");
    command.push_escaped_str(" --emit html-static-files");
    command.push_escaped_str(" --emit html-non-static-files");
    command.push_escaped_str(" --emit");
    command.push_literal_str(" dep-info=$out_file.dep");

    command.push_escaped_str(" --out-dir");
    command.push_literal_str(" $out_dir");

    command.push_literal_str(" $in_file");
    rule.add_variable(command);

    let mut depfile = Variable::new("depfile");
    depfile.push_literal_str("$out.dep");
    rule.add_variable(depfile);

    let mut deps = Variable::new("deps");
    deps.push_escaped_str("gcc");
    rule.add_variable(deps);

    file.add_rule(rule);
}

/// Adds the arguments associated with a given [`Target`] to the provided `command` [`Variable`].
fn add_arguments(config: &Config, command: &mut Variable, target: Target) {
    match target {
        Target::Build => {}
        Target::Host => {
            if let Some(triplet) = config.cli.host_triplet.as_ref() {
                command.push_escaped_str(" --target ");
                command.push_escaped_argument(&Argument::new_os_str(triplet));
            }
        }
        Target::Revm | Target::RevmStub => {
            let triplet = if target == Target::Revm {
                &config.revm_triplet_path
            } else {
                &config.revm_stub_triplet_path
            };

            command.push_escaped_str(" -Zunstable-options");
            command.push_escaped_str(" --target ");
            command.push_escaped_argument(&Argument::new_path(triplet));

            command.push_escaped_str(" -C relocation-model=pie");
            command.push_escaped_str(" -C target-feature=+crt-static");

            command.push_escaped_str(" -Z direct-access-external-data=yes");
        }
    }

    let opts = match target {
        Target::Build | Target::Host => &config.general_opts,
        Target::Revm => &config.revm_opts,
        Target::RevmStub => &config.revm_stub_opts,
    };

    command.push_escaped_str(" -C opt-level=");
    command.push_escaped_str(opts.opt_level.option());

    if opts.incremental {
        let mut incremental_dir = config.cli.build_dir_path.join(target.folder());
        incremental_dir.push("cache");

        command.push_escaped_str(" -C incremental=");
        command.push_escaped_argument(&Argument::new_path(incremental_dir));
    }

    command.push_escaped_str(" -C debug-assertions=");
    command.push_escaped_str(&format!("{}", opts.debug_assertions));

    command.push_escaped_str(" -C debuginfo=");
    if opts.debug_info {
        command.push_escaped_str("full");
    } else {
        command.push_escaped_str("none");
    }

    command.push_escaped_str(" -C lto=");
    command.push_escaped_str(opts.lto.option());

    command.push_literal_str(" $cfgs");
}

/// Adds the [`Rule`] to call `rustfmt` on a file.
fn add_rustfmt_rule(file: &mut NinjaFile, _: &Config) {
    let mut format = Rule::new("rustfmt");
    let mut command = Variable::new("command");
    command.push_literal_str("$rustfmt");
    command.push_escaped_str(" --edition 2024");
    command.push_escaped_str(" --color always");
    command.push_literal_str(" $in_files");
    format.add_variable(command);
    file.add_rule(format);
}

/// Adds miscellaneous [`Rule`]s used for various [`Build`] statements.
fn add_misc_rules(file: &mut NinjaFile, _: &Config) {
    let mut execute = Rule::new("execute");
    let mut command = Variable::new("command");
    command.push_literal_str("$binary $args");
    execute.add_variable(command);
    file.add_rule(execute);

    #[cfg(unix)]
    {
        let mut copy = Rule::new("copy");
        let mut command = Variable::new("command");
        command.push_escaped_str("cp");
        command.push_literal_str(" $in_file");
        command.push_literal_str(" $out_file");
        copy.add_variable(command);
        file.add_rule(copy);
    }
    #[cfg(not(unix))]
    todo!("write copy command")
}

/// Adds [`Build`] statements to format all [`Subproject`]s.
fn add_rustfmt_builds(file: &mut NinjaFile, config: &Config) {
    let mut format = Build::new("phony");
    format.add_output(FilePath::from_literal("format"));
    format.add_implicit_input(FilePath::from_literal("ALWAYS"));

    let mut format_subprojects = Vec::new();

    for subproject in &config.subprojects {
        if !subproject.is_workspace_member() {
            continue;
        }

        let mut format_subproject = Build::new("phony");
        let output = FilePath::from_literal(format!("format-{}", subproject.name()));
        format_subproject.add_output(output.clone());
        format_subproject.add_implicit_input(FilePath::from_literal("ALWAYS"));

        let library_iter = subproject
            .library()
            .map(|library| (true, subproject.name(), library.root_module()))
            .into_iter();
        let binary_iter = subproject
            .binaries()
            .iter()
            .map(|binary| (false, binary.name(), binary.root_module()));
        let iter = library_iter.chain(binary_iter);

        let mut format_paths = Vec::new();
        for (is_library, name, root_module_path) in iter {
            let mut build = Build::new("rustfmt");

            let mut output = output.clone();
            if is_library {
                output.push_escaped("-lib");
            } else if subproject.binaries().len() == 1 {
                output.push_escaped("-bin");
            } else {
                output.push_escaped("-bin-");
                output.push_escaped(name);
            };
            build.add_output(output.clone());

            build.add_input(FilePath::from_path(root_module_path));
            build.add_implicit_input(FilePath::from_literal("ALWAYS"));

            let mut in_files = Variable::new("in_files");
            in_files.push_escaped_argument(&Argument::new_path(root_module_path));
            build.add_variable(in_files);

            format_subproject.add_input(output);
            format_paths.push(build);
        }

        format.add_input(output);
        format_subprojects.push((format_subproject, format_paths));
    }

    file.add_build(format);
    for (format_subproject, format_paths) in format_subprojects {
        file.add_build(format_subproject);
        for format_path in format_paths {
            file.add_build(format_path);
        }
    }
}

/// Adds [`Build`] statements to compile all [`Subproject`]s.
fn add_rustc_builds(file: &mut NinjaFile, config: &Config) {
    let mut build_tool_descs = Vec::new();
    let mut host_tool_descs = Vec::new();

    for target in Target::ALL_TARGETS {
        for subproject in &config.subprojects {
            add_rustc_library(file, config, target, subproject);
            for binary in subproject.binaries() {
                let Some(path) = add_rustc_binary(file, config, target, subproject, binary) else {
                    continue;
                };

                match target {
                    Target::Build => build_tool_descs.push((binary.name(), path)),
                    Target::Host => host_tool_descs.push((binary.name(), path)),
                    Target::Revm | Target::RevmStub => {}
                }
            }
        }
    }

    let build_tools_path = config.cli.build_dir_path.join("tools");
    let host_tools_path = config.cli.out_dir_path.join("tools");

    let mut build_tools = Build::new("phony");
    build_tools.add_output(FilePath::from_literal("build-tools"));
    for (name, tool_path) in build_tool_descs {
        let mut copy = Build::new("copy");
        copy.add_input(FilePath::from_path(&tool_path));

        let output_path = build_tools_path.join(name);
        copy.add_output(FilePath::from_path(&output_path));

        let mut in_file = Variable::new("in_file");
        in_file.push_escaped_argument(&Argument::new_path(&tool_path));
        copy.add_variable(in_file);

        let mut out_file = Variable::new("out_file");
        out_file.push_escaped_argument(&Argument::new_path(&output_path));
        copy.add_variable(out_file);

        file.add_build(copy);
        build_tools.add_input(FilePath::from_path(output_path));
    }
    file.add_build(build_tools);

    let mut host_tools = Build::new("phony");
    host_tools.add_output(FilePath::from_literal("host-tools"));
    for (name, tool_path) in host_tool_descs {
        let mut copy = Build::new("copy");
        copy.add_input(FilePath::from_path(&tool_path));

        let output_path = host_tools_path.join(name);
        copy.add_output(FilePath::from_path(&output_path));

        let mut in_file = Variable::new("in_file");
        in_file.push_escaped_argument(&Argument::new_path(&tool_path));
        copy.add_variable(in_file);

        let mut out_file = Variable::new("out_file");
        out_file.push_escaped_argument(&Argument::new_path(&output_path));
        copy.add_variable(out_file);

        file.add_build(copy);
        host_tools.add_input(FilePath::from_path(output_path));
    }
    file.add_build(host_tools);
}

/// Adds a [`Build`] statement to compile the [`Subproject`]'s library for the provided [`Target`].
fn add_rustc_library(
    file: &mut NinjaFile,
    config: &Config,
    target: Target,
    subproject: &Subproject,
) {
    let Some(library) = subproject.library() else {
        return;
    };
    let Some(library_target) = library.library_target(target) else {
        return;
    };

    let mut artifacts_path = config.cli.build_dir_path.join(target.folder());
    artifacts_path.push("rustc");

    let crate_name = convert_name_to_rust_name(subproject.name());
    let output_artifact_path = artifacts_path.join(format!("lib{crate_name}.rlib"));

    let mut build = Build::new(target.rustc_rule());
    build.add_output(FilePath::from_path(&output_artifact_path));
    build.add_input(FilePath::from_path(library.root_module()));

    let mut driver = Variable::new("driver");
    driver.push_literal_str("$rustc");
    build.add_variable(driver);

    let mut crate_name_var = Variable::new("crate_name");
    crate_name_var.push_escaped_str(&crate_name);
    build.add_variable(crate_name_var);

    let mut crate_type = Variable::new("crate_type");
    crate_type.push_escaped_str("lib");
    build.add_variable(crate_type);

    let mut extern_var = Variable::new("extern");
    for dep in library.dependencies().chain(library_target.dependencies()) {
        let dep_name = convert_name_to_rust_name(dep);
        let dep_artifact_path = artifacts_path.join(format!("lib{dep_name}.rlib"));

        extern_var.push_escaped_str(" --extern");
        extern_var.push_escaped_str(" ");
        extern_var.push_escaped_str(&dep_name);
        extern_var.push_literal_str("=");
        extern_var.push_escaped_argument(&Argument::new_path(&dep_artifact_path));

        build.add_implicit_input(FilePath::from_path(dep_artifact_path));
    }

    if !extern_var.is_empty() {
        build.add_variable(extern_var);
    }

    let mut search_dir = Variable::new("search_dir");
    search_dir.push_escaped_argument(&Argument::new_path(artifacts_path));
    build.add_variable(search_dir);

    let mut in_file = Variable::new("in_file");
    in_file.push_escaped_argument(&Argument::new_path(library.root_module()));
    build.add_variable(in_file);

    let mut out_file = Variable::new("out_file");
    out_file.push_escaped_argument(&Argument::new_path(&output_artifact_path));
    build.add_variable(out_file);

    if subproject.is_workspace_member() {
        let mut lints = Variable::new("lints");
        lints.push_literal_str("$rust_lints");
        build.add_variable(lints);
    }

    file.add_build(build);
}

/// Adds a [`Build`] statement to compile the [`Binary`] associated with the [`Subproject`] for the
/// provided [`Target`].
fn add_rustc_binary(
    file: &mut NinjaFile,
    config: &Config,
    target: Target,
    subproject: &Subproject,
    binary: &Binary,
) -> Option<PathBuf> {
    let binary_target = binary.binary_target(target)?;

    let mut artifacts_path = config.cli.build_dir_path.join(target.folder());
    artifacts_path.push("rustc");

    let crate_name = convert_name_to_rust_name(binary.name());
    let output_artifact_path = artifacts_path.join(&crate_name);

    let mut build = Build::new(target.rustc_rule());
    build.add_output(FilePath::from_path(&output_artifact_path));
    build.add_input(FilePath::from_path(binary.root_module()));

    let mut driver = Variable::new("driver");
    driver.push_literal_str("$rustc");
    build.add_variable(driver);

    let mut crate_name_var = Variable::new("crate_name");
    crate_name_var.push_escaped_str(&crate_name);
    build.add_variable(crate_name_var);

    let mut crate_type = Variable::new("crate_type");
    crate_type.push_escaped_str("bin");
    build.add_variable(crate_type);

    let mut extern_var = Variable::new("extern");

    let library_opt = if subproject.library().is_some() {
        Some(subproject.name())
    } else {
        None
    };

    for dep in library_opt
        .into_iter()
        .chain(binary.dependencies())
        .chain(binary_target.dependencies())
    {
        let dep_name = convert_name_to_rust_name(dep);
        let dep_artifact_path = artifacts_path.join(format!("lib{dep_name}.rlib"));

        extern_var.push_escaped_str(" --extern");
        extern_var.push_escaped_str(" ");
        extern_var.push_escaped_str(&dep_name);
        extern_var.push_literal_str("=");
        extern_var.push_escaped_argument(&Argument::new_path(&dep_artifact_path));

        build.add_implicit_input(FilePath::from_path(dep_artifact_path));
    }

    if !extern_var.is_empty() {
        build.add_variable(extern_var);
    }

    let mut extra_args = Variable::new("extra_args");
    if let Some(linker_script) = binary_target.linker_script().as_ref() {
        extra_args.push_escaped_str("-C link-arg=-T");
        extra_args.push_escaped_argument(&Argument::new_path(linker_script));
    }
    build.add_variable(extra_args);

    let mut search_dir = Variable::new("search_dir");
    search_dir.push_escaped_argument(&Argument::new_path(artifacts_path));
    build.add_variable(search_dir);

    let mut in_file = Variable::new("in_file");
    in_file.push_escaped_argument(&Argument::new_path(binary.root_module()));
    build.add_variable(in_file);

    let mut out_file = Variable::new("out_file");
    out_file.push_escaped_argument(&Argument::new_path(&output_artifact_path));
    build.add_variable(out_file);

    if subproject.is_workspace_member() {
        let mut lints = Variable::new("lints");
        lints.push_literal_str("$rust_lints");
        build.add_variable(lints);
    }

    file.add_build(build);
    Some(output_artifact_path)
}

/// Adds [`Build`] statements to compile all [`Subproject`]s.
fn add_clippy_builds(file: &mut NinjaFile, config: &Config) {
    let mut builds = Vec::new();
    for target in Target::ALL_TARGETS {
        for subproject in &config.subprojects {
            add_clippy_library(&mut builds, config, target, subproject);
            for binary in subproject.binaries() {
                add_clippy_binary(&mut builds, config, target, subproject, binary);
            }
        }
    }

    let mut clippy = Build::new("phony");
    clippy.add_output(FilePath::from_literal("clippy"));
    for (_, output_path) in builds.iter_mut() {
        let output_path = std::mem::replace(output_path, FilePath::new());
        clippy.add_input(output_path);
    }

    file.add_build(clippy);
    for (build, _) in builds.into_iter() {
        file.add_build(build);
    }
}

/// Adds a [`Build`] statement to lint the [`Subproject`]'s library for the provided [`Target`].
fn add_clippy_library(
    builds: &mut Vec<(Build, FilePath)>,
    config: &Config,
    target: Target,
    subproject: &Subproject,
) {
    if subproject.name() == "core" {
        return;
    }

    let Some(library) = subproject.library() else {
        return;
    };
    let Some(library_target) = library.library_target(target) else {
        return;
    };

    let mut artifacts_path = config.cli.build_dir_path.join(target.folder());
    artifacts_path.push("clippy");

    let crate_name = convert_name_to_rust_name(subproject.name());
    let output_artifact_path = artifacts_path.join(format!("lib{crate_name}.rlib"));

    let mut build = Build::new(target.rustc_rule());
    build.add_output(FilePath::from_path(&output_artifact_path));
    build.add_input(FilePath::from_path(library.root_module()));

    let mut driver = Variable::new("driver");
    driver.push_literal_str("$clippy");
    build.add_variable(driver);

    let mut crate_name_var = Variable::new("crate_name");
    crate_name_var.push_escaped_str(&crate_name);
    build.add_variable(crate_name_var);

    let mut crate_type = Variable::new("crate_type");
    crate_type.push_escaped_str("lib");
    build.add_variable(crate_type);

    let mut extern_var = Variable::new("extern");
    for dep in library.dependencies().chain(library_target.dependencies()) {
        let dep_name = convert_name_to_rust_name(dep);
        let dep_artifact_path = if dep == "core" {
            let mut core_artifact_path = config.cli.build_dir_path.join(target.folder());
            core_artifact_path.push("rustc");
            core_artifact_path.push(format!("lib{dep_name}.rlib"));
            core_artifact_path
        } else {
            artifacts_path.join(format!("lib{dep_name}.rlib"))
        };

        extern_var.push_escaped_str(" --extern");
        extern_var.push_escaped_str(" ");
        extern_var.push_escaped_str(&dep_name);
        extern_var.push_literal_str("=");
        extern_var.push_escaped_argument(&Argument::new_path(&dep_artifact_path));

        build.add_implicit_input(FilePath::from_path(dep_artifact_path));
    }

    if !extern_var.is_empty() {
        build.add_variable(extern_var);
    }

    let mut search_dir = Variable::new("search_dir");
    search_dir.push_escaped_argument(&Argument::new_path(artifacts_path));
    build.add_variable(search_dir);

    let mut in_file = Variable::new("in_file");
    in_file.push_escaped_argument(&Argument::new_path(library.root_module()));
    build.add_variable(in_file);

    let mut out_file = Variable::new("out_file");
    out_file.push_escaped_argument(&Argument::new_path(&output_artifact_path));
    build.add_variable(out_file);

    let mut lints = Variable::new("lints");
    lints.push_literal_str("$rust_lints");
    build.add_variable(lints);

    builds.push((build, FilePath::from_path(&output_artifact_path)));
}

/// Adds a [`Build`] statement to compile the [`Binary`] associated with the [`Subproject`] for the
/// provided [`Target`].
fn add_clippy_binary(
    builds: &mut Vec<(Build, FilePath)>,
    config: &Config,
    target: Target,
    subproject: &Subproject,
    binary: &Binary,
) {
    if subproject.name() == "core" {
        return;
    }

    let Some(binary_target) = binary.binary_target(target) else {
        return;
    };

    let mut artifacts_path = config.cli.build_dir_path.join(target.folder());
    artifacts_path.push("clippy");

    let crate_name = convert_name_to_rust_name(binary.name());
    let output_artifact_path = artifacts_path.join(&crate_name);

    let mut build = Build::new(target.rustc_rule());
    build.add_output(FilePath::from_path(&output_artifact_path));
    build.add_input(FilePath::from_path(binary.root_module()));
    build.add_implicit_input(FilePath::from_literal("ALWAYS"));

    let mut driver = Variable::new("driver");
    driver.push_literal_str("$clippy");
    build.add_variable(driver);

    let mut crate_name_var = Variable::new("crate_name");
    crate_name_var.push_escaped_str(&crate_name);
    build.add_variable(crate_name_var);

    let mut crate_type = Variable::new("crate_type");
    crate_type.push_escaped_str("bin");
    build.add_variable(crate_type);

    let mut extern_var = Variable::new("extern");

    let library_opt = if subproject.library().is_some() {
        Some(subproject.name())
    } else {
        None
    };

    for dep in library_opt
        .into_iter()
        .chain(binary.dependencies())
        .chain(binary_target.dependencies())
    {
        let dep_name = convert_name_to_rust_name(dep);
        let dep_artifact_path = if dep == "core" {
            let mut core_artifact_path = config.cli.build_dir_path.join(target.folder());
            core_artifact_path.push("rustc");
            core_artifact_path.push(format!("lib{dep_name}.rlib"));
            core_artifact_path
        } else {
            artifacts_path.join(format!("lib{dep_name}.rlib"))
        };

        extern_var.push_escaped_str(" --extern");
        extern_var.push_escaped_str(" ");
        extern_var.push_escaped_str(&dep_name);
        extern_var.push_literal_str("=");
        extern_var.push_escaped_argument(&Argument::new_path(&dep_artifact_path));

        build.add_implicit_input(FilePath::from_path(dep_artifact_path));
    }

    if !extern_var.is_empty() {
        build.add_variable(extern_var);
    }

    let mut extra_args = Variable::new("extra_args");
    if let Some(linker_script) = binary_target.linker_script().as_ref() {
        extra_args.push_escaped_str("-C link-arg=-T");
        extra_args.push_escaped_argument(&Argument::new_path(linker_script));
    }
    build.add_variable(extra_args);

    let mut search_dir = Variable::new("search_dir");
    search_dir.push_escaped_argument(&Argument::new_path(artifacts_path));
    build.add_variable(search_dir);

    let mut in_file = Variable::new("in_file");
    in_file.push_escaped_argument(&Argument::new_path(binary.root_module()));
    build.add_variable(in_file);

    let mut out_file = Variable::new("out_file");
    out_file.push_escaped_argument(&Argument::new_path(&output_artifact_path));
    build.add_variable(out_file);

    let mut lints = Variable::new("lints");
    lints.push_literal_str("$rust_lints");
    lints.push_literal_str(" $clippy_lints");
    build.add_variable(lints);

    builds.push((build, FilePath::from_path(&output_artifact_path)));
}

/// Adds [`Build`] statements to document all [`Subproject`]s.
fn add_rustdoc_builds(file: &mut NinjaFile, config: &Config, private: bool) {
    let mut builds = Vec::new();
    for target in Target::ALL_TARGETS {
        for subproject in &config.subprojects {
            add_rustdoc_library(&mut builds, config, target, subproject, private);
            for binary in subproject.binaries() {
                add_rustdoc_binary(&mut builds, config, target, subproject, binary, private);
            }
        }
    }

    let mut doc = Build::new("phony");
    if private {
        doc.add_output(FilePath::from_literal("doc-private"));
    } else {
        doc.add_output(FilePath::from_literal("doc"));
    }
    for (_, output_path) in builds.iter_mut() {
        let output_path = std::mem::replace(output_path, FilePath::new());
        doc.add_input(output_path);
    }

    file.add_build(doc);
    for (build, _) in builds.into_iter() {
        file.add_build(build);
    }
}

/// Adds a [`Build`] statement to document the [`Subproject`]'s library for the provided
/// [`Target`].
///
/// `private` controls whether private items are also documented.
fn add_rustdoc_library(
    builds: &mut Vec<(Build, FilePath)>,
    config: &Config,
    target: Target,
    subproject: &Subproject,
    private: bool,
) {
    let Some(library) = subproject.library() else {
        return;
    };
    let Some(library_target) = library.library_target(target) else {
        return;
    };

    let mut artifacts_path = config.cli.build_dir_path.join(target.folder());
    if private {
        artifacts_path.push("doc-private");
    } else {
        artifacts_path.push("doc");
    }

    let crate_name = convert_name_to_rust_name(subproject.name());
    let mut output_artifact_path = artifacts_path.join(&crate_name);
    output_artifact_path.push("index.html");

    let mut build = Build::new(target.rustdoc_rule());
    build.add_output(FilePath::from_path(&output_artifact_path));
    build.add_input(FilePath::from_path(library.root_module()));

    let mut crate_name_var = Variable::new("crate_name");
    crate_name_var.push_escaped_str(&crate_name);
    build.add_variable(crate_name_var);

    let mut crate_type = Variable::new("crate_type");
    crate_type.push_escaped_str("lib");
    build.add_variable(crate_type);

    let mut extern_var = Variable::new("extern");
    let mut rustc_artifacts_path = config.cli.build_dir_path.join(target.folder());
    rustc_artifacts_path.push("rustc");
    for dep in library.dependencies().chain(library_target.dependencies()) {
        let dep_name = convert_name_to_rust_name(dep);
        let dep_artifact_path = rustc_artifacts_path.join(format!("lib{dep_name}.rlib"));

        extern_var.push_escaped_str(" --extern");
        extern_var.push_escaped_str(" ");
        extern_var.push_escaped_str(&dep_name);
        extern_var.push_literal_str("=");
        extern_var.push_escaped_argument(&Argument::new_path(&dep_artifact_path));

        build.add_implicit_input(FilePath::from_path(dep_artifact_path));
    }

    if !extern_var.is_empty() {
        build.add_variable(extern_var);
    }

    let mut search_dir = Variable::new("search_dir");
    search_dir.push_escaped_argument(&Argument::new_path(rustc_artifacts_path));
    build.add_variable(search_dir);

    let mut in_file = Variable::new("in_file");
    in_file.push_escaped_argument(&Argument::new_path(library.root_module()));
    build.add_variable(in_file);

    let mut out_dir = Variable::new("out_dir");
    out_dir.push_escaped_argument(&Argument::new_path(&artifacts_path));
    build.add_variable(out_dir);

    let mut out_file = Variable::new("out_file");
    out_file.push_escaped_argument(&Argument::new_path(&output_artifact_path));
    build.add_variable(out_file);

    builds.push((build, FilePath::from_path(output_artifact_path)));
}

/// Adds a [`Build`] statement to document the [`Binary`] associated with the [`Subproject`] for the
/// provided [`Target`].
///
/// `private` controls whether private items are also documented.
fn add_rustdoc_binary(
    builds: &mut Vec<(Build, FilePath)>,
    config: &Config,
    target: Target,
    subproject: &Subproject,
    binary: &Binary,
    private: bool,
) {
    let Some(binary_target) = binary.binary_target(target) else {
        return;
    };

    let mut artifacts_path = config.cli.build_dir_path.join(target.folder());
    if private {
        artifacts_path.push("doc-private");
    } else {
        artifacts_path.push("doc");
    }

    let crate_name = convert_name_to_rust_name(subproject.name());
    let mut output_artifact_path = artifacts_path.join(&crate_name);
    output_artifact_path.push("index.html");

    let mut build = Build::new(target.rustdoc_rule());
    build.add_output(FilePath::from_path(&output_artifact_path));
    build.add_input(FilePath::from_path(binary.root_module()));

    let mut crate_name_var = Variable::new("crate_name");
    crate_name_var.push_escaped_str(&crate_name);
    build.add_variable(crate_name_var);

    let mut crate_type = Variable::new("crate_type");
    crate_type.push_escaped_str("bin");
    build.add_variable(crate_type);

    let mut extern_var = Variable::new("extern");

    let library_opt = if subproject.library().is_some() {
        Some(subproject.name())
    } else {
        None
    };

    let mut rustc_artifacts_path = config.cli.build_dir_path.join(target.folder());
    rustc_artifacts_path.push("rustc");
    for dep in library_opt
        .into_iter()
        .chain(binary.dependencies())
        .chain(binary_target.dependencies())
    {
        let dep_name = convert_name_to_rust_name(dep);
        let dep_artifact_path = rustc_artifacts_path.join(format!("lib{dep_name}.rlib"));

        extern_var.push_escaped_str(" --extern");
        extern_var.push_escaped_str(" ");
        extern_var.push_escaped_str(&dep_name);
        extern_var.push_literal_str("=");
        extern_var.push_escaped_argument(&Argument::new_path(&dep_artifact_path));

        build.add_implicit_input(FilePath::from_path(dep_artifact_path));
    }

    if !extern_var.is_empty() {
        build.add_variable(extern_var);
    }

    let mut search_dir = Variable::new("search_dir");
    search_dir.push_escaped_argument(&Argument::new_path(rustc_artifacts_path));
    build.add_variable(search_dir);

    let mut in_file = Variable::new("in_file");
    in_file.push_escaped_argument(&Argument::new_path(binary.root_module()));
    build.add_variable(in_file);

    if private {
        let mut extra_args = Variable::new("extra_args");
        extra_args.push_escaped_str("--document-private-items");
        build.add_variable(extra_args);
    }

    let mut out_dir = Variable::new("out_dir");
    out_dir.push_escaped_argument(&Argument::new_path(&artifacts_path));
    build.add_variable(out_dir);

    let mut out_file = Variable::new("out_file");
    out_file.push_escaped_argument(&Argument::new_path(&output_artifact_path));
    build.add_variable(out_file);

    builds.push((build, FilePath::from_path(output_artifact_path)));
}

/// Adds a [`Build`] for `miri`'s `sysroot` to ensure proper sequencing between setting the sysroot
/// up and utilizing `miri`'s sysroot.
fn add_miri_sysroot(file: &mut NinjaFile, config: &Config) -> PathBuf {
    let mut build = Build::new("execute");
    let output_path = config.cli.build_dir_path.join("miri-sysroot");
    build.add_output(FilePath::from_path(&output_path));

    let mut binary = Variable::new("binary");
    binary.push_literal_str("$cargo");
    build.add_variable(binary);

    let mut args = Variable::new("args");
    args.push_escaped_str("miri");
    args.push_escaped_str(" setup");
    args.push_escaped_str(" --print-sysroot");
    #[cfg(unix)]
    {
        args.push_escaped_str(" >");
        args.push_escaped_str(" ");
        args.push_escaped_argument(&Argument::new_path(&output_path));
    }
    #[cfg(not(unix))]
    todo!("implement miri setup files");
    build.add_variable(args);

    file.add_build(build);
    output_path
}

/// Adds [`Build`] statements to build all [`Library`][l]s for execution under `miri`.
///
/// [l]: crate::config::subproject::Library
fn add_miri_builds(file: &mut NinjaFile, config: &Config, miri_sysroot: &Path) {
    let dummy = LibraryTarget::new();
    let target = Target::Build;
    for subproject in &config.subprojects {
        let Some(library) = subproject.library() else {
            continue;
        };
        let library_target = library.library_target(target).unwrap_or(&dummy);

        let mut artifacts_path = config.cli.build_dir_path.join(target.folder());
        artifacts_path.push("miri");

        let crate_name = convert_name_to_rust_name(subproject.name());
        let output_artifact_path = artifacts_path.join(format!("lib{crate_name}.rlib"));

        let mut build = Build::new(target.rustc_rule());
        build.add_output(FilePath::from_path(&output_artifact_path));
        build.add_input(FilePath::from_path(library.root_module()));
        build.add_implicit_input(FilePath::from_path(miri_sysroot));

        let mut driver = Variable::new("driver");
        driver.push_literal_str("$rustc");
        build.add_variable(driver);

        let mut crate_name_var = Variable::new("crate_name");
        crate_name_var.push_escaped_str(&crate_name);
        build.add_variable(crate_name_var);

        let mut crate_type = Variable::new("crate_type");
        crate_type.push_escaped_str("lib");
        build.add_variable(crate_type);

        let mut extern_var = Variable::new("extern");
        for dep in library.dependencies().chain(library_target.dependencies()) {
            let dep_name = convert_name_to_rust_name(dep);
            let dep_artifact_path = artifacts_path.join(format!("lib{dep_name}.rlib"));

            extern_var.push_escaped_str(" --extern");
            extern_var.push_escaped_str(" ");
            extern_var.push_escaped_str(&dep_name);
            extern_var.push_literal_str("=");
            extern_var.push_escaped_argument(&Argument::new_path(&dep_artifact_path));

            build.add_implicit_input(FilePath::from_path(dep_artifact_path));
        }

        if !extern_var.is_empty() {
            build.add_variable(extern_var);
        }

        let mut extra_args = Variable::new("extra_args");
        extra_args.push_escaped_str("-C opt-level=0");
        extra_args.push_escaped_str(" --sysroot");
        #[cfg(unix)]
        {
            extra_args.push_escaped_str(" $(cat");
            extra_args.push_escaped_str(" ");
            extra_args.push_escaped_path(miri_sysroot);
            extra_args.push_literal_str(")");
        }
        #[cfg(not(unix))]
        todo!();
        build.add_variable(extra_args);

        let mut search_dir = Variable::new("search_dir");
        search_dir.push_escaped_argument(&Argument::new_path(artifacts_path));
        build.add_variable(search_dir);

        let mut in_file = Variable::new("in_file");
        in_file.push_escaped_argument(&Argument::new_path(library.root_module()));
        build.add_variable(in_file);

        let mut out_file = Variable::new("out_file");
        out_file.push_escaped_argument(&Argument::new_path(&output_artifact_path));
        build.add_variable(out_file);

        file.add_build(build);
    }
}

/// Adds [`Build`] statements to build and execute all unit tests.
fn add_unit_test_builds(file: &mut NinjaFile, config: &Config) {
    let mut execute_unit_tests = Build::new("phony");
    execute_unit_tests.add_output(FilePath::from_literal("execute-unit-tests"));

    let dummy_library = LibraryTarget::new();
    let dummy_binary = BinaryTarget::new();
    let target = Target::Build;
    for subproject in &config.subprojects {
        if !subproject.is_workspace_member() {
            continue;
        }

        let library_opt = subproject.library().map(|library| {
            let library_target = library.library_target(target).unwrap_or(&dummy_library);

            (
                true,
                subproject.name(),
                library.root_module(),
                library.dependencies().chain(library_target.dependencies()),
            )
        });

        let binary_iter = subproject.binaries().iter().map(|binary| {
            let binary_target = binary.binary_target(target).unwrap_or(&dummy_binary);

            (
                false,
                binary.name(),
                binary.root_module(),
                binary.dependencies().chain(binary_target.dependencies()),
            )
        });

        let mut artifacts_path = config.cli.build_dir_path.join(target.folder());
        artifacts_path.push("rustc");

        let mut execute_subproject_unit_tests = Build::new("phony");
        let mut execute_subproject_unit_tests_path = FilePath::from_literal("execute-unit-tests-");
        execute_subproject_unit_tests_path.push_escaped(subproject.name());
        execute_subproject_unit_tests.add_output(execute_subproject_unit_tests_path.clone());

        let iter = library_opt.into_iter().chain(binary_iter);
        for (is_library, name, root_module, dependencies) in iter {
            let crate_name = convert_name_to_rust_name(name);
            let file_name = if is_library {
                format!("unit-tests-{name}-lib")
            } else {
                format!("unit-tests-{}-bin-{name}", subproject.name())
            };
            let output_artifact_path = artifacts_path.join(&file_name);

            let mut build = Build::new(target.rustc_rule());
            build.add_output(FilePath::from_path(&output_artifact_path));
            build.add_input(FilePath::from_path(root_module));

            let mut driver = Variable::new("driver");
            driver.push_literal_str("$rustc");
            build.add_variable(driver);

            let mut crate_name_var = Variable::new("crate_name");
            crate_name_var.push_escaped_str(&crate_name);
            build.add_variable(crate_name_var);

            let mut crate_type = Variable::new("crate_type");
            crate_type.push_escaped_str("lib");
            build.add_variable(crate_type);

            let mut extern_var = Variable::new("extern");

            let library_opt = if !is_library && subproject.library().is_some() {
                Some(subproject.name())
            } else {
                None
            };

            for dep in library_opt.into_iter().chain(dependencies) {
                let dep_name = convert_name_to_rust_name(dep);
                let dep_artifact_path = artifacts_path.join(format!("lib{dep_name}.rlib"));

                extern_var.push_escaped_str(" --extern");
                extern_var.push_escaped_str(" ");
                extern_var.push_escaped_str(&dep_name);
                extern_var.push_literal_str("=");
                extern_var.push_escaped_argument(&Argument::new_path(&dep_artifact_path));

                build.add_implicit_input(FilePath::from_path(dep_artifact_path));
            }

            if !extern_var.is_empty() {
                build.add_variable(extern_var);
            }

            let mut extra_args = Variable::new("extra_args");
            extra_args.push_escaped_str("--test");
            build.add_variable(extra_args);

            let mut search_dir = Variable::new("search_dir");
            search_dir.push_escaped_argument(&Argument::new_path(&artifacts_path));
            build.add_variable(search_dir);

            let mut in_file = Variable::new("in_file");
            in_file.push_escaped_argument(&Argument::new_path(root_module));
            build.add_variable(in_file);

            let mut out_file = Variable::new("out_file");
            out_file.push_escaped_argument(&Argument::new_path(&output_artifact_path));
            build.add_variable(out_file);

            let mut execute = Build::new("execute");
            execute.add_implicit_input(FilePath::from_path(&output_artifact_path));

            let mut unit_test_run_path = FilePath::from_literal("execute-");
            unit_test_run_path.push_literal(&file_name);
            execute.add_output(unit_test_run_path.clone());

            let mut binary = Variable::new("binary");
            binary.push_escaped_argument(&Argument::new_path(&output_artifact_path));
            execute.add_variable(binary);

            let mut args = Variable::new("args");
            args.push_escaped_str(" --color always");
            execute.add_variable(args);

            file.add_build(execute);
            file.add_build(build);
            execute_subproject_unit_tests.add_input(unit_test_run_path);
        }

        file.add_build(execute_subproject_unit_tests);
        execute_unit_tests.add_input(execute_subproject_unit_tests_path);
    }

    file.add_build(execute_unit_tests);
}

/// Adds [`Build`] statements to execute all unit tests utilizing `miri`.
fn add_miri_unit_test_builds(file: &mut NinjaFile, config: &Config, miri_sysroot: &Path) {
    let mut execute_unit_tests = Build::new("phony");
    execute_unit_tests.add_output(FilePath::from_literal("execute-miri-unit-tests"));

    let dummy_library = LibraryTarget::new();
    let dummy_binary = BinaryTarget::new();
    let target = Target::Build;
    for subproject in &config.subprojects {
        if !subproject.is_workspace_member() {
            continue;
        }

        let library_opt = subproject.library().map(|library| {
            let library_target = library.library_target(target).unwrap_or(&dummy_library);

            (
                true,
                subproject.name(),
                library.root_module(),
                library.dependencies().chain(library_target.dependencies()),
            )
        });

        let binary_iter = subproject.binaries().iter().map(|binary| {
            let binary_target = binary.binary_target(target).unwrap_or(&dummy_binary);

            (
                false,
                binary.name(),
                binary.root_module(),
                binary.dependencies().chain(binary_target.dependencies()),
            )
        });

        let mut artifacts_path = config.cli.build_dir_path.join(target.folder());
        artifacts_path.push("miri");

        let mut execute_subproject_unit_tests = Build::new("phony");
        let mut execute_subproject_unit_tests_path =
            FilePath::from_literal("execute-miri-unit-tests-");
        execute_subproject_unit_tests_path.push_escaped(subproject.name());
        execute_subproject_unit_tests.add_output(execute_subproject_unit_tests_path.clone());

        let iter = library_opt.into_iter().chain(binary_iter);
        for (is_library, name, root_module, dependencies) in iter {
            let crate_name = convert_name_to_rust_name(name);
            let file_name = if is_library {
                format!("execute-miri-unit-tests-{name}-lib")
            } else {
                format!("execute-miri-unit-tests-{}-bin-{name}", subproject.name())
            };
            let output_artifact_path = artifacts_path.join(&file_name);

            let mut build = Build::new(target.rustc_rule());
            build.add_output(FilePath::from_literal(&file_name));
            build.add_input(FilePath::from_path(root_module));
            build.add_implicit_input(FilePath::from_path(miri_sysroot));

            let mut driver = Variable::new("driver");
            driver.push_literal_str("$miri");
            build.add_variable(driver);

            let mut crate_name_var = Variable::new("crate_name");
            crate_name_var.push_escaped_str(&crate_name);
            build.add_variable(crate_name_var);

            let mut crate_type = Variable::new("crate_type");
            crate_type.push_escaped_str("lib");
            build.add_variable(crate_type);

            let mut extern_var = Variable::new("extern");

            let library_opt = if !is_library && subproject.library().is_some() {
                Some(subproject.name())
            } else {
                None
            };

            for dep in library_opt.into_iter().chain(dependencies) {
                let dep_name = convert_name_to_rust_name(dep);
                let dep_artifact_path = artifacts_path.join(format!("lib{dep_name}.rlib"));

                extern_var.push_escaped_str(" --extern");
                extern_var.push_escaped_str(" ");
                extern_var.push_escaped_str(&dep_name);
                extern_var.push_literal_str("=");
                extern_var.push_escaped_argument(&Argument::new_path(&dep_artifact_path));

                build.add_implicit_input(FilePath::from_path(dep_artifact_path));
            }

            if !extern_var.is_empty() {
                build.add_variable(extern_var);
            }

            let mut extra_args = Variable::new("extra_args");
            extra_args.push_escaped_str("-C opt-level=0");
            extra_args.push_escaped_str(" --sysroot");
            #[cfg(unix)]
            {
                extra_args.push_escaped_str(" $(cat");
                extra_args.push_escaped_str(" ");
                extra_args.push_escaped_path(miri_sysroot);
                extra_args.push_literal_str(")");
            }
            #[cfg(not(unix))]
            todo!();
            extra_args.push_escaped_str(" --test");
            build.add_variable(extra_args);

            let mut search_dir = Variable::new("search_dir");
            search_dir.push_escaped_argument(&Argument::new_path(&artifacts_path));
            build.add_variable(search_dir);

            let mut in_file = Variable::new("in_file");
            in_file.push_escaped_argument(&Argument::new_path(root_module));
            build.add_variable(in_file);

            let mut out_file = Variable::new("out_file");
            out_file.push_escaped_argument(&Argument::new_path(&output_artifact_path));
            build.add_variable(out_file);

            file.add_build(build);
            execute_subproject_unit_tests.add_input(FilePath::from_path(&file_name));
        }

        file.add_build(execute_subproject_unit_tests);
        execute_unit_tests.add_input(execute_subproject_unit_tests_path);
    }

    file.add_build(execute_unit_tests);
}

/// Adds a [`Build`] statement that regenerates the `build.ninja` file to mitigate out of date
/// `build.ninja` files.
fn add_regenerate_build(file: &mut NinjaFile, config: &Config) {
    let mut regenerate = Build::new("execute");
    regenerate.add_output(FilePath::from_path(&config.cli.build_ninja_path));
    regenerate.add_output(FilePath::from_path(&config.cli.rust_project_path));

    let mut generate_build_files_path = config.cli.build_dir_path.join("tools");
    generate_build_files_path.push("generate-build-files");

    regenerate.add_implicit_input(FilePath::from_path(&generate_build_files_path));
    regenerate.add_implicit_input(FilePath::from_path(&config.cli.config_path));

    let mut binary_var = Variable::new("binary");
    binary_var.push_escaped_argument(&Argument::new_path(&generate_build_files_path));
    regenerate.add_variable(binary_var);

    let mut generator_var = Variable::new("generator");
    generator_var.push_literal_str("1");
    regenerate.add_variable(generator_var);

    file.add_build(regenerate);
}
