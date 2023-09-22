{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.05";
    nixpkgs-mozilla.url = "github:mozilla/nixpkgs-mozilla/master";
  };

  outputs = { nixpkgs, nixpkgs-mozilla, ... }:
    let
      inherit (nixpkgs) lib;

      makePackage = (system: dev:
        let
          pkgs = import nixpkgs {
            inherit system;
            overlays = [ nixpkgs-mozilla.overlays.rust ];
          };

          rustPlatform =
            let
              rust = (pkgs.rustChannelOf {
                channel = "1.72.1";
                sha256 = "sha256-dxE7lmCFWlq0nl/wKcmYvpP9zqQbBitAQgZ1zx9Ooik=";
              }).rust.override {
                extensions = if dev then [ "rust-src" ] else [ ];
              };
            in
            pkgs.makeRustPlatform {
              cargo = rust;
              rustc = rust;
            };
        in
        rustPlatform.buildRustPackage {
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

          cargoLock = {
            lockFile = ./Cargo.lock;
            outputHashes = {
              "async-dispatcher-0.1.0" = "sha256-rqpQ176/PnI9vvPrwQvK3GJbryjb3hHkb+o1RyCZ3Vg=";
              "classicube-helpers-2.0.0+classicube.1.3.6" = "sha256-yUl0B0E8P618S0662u70zUGRAG2bETVmb4G7Tbv+ZP4=";
              "classicube-sys-3.0.0+classicube.1.3.6" = "sha256-algb9pgkJdXaswcB6m8DITzORGtOQkSgkhVvwgNXAhI=";
            };
          };

          nativeBuildInputs = with pkgs; [
            pkg-config
            rustPlatform.bindgenHook
          ];

          buildInputs = with pkgs; [
            openssl
          ];

          doCheck = false;
        }
      );
    in
    builtins.foldl' lib.recursiveUpdate { } (builtins.map
      (system: {
        devShells.${system}.default = makePackage system true;
        packages.${system}.default = makePackage system false;
      })
      lib.systems.flakeExposed);
}
