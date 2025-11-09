set -e

clear
cargo fmt

mkdir -p run/aarch64/fat/EFI/BOOT
cp \
    ~/projects/ovmf-binaries/aarch64/code.fd \
    ~/projects/ovmf-binaries/aarch64/vars.fd \
    run/aarch64/

cp ~/projects/limine-binaries/BOOTAA64.EFI run/aarch64/fat/EFI/BOOT/
cat <<EOF > run/aarch64/fat/limine.conf
serial: yes
verbose: yes
randomize_memory: yes

/linux
protocol: linux
path: boot():/revm.efi

/limine
protocol: limine
path: boot():/revm.efi
kaslr: yes

/efi
protocol: efi
path: boot():/revm.efi

EOF

cargo xtask package --arch aarch64 --profile dev --output-path run/aarch64/fat/revm.efi

qemu-system-aarch64 \
    -machine virt -cpu a64fx \
    -drive if=pflash,format=raw,file=run/aarch64/code.fd \
    -drive if=pflash,format=raw,file=run/aarch64/vars.fd \
    -drive file=fat:rw:run/aarch64/fat,format=raw \
    -device ramfb \
    -device qemu-xhci -device usb-kbd \
    -serial file:run/aarch64/serial.txt \
    -D run/aarch64/qemu-log.txt -d int -d cpu_reset \
    -s
