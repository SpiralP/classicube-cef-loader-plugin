{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-24.05";
  };

  outputs = { nixpkgs, ... }:
    let
      inherit (nixpkgs) lib;

      makePackages = (system: dev:
        let
          pkgs = import nixpkgs {
            inherit system;
          };
          rustManifest = lib.importTOML ./Cargo.toml;
        in
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = rustManifest.package.name;
            version = rustManifest.package.version;

            src = lib.sourceByRegex ./. [
              "^\.cargo(/.*)?$"
              "^build\.rs$"
              "^Cargo\.(lock|toml)$"
              "^src(/.*)?$"
            ];

            cargoLock = {
              lockFile = ./Cargo.lock;
              outputHashes = {
                "async-dispatcher-0.1.0" = "sha256-rqpQ176/PnI9vvPrwQvK3GJbryjb3hHkb+o1RyCZ3Vg=";
                "classicube-helpers-2.0.0+classicube.1.3.6" = "sha256-mGRzuvxKXKBxKgZuLkc7qzmABIj9uADuxAkahR8e840=";
              };
            };

            nativeBuildInputs = with pkgs; [
              pkg-config
              rustPlatform.bindgenHook
            ] ++ (if dev then
              with pkgs; [
                cargo-release
                clippy
                rustfmt
                rust-analyzer
              ] else [ ]);

            buildInputs = with pkgs; [
              openssl
            ];
          };
        }
      );
    in
    builtins.foldl' lib.recursiveUpdate { } (builtins.map
      (system: {
        devShells.${system} = makePackages system true;
        packages.${system} = makePackages system false;
      })
      [
        "x86_64-linux"
        "aarch64-linux"
        # cef "Linux ARM" is armv7
        "armv7l-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ]);
}
