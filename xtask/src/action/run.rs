//! Helper functions to run a packaged `revm` given a [`RunConfig`].

use std::fs;

use anyhow::{Context, Result};

use crate::{
    action::{self, run_cmd},
    cli::{
        self,
        run::{PackageConfig, RunConfig},
    },
    common::Arch,
};

/// Runs a packaged `revm` as specified by `config`.
///
/// # Errors
///
/// Returns errors when the `cargo build` command fails, an error in the packaging process occurs,
/// or an error occurs when running `revm` using `QEMU`.
pub fn run(config: RunConfig) -> Result<()> {
    let run_dir = config.run_dir;
    let fat_dir = run_dir.join("fat");
    let efi_boot_dir = fat_dir.join("EFI/BOOT/");

    fs::create_dir_all(&efi_boot_dir)?;

    let target_package_path = fat_dir.join("revm.efi");
    match config.package {
        PackageConfig::Path(path) => {
            let _ = fs::copy(path, target_package_path)?;
        }
        PackageConfig::Package { stub, revm } => {
            let package_config = cli::package::PackageConfig {
                stub,
                revm,
                output_path: target_package_path,
            };
            let _ = action::package::package(package_config)?;
        }
    };

    // Copy the Limine binary to the run directory.
    let limine_path_from = config.limine_dir.join(config.arch.as_limine_binary());
    let limine_path_to = efi_boot_dir.join(
        limine_path_from
            .file_name()
            .context("limine path has no file name")?,
    );
    let _ = fs::copy(&limine_path_from, &limine_path_to)?;

    let ovmf_path_from = config.ovmf_dir.join(config.arch.as_ovmf_folder());
    let ovmf_code_to = run_dir.join("code.fd");
    let ovmf_vars_to = run_dir.join("vars.fd");

    let _ = fs::copy(ovmf_path_from.join("code.fd"), &ovmf_code_to)?;
    let _ = fs::copy(ovmf_path_from.join("vars.fd"), &ovmf_vars_to)?;

    // Update permissions so that the copied files are readable. This prevents consecutive runs
    // from failing to copy files into place.
    for path in [&limine_path_to, &ovmf_code_to, &ovmf_vars_to] {
        let metadata = fs::metadata(path)?;
        let mut permisions = metadata.permissions();

        #[expect(clippy::permissions_set_readonly_false)]
        permisions.set_readonly(false);
        fs::set_permissions(path, permisions)?;
    }

    fs::write(
        fat_dir.join("limine.conf"),
        "\
serial: yes
verbose: yes

/efi
protocol: efi
path: boot():/revm.efi

/linux
protocol: linux
path: boot():/revm.efi

/limine
protocol: limine
path: boot():/revm.efi
    ",
    )?;

    let mut cmd = std::process::Command::new(config.arch.as_qemu_executable());

    match config.arch {
        Arch::Aarch64 => {
            cmd.args(["-machine", "virt"]);
            cmd.args(["-cpu", "a64fx"]);

            cmd.args(["-device", "ramfb"]);
            cmd.args(["-device", "qemu-xhci", "-device", "usb-kbd"]);
        }
        Arch::X86_32 | Arch::X86_64 => {
            cmd.args(["-machine", "q35"]);
            cmd.args(["-cpu", "max"]);
        }
    }

    // QEMU guest should have 512 MB of memory.
    cmd.args(["-m", "512M"]);

    // Assign OVMF firmware.
    cmd.arg("-drive").arg(format!(
        "if=pflash,format=raw,file={}",
        ovmf_code_to.display()
    ));
    cmd.arg("-drive").arg(format!(
        "if=pflash,format=raw,file={}",
        ovmf_vars_to.display()
    ));

    // Utilize `fat_dir` as a virtualized fat filesystem.
    cmd.arg("-drive")
        .arg(format!("format=raw,file=fat:rw:{}", fat_dir.display()));

    // Enable serial and qemu logging.
    cmd.arg("-serial")
        .arg(format!("file:{}/serial.txt", run_dir.display()));
    cmd.arg("-D")
        .arg(format!("{}/qemu-log.txt", run_dir.display()));

    // Log QEMU interrupts.
    cmd.args(["-d", "int"]);

    // Enable integrated GDB stub.
    cmd.arg("-s");

    run_cmd(cmd).map_err(|error| error.into())
}
