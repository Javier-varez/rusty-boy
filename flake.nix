{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    naersk.url = "github:nix-community/naersk/master";
    utils.url = "github:numtide/flake-utils";
    playdate-sdk.url = "github:RegularTetragon/playdate-sdk-flake";
  };

  outputs =
    {
      self,
      nixpkgs,
      naersk,
      utils,
      playdate-sdk,
    }:
    utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
        naersk-lib = pkgs.callPackage naersk { };
      in
      {
        packages = rec {
          rusty-boy = naersk-lib.buildPackage {
            src = ./.;
            root = ./.;
            buildInputs = [ pkgs.SDL2 ];
          };

          # TODO: build rusty-date as well
          default = rusty-boy;
        };
        devShell =
          with pkgs;
          mkShell {
            buildInputs =
              [
                pkg-config
                SDL2
                gcc-arm-embedded-13
                rgbds # For building game boy test games from source
                gnumake
              ]
              ++ (lib.optionals stdenv.isLinux [
                playdate-sdk.packages.x86_64-linux.default
              ]);
            RUST_SRC_PATH = rustPlatform.rustLibSrc;
          }
          // (lib.optionalAttrs stdenv.isLinux {
            PLAYDATE_SDK_PATH = "${playdate-sdk.packages.x86_64-linux.default}";
          });
      }
    );
}
