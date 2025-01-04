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
          crank = naersk-lib.buildPackage {
            src = pkgs.fetchFromGitHub {
              owner = "pd-rs";
              repo = "crank";
              rev = "main";
              sha256 = "sha256-Le/jW8Ej2qouZ0+8AShbNXyZJBvb/I0H1o4Z+5fv7G8=";
            };
          };

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
            buildInputs = [
              SDL2
              playdate-sdk.packages.x86_64-linux.default
              self.packages."${pkgs.system}".crank
              gcc-arm-embedded-13
            ];
            RUST_SRC_PATH = rustPlatform.rustLibSrc;
            PLAYDATE_SDK_PATH = "${playdate-sdk.packages.x86_64-linux.default}";
          };
      }
    );
}
