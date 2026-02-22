{
  description = "revm";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs }:
  let
    system = "x86_64-linux";
    pkgs = import nixpkgs { inherit system; };
  in rec {
    packages.${system} = {
      ovmf-latest = builtins.fetchTarball {
        url = https://github.com/rust-osdev/ovmf-prebuilt/releases/download/edk2-stable202402-r1/edk2-stable202402-r1-bin.tar.xz;
        sha256 = "19pcfp9wbcmn39qlg4f2i0nf02pkhk9vc796ivnnapkxkz5lyvjz";
      };

      limine-latest = lib.buildLimineFromTarball {
        version = "10.6.5";
        sha256 = "05p58m4jlw49lm8cpdxcwbvzdnwny8qc6b07ij43dqrsr320gy08";
      };
    };

    devShells.${system} = {
      default = pkgs.mkShell {
        packages = [];

        OVMF_DIR = "${packages.${system}.ovmf-latest}";
        LIMINE_DIR = "${packages.${system}.limine-latest}/share/limine";

        LIBCLANG_PATH = "${pkgs.llvmPackages.libclang.lib}/lib";
      };
    };
    
    lib = {
      buildLimine = { version, src }: pkgs.stdenv.mkDerivation rec {
        pname = "limine";
        inherit version;

        inherit src;

        nativeBuildInputs = [
          pkgs.clang
          pkgs.lld
          pkgs.llvm
          pkgs.nasm
        ];

        configureFlags = [
          "--enable-uefi-x86_64"
          "--enable-uefi-aarch64"
          "--enable-uefi-ia32"
        ];

        patches = [
          ./patches/limine-enable-linux-entry-64.patch
        ];
      };

      buildLimineFromTarball = { version, sha256 ? "" }: lib.buildLimine {
        inherit version;

        src = builtins.fetchTarball {
          url = "https://github.com/limine-bootloader/limine/releases/download/v${version}/limine-${version}.tar.gz";
          inherit sha256;
        };
      };
    };
  };
}
