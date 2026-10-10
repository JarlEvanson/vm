use crate::config::Config;

#[path = "elf/configure.rs"]
mod elf;
#[path = "pe/configure.rs"]
mod pe;

pub fn configure(config: &mut Config) {
    elf::configure(config);
    pe::configure(config);
}
