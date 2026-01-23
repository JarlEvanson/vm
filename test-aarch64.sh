#!/usr/bin/env sh
set -euo pipefail

usage() {
    cat <<EOF
Usage: $0 [OPTIONS]

Options:
  --ovmf-dir <path>     Directory containing code.fd and vars.fd
  --limine-dir <path>   Directory containing BOOTAA64.EFI
  -h, --help            Show this help message

Environment variables:
  OVMF_DIR
  LIMINE_DIR

CLI options take precedence over environment variables.
EOF
}

OVMF_DIR="${OVMF_DIR:-}"
LIMINE_DIR="${LIMINE_DIR:-}"

while [ "$#" -gt 0 ]; do
    case "$1" in
        --ovmf-dir)
            OVMF_DIR="$2"
            shift 2
            ;;
        --limine-dir)
            LIMINE_DIR="$2"
            shift 2
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

: "${OVMF_DIR:?OVMF directory not specified (use --ovmf-dir or OVMF_DIR)}"
: "${LIMINE_DIR:?Limine EFI directory not specified (use --limine-dir or LIMINE_DIR)}"

OVMF_CODE="${OVMF_DIR}/aarch64/code.fd"
OVMF_VARS="${OVMF_DIR}/aarch64/vars.fd"
LIMINE_EFI="${LIMINE_DIR}/BOOTAA64.EFI"

if [ ! -f "${OVMF_CODE}" ]; then
    echo "Missing OVMF code file: ${OVMF_CODE}" >&2
    exit 1
fi

if [ ! -f "${OVMF_VARS}" ]; then
    echo "Missing OVMF vars file: ${OVMF_VARS}" >&2
    exit 1
fi

if [ ! -f "${LIMINE_EFI}" ]; then
    echo "Missing Limine EFI binary: ${LIMINE_EFI}" >&2
    exit 1
fi

clear
cargo fmt

RUN_DIR="run/aarch64"
FAT_DIR="${RUN_DIR}/fat"
EFI_BOOT_DIR="${FAT_DIR}/EFI/BOOT"

mkdir -p "${EFI_BOOT_DIR}"

cp "${OVMF_CODE}" "${OVMF_VARS}" "${RUN_DIR}/"
cp "${LIMINE_EFI}" "${EFI_BOOT_DIR}/BOOTAA64.EFI"

chmod 666 "${RUN_DIR}/vars.fd"
chmod 666 "${RUN_DIR}/code.fd"
chmod 666 "${EFI_BOOT_DIR}/BOOTAA64.EFI"

cat <<EOF > "${FAT_DIR}/limine.conf"
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
kaslr: yes
EOF

cargo xtask package \
    --arch aarch64 \
    --profile dev \
    --output-path "${FAT_DIR}/revm.efi"

qemu-system-aarch64 \
    -machine virt -cpu a64fx -m 512M \
    -drive if=pflash,format=raw,file="${RUN_DIR}/code.fd" \
    -drive if=pflash,format=raw,file="${RUN_DIR}/vars.fd" \
    -drive file=fat:rw:"${FAT_DIR}",format=raw \
    -device ramfb \
    -device qemu-xhci -device usb-kbd \
    -serial file:"${RUN_DIR}/serial.txt" \
    -D "${RUN_DIR}/qemu-log.txt" -d int -d cpu_reset \
    -s
