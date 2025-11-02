{
  description = "Nix Docker Layered Image test.";

  # Nixpkgs / NixOS version to use.
  inputs = {
    nixpkgs.url = "nixpkgs/nixos-unstable";
    naersk = {
      url = "github:nix-community/naersk";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, naersk, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
        let
          pkgs = (import nixpkgs) {
            inherit system;
          };
          pkgName = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.name;
          naersk' = pkgs.callPackage naersk {};
          rustApp =  naersk'.buildPackage {
            src = ./.;
          };
        in {
          packages = {
            default = rustApp;
            docker = pkgs.dockerTools.streamLayeredImage {
              name = "localhost/${pkgName}";
              tag = "latest";
              contents = [ rustApp ];
              config.Cmd = "${rustApp}/bin/${pkgName}";
            };
          };

          # For `nix develop` (optional, can be skipped):
          devShell = pkgs.mkShell {
            nativeBuildInputs = with pkgs; [ rustc cargo jujutsu ];
          };
        }
      );
}

