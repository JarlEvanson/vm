set -ex

sh build-all.sh

cargo xtask run \
    --arch x86_32 \
    --run-dir run/x86_32-x86_32 \
    --package-path target/revm-x86_32-x86_32.efi

cargo xtask run \
    --arch x86_32 \
    --run-dir run/x86_32-x86_64 \
    --package-path target/revm-x86_32-x86_64.efi

cargo xtask run \
    --arch x86_64 \
    --run-dir run/x86_64-x86_32 \
    --package-path target/revm-x86_64-x86_32.efi

cargo xtask run \
    --arch x86_64 \
    --run-dir run/x86_64-x86_64 \
    --package-path target/revm-x86_64-x86_64.efi
