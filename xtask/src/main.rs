//! Automation for analyzing, building, formatting, packaging, and testing `revm` and associated
//! executables and other assets.

use anyhow::Result;

use crate::{
    action::{build_revm::build_revm, build_stub::build_revm_stub},
    cli::Action,
};

pub mod action;
pub mod cli;
pub mod common;

fn main() -> Result<()> {
    match cli::get_action() {
        Action::BuildStub(config) => {
            let path = build_revm_stub(config)?;
            println!("revm_stub located at \"{}\"", path.display());
        }
        Action::BuildRevm(config) => {
            let path = build_revm(config)?;
            println!("revm located at \"{}\"", path.display());
        }
    }

    Ok(())
}
