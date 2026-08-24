use std::fmt::{self, Write};

use crate::{
    config::{Config, Target},
    convert_name_to_rust_name,
};

pub fn generate(output: &mut String, config: &Config) -> fmt::Result {
    writeln!(output, "{{")?;

    writeln!(output, "\t\"rustc\": {:?},", config.arguments.rustc)?;
    writeln!(output, "\t\"sysroot\": {:?},", config.rustc_sysroot)?;

    let mut rustc_sysroot_src = config.rustc_sysroot.join("lib");
    rustc_sysroot_src.push("rustlib");
    rustc_sysroot_src.push("src");
    rustc_sysroot_src.push("rust");
    rustc_sysroot_src.push("library");
    writeln!(output, "\t\"sysroot_src\": {:?},", rustc_sysroot_src)?;

    writeln!(output, "\t\"crates\": [")?;
    for (index, subproject_info) in config.subproject_info.iter().enumerate() {
        writeln!(output, "\t\t{{")?;

        let display_name = format!(
            "{} ({})",
            subproject_info.name,
            subproject_info.target.folder()
        );
        writeln!(output, "\t\t\t\"display_name\": {display_name:?},")?;
        writeln!(
            output,
            "\t\t\t\"root_module\": {:?},",
            subproject_info.root_module
        )?;
        writeln!(output, "\t\t\t\"edition\": \"2024\",")?;

        if subproject_info.libraries.is_empty() {
            writeln!(output, "\t\t\t\"deps\": [],")?;
        } else {
            writeln!(output, "\t\t\t\"deps\": [")?;
            for (index, dep) in subproject_info.libraries.iter().enumerate() {
                writeln!(output, "\t\t\t\t{{")?;

                writeln!(
                    output,
                    "\t\t\t\t\t\"name\": {:?},",
                    convert_name_to_rust_name(dep)
                )?;
                let crate_index = config.graph[&(dep.clone(), subproject_info.target)];
                writeln!(output, "\t\t\t\t\t\"crate\": {crate_index}")?;

                if index == subproject_info.libraries.len() - 1 {
                    writeln!(output, "\t\t\t\t}}")?;
                } else {
                    writeln!(output, "\t\t\t\t}},")?;
                }
            }

            writeln!(output, "\t\t\t],")?;
        }

        let cfgs = match subproject_info.target {
            Target::Build => &config.build_cfgs,
            Target::Host => &config.host_cfgs,
            Target::Revm => &config.revm_cfgs,
            Target::RevmStub => &config.revm_stub_cfgs,
        };

        writeln!(output, "\t\t\t\"cfg\": [")?;
        for (index, cfg) in cfgs.iter().enumerate() {
            if index == cfgs.len() - 1 {
                writeln!(output, "\t\t\t\t\"{}\"", cfg.escape_default())?;
            } else {
                writeln!(output, "\t\t\t\t\"{}\",", cfg.escape_default())?;
            }
        }

        if subproject_info.name == "configure" {
            writeln!(output, "\t\t\t],")?;

            writeln!(output, "\t\t\t\"source\": {{")?;

            writeln!(output, "\t\t\t\t\"include_dirs\": [")?;
            writeln!(output, "\t\t\t\t\t\".\"")?;
            writeln!(output, "\t\t\t\t],")?;

            writeln!(output, "\t\t\t\t\"exclude_dirs\": []")?;

            writeln!(output, "\t\t\t}}")?;
        } else {
            writeln!(output, "\t\t\t]")?;
        }

        if index == config.subproject_info.len() - 1 {
            writeln!(output, "\t\t}}")?;
        } else {
            writeln!(output, "\t\t}},")?;
        }
    }
    writeln!(output, "\t]")?;

    writeln!(output, "}}")
}
