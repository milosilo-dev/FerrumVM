{
  description = "Ferrum vm development environment";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs = { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in {
      devShells.${system}.default = pkgs.mkShell {
        packages = with pkgs; [
          rustc
          cargo
          gcc
          pkg-config
          pkgsCross.i686-embedded.buildPackages.gcc
          nasm
          fakeroot
          parted
          pkgsCross.mingwW64.stdenv.cc
        ];

        shellHook = ''
            mkdir -p .dev-bin
            ln -sf "$(command -v gcc)" .dev-bin/x86_64-linux-gnu-gcc
            export PATH="$PWD/.dev-bin:$PATH"
        '';
      };
    };
}