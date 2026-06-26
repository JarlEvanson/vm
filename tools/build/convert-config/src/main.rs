use std::{
    fmt::{self, Write},
    process::ExitCode,
};

fn main() -> ExitCode {
    let mut args = std::env::args();

    // Skip executable name.
    let executable = args.next();
    let Some(target) = args.next() else {
        let executable = executable.unwrap_or_else(|| String::from("convert-config"));
        eprintln!("Usage: {executable} <cfg | mk> <CONFIG_PATH>");
        return ExitCode::FAILURE;
    };

    if target != "cfg" && target != "mk" {
        let executable = executable.unwrap_or_else(|| String::from("convert-config"));
        eprintln!("Usage: {executable} <cfg | mk> <CONFIG_PATH>");
        return ExitCode::FAILURE;
    }

    let Some(mut config_file_path) = args.next() else {
        let executable = executable.unwrap_or_else(|| String::from("convert-config"));
        eprintln!("Usage: {executable} <CFG | MK> <CONFIG_PATH>");
        return ExitCode::FAILURE;
    };

    if args.next().is_some() {
        eprintln!("Extraneous arguments provided: exiting");
        return ExitCode::FAILURE;
    }

    let config = match std::fs::read_to_string(&config_file_path) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("error opening {config_file_path}: {error}");
            return ExitCode::FAILURE;
        }
    };

    let mut output = String::new();
    match generate_makefile(&mut output, &config, target == "cfg") {
        Ok(()) => {}
        Err(error) => {
            eprintln!("error converting {config_file_path}: {error}");
            return ExitCode::FAILURE;
        }
    }

    let extension = if target == "cfg" { ".cfg" } else { ".mk" };

    config_file_path.push_str(extension);
    match std::fs::write(&config_file_path, output) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error writing `{config_file_path}': {error}");
            ExitCode::FAILURE
        }
    }
}

fn generate_makefile(output: &mut String, config: &str, cfg: bool) -> fmt::Result {
    for line in config.lines() {
        let (config, comment) = line
            .split_once('#')
            .map(|(config, comment)| (config, Some(comment)))
            .unwrap_or_else(|| (line, None));

        let Some((key, value)) = config.split_once('=') else {
            if !cfg {
                write!(output, "{config}")?;
                if let Some(comment) = comment {
                    write!(output, "#{comment}")?;
                }

                writeln!(output)?;
            }
            continue;
        };

        if cfg {
            let key = key.trim();
            let value = value.trim();

            writeln!(output, "--cfg")?;
            write!(output, "{key}=")?;
            if !value.starts_with('\"') || !value.ends_with('\"') {
                writeln!(output, "\"{value}\"")?;
            } else {
                writeln!(output, "{value}")?;
            }
        } else {
            write!(output, "{key} ::= {value}")?;
            if let Some(comment) = comment {
                write!(output, "#{comment}")?;
            }

            writeln!(output)?;
        }
    }

    Ok(())
}
