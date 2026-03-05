set -ex

cargo xtask build-stub --arch x86_32
cargo xtask build-stub --arch x86_64

cargo xtask build-revm --arch x86_32
cargo xtask build-revm --arch x86_64

cargo xtask package \
    --stub-path target/x86_32-unknown-none/debug/revm-stub \
    --revm-path target/x86_32-unknown-none/debug/revm \
    --output-path target/revm-x86_32-x86_32.efi

cargo xtask package \
    --stub-path target/x86_32-unknown-none/debug/revm-stub \
    --revm-path target/x86_64-unknown-none/debug/revm \
    --output-path target/revm-x86_32-x86_64.efi

cargo xtask package \
    --stub-path target/x86_64-unknown-none/debug/revm-stub \
    --revm-path target/x86_32-unknown-none/debug/revm \
    --output-path target/revm-x86_64-x86_32.efi

cargo xtask package \
    --stub-path target/x86_64-unknown-none/debug/revm-stub \
    --revm-path target/x86_64-unknown-none/debug/revm \
    --output-path target/revm-x86_64-x86_64.efi
