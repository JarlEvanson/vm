use crate::config::Config;

#[path = "elf/configure.rs"]
mod elf;

pub fn configure(config: &mut Config) {
    elf::configure(config);
}
