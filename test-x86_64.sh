set -e

clear
cargo fmt

mkdir -p run/x86_64/fat/EFI/BOOT
cp \
    ~/projects/ovmf-binaries/x64/code.fd \
    ~/projects/ovmf-binaries/x64/vars.fd \
    run/x86_64/

cp ~/projects/limine-binaries/BOOTX64.EFI run/x86_64/fat/EFI/BOOT/
cat <<EOF > run/x86_64/fat/limine.conf
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

EOF

cargo xtask package --arch x86_64 --profile dev --output-path run/x86_64/fat/revm.efi

qemu-system-x86_64 \
    -machine q35 -m 512M \
    -drive if=pflash,format=raw,file=run/x86_64/code.fd \
    -drive if=pflash,format=raw,file=run/x86_64/vars.fd \
    -drive file=fat:rw:run/x86_64/fat,format=raw \
    -debugcon file:run/x86_64/debugcon.txt \
    -serial file:run/x86_64/serial.txt \
    -D run/x86_64/qemu-log.txt -d int \
    -s
