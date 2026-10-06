use crate::config::Config;

#[path = "generate-build-files/configure.rs"]
mod generate_build_files;

pub fn configure(config: &mut Config) {
    generate_build_files::configure(config)
}
