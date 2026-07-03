{
  description = "FerrumVM - A KVM-based virtual machine monitor";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
    let
      pkgs = import nixpkgs { inherit system; };

      # Cross-compiler for the 32-bit (i686) freestanding firmware
      crossPkgs = pkgs.pkgsCross.i686-embedded.buildPackages;

      # Tool paths injected into build.rs via env vars
      tools = {
        FERRUM_ASM = "${pkgs.nasm}/bin/nasm";
        FERRUM_ASL = "${pkgs.acpica-tools}/bin/iasl";
        FERRUM_CC32 = "${crossPkgs.gcc}/bin/gcc";
        FERRUM_CC64 = "${pkgs.gcc}/bin/gcc";
        FERRUM_LD = "${pkgs.binutils-unwrapped}/bin/ld";
        FERRUM_OBJCOPY = "${pkgs.binutils-unwrapped}/bin/objcopy";
      };

      ferrumvm = pkgs.rustPlatform.buildRustPackage {
        pname = "ferrumvm";
        version = "0.1.0";
        src = ./.;
        cargoLock.lockFile = ./Cargo.lock;

        nativeBuildInputs = with pkgs; [
          nasm
          acpica-tools
          gcc
          binutils-unwrapped
          crossPkgs.gcc
          crossPkgs.binutils-unwrapped
        ];

        inherit (tools)
          FERRUM_ASM FERRUM_ASL FERRUM_CC32 FERRUM_CC64 FERRUM_LD FERRUM_OBJCOPY;

        meta.mainProgram = "ferrumvm";
      };

      ferrum-driver = pkgs.stdenv.mkDerivation {
        name = "ferrum-driver";
        src = ./guest/driver;
        buildInputs = [ pkgs.linuxPackages.kernel.dev ];
        buildPhase = ''
          make -C "${pkgs.linuxPackages.kernel.dev}/lib/modules/${pkgs.linuxPackages.kernel.modDirVersion}/build" M=$PWD modules
        '';
        installPhase = ''
          mkdir -p $out/lib/modules/${pkgs.linuxPackages.kernel.modDirVersion}/misc
          cp driver.ko $out/lib/modules/${pkgs.linuxPackages.kernel.modDirVersion}/misc/
        '';
      };
    in
    {
      packages = {
        inherit ferrumvm ferrum-driver;
        default = ferrumvm;
      };

      devShells.default = pkgs.mkShell {
        inputsFrom = [ ferrumvm ];
        packages = with pkgs; [
          linuxPackages.kernel.dev
          rustc
          cargo
        ];
        inherit (tools)
          FERRUM_ASM FERRUM_ASL FERRUM_CC32 FERRUM_CC64 FERRUM_LD FERRUM_OBJCOPY;
        shellHook = ''
          echo "FerrumVM development shell"
          echo "  tools:  nasm, iasl, gcc (native + i686-elf cross)"
          echo "  kernel: ${pkgs.linuxPackages.kernel.modDirVersion}"
          echo "  env:    FERRUM_ASM FERRUM_ASL FERRUM_CC32 FERRUM_CC64 FERRUM_LD FERRUM_OBJCOPY"
        '';
      };

      apps.default = {
        type = "app";
        program = "${ferrumvm}/bin/ferrumvm";
      };
    });
}
