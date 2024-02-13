{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs";
    flake-utils.url = "github:numtide/flake-utils";
  };
  outputs = {
    self,
    nixpkgs,
    flake-utils,
  }: let
    pkgs = import nixpkgs {system = "x86_64-linux";};
  in
    flake-utils.lib.eachDefaultSystem
    (system: let
      pkgs = import nixpkgs {inherit system;};
    in {
      devShells.default = pkgs.mkShell {
        buildInputs = with pkgs; [
          rustc
          cargo
          cargo-tauri
          rustfmt
          rust-analyzer
          pkg-config
          openssl_3
        ];
      };
    });
}
