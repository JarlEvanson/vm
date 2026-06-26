use std::process::ExitCode;

fn main() -> ExitCode {
    let mut arguments = std::env::args();

    let executable_name = arguments
        .next()
        .unwrap_or_else(|| String::from("fix-dependencies"));
    if arguments.len() != 2 {
        eprintln!("Usage: {executable_name} <DEPENDENCY_FILE> <RLIB_PATH>");
        return ExitCode::FAILURE;
    }

    // Skip executable name.
    let dependency_file = arguments.next().unwrap();
    let rlib_path = arguments.next().unwrap();

    let Some((_, rlib_name)) = rlib_path.rsplit_once('/') else {
        eprintln!("<RLIB_PATH> must include folder");
        return ExitCode::FAILURE;
    };

    let input = match std::fs::read_to_string(&dependency_file) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("error reading {dependency_file}: {error}");
            return ExitCode::FAILURE;
        }
    };

    let mut output = String::new();

    let mut input = input.split('\n').peekable();
    while let Some(line) = input.next() {
        let Some((rule, deps)) = line.split_once(':') else {
            output.push_str(line);
            if input.peek().is_some() {
                output.push('\n');
            }
            continue;
        };

        if rule.trim() == rlib_name {
            output.push_str(&rule.replace(rlib_name, &rlib_path));
            output.push(':');
            output.push_str(deps);
        } else {
            output.push_str(line);
        }

        if input.peek().is_some() {
            output.push('\n');
        }
    }

    match std::fs::write(&dependency_file, output) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error writing {dependency_file}: {error}");
            ExitCode::FAILURE
        }
    }
}
