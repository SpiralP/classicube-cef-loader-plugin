{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-22.11";
    nixpkgs-mozilla.url = "github:mozilla/nixpkgs-mozilla/master";
  };

  outputs = { nixpkgs, nixpkgs-mozilla, ... }:
    let
      inherit (nixpkgs) lib;
    in
    builtins.foldl' lib.recursiveUpdate { } (builtins.map
      (system:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ nixpkgs-mozilla.overlays.rust ];
          };

          rustPlatform =
            let
              rust = (pkgs.rustChannelOf {
                date = "2023-04-23";
                channel = "nightly";
                sha256 = "sha256-f+dMK7oRvMx2VYzqJru4ElIngARn4d2q2GkAPdlZrW0=";
              }).rust.override {
                extensions = [ "rust-src" ];
              };
            in
            pkgs.makeRustPlatform {
              cargo = rust;
              rustc = rust;
            };

          package = rustPlatform.buildRustPackage {
            name = "classicube-cef-loader-plugin";
            src = lib.cleanSourceWith rec {
              src = ./.;
              filter = path: type:
                lib.cleanSourceFilter path type
                && (
                  let
                    baseName = builtins.baseNameOf (builtins.toString path);
                    relPath = lib.removePrefix (builtins.toString ./.) (builtins.toString path);
                  in
                  lib.any (re: builtins.match re relPath != null) [
                    "/Cargo.toml"
                    "/Cargo.lock"
                    "/\.cargo"
                    "/\.cargo/.*"
                    "/src"
                    "/src/.*"
                  ]
                );
            };
            cargoSha256 = "sha256-EQcmrwsrR1vOs7jn/Bea/zlT6poxTbbvNZgwFV+k064=";
            nativeBuildInputs = with pkgs; [
              pkg-config
              rustPlatform.bindgenHook
            ];
            buildInputs = with pkgs; [
              openssl
            ];

            doCheck = false;
          };
        in
        rec {
          devShells.${system}.default = package.overrideAttrs (old: {
            nativeBuildInputs = with pkgs; old.nativeBuildInputs ++ [
              clippy
              rustfmt
              rust-analyzer
            ];
          });
          packages.${system}.default = package;
        }
      )
      lib.systems.flakeExposed);
}
