{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.11";
  };

  outputs = { nixpkgs, ... }:
    let
      inherit (nixpkgs) lib;

      makePackages = (system: dev:
        let
          pkgs = import nixpkgs {
            inherit system;
          };
          inherit (lib.importTOML ./Cargo.toml) package;
        in
        {
          default = pkgs.rustPlatform.buildRustPackage {
            pname = package.name;
            version = package.version;

            src = lib.sourceByRegex ./. [
              "^\.cargo(/.*)?$"
              "^Cargo\.(lock|toml)$"
              "^src(/.*)?$"
            ];

            cargoLock = {
              lockFile = ./Cargo.lock;
              outputHashes = {
                "async-dispatcher-0.1.0" = "sha256-rqpQ176/PnI9vvPrwQvK3GJbryjb3hHkb+o1RyCZ3Vg=";
                "classicube-helpers-2.0.0+classicube.1.3.6" = "sha256-V5PBZR0rj42crA1fGUjMk4rDh0ZpjjNcbMCe6bgotW8=";
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

          update-cef-version = pkgs.writeShellApplication {
            name = "update-cef-version";
            runtimeInputs = with pkgs; [
              coreutils
              sd
              ripgrep
              zx
            ];
            text = ''
              if ! NEW_VERSION="$(zx .github/check-cef-version.mjs)"; then
                if test -n "$NEW_VERSION"; then
                  echo "new CEF version: $NEW_VERSION"

                  REGEX='\d+\.\d+\.\d+\+\w+\+chromium-\d+\.\d+\.\d+\.\d+'
                  OLD_VERSION="$(rg -o "$REGEX" src/cef_binary_updater.rs | head -n1)"

                  echo "$OLD_VERSION" "$NEW_VERSION"
                  test -z "$OLD_VERSION" && exit 1
                  test -z "$NEW_VERSION" && exit 1
                  test "$OLD_VERSION" = "$NEW_VERSION" && exit 0

                  sd --fixed-strings "$OLD_VERSION" "$NEW_VERSION" src/cef_binary_updater.rs
                fi
                exit 1
              fi
            '';
          };
        }
      );
    in
    builtins.foldl' lib.recursiveUpdate { } (builtins.map
      (system: {
        devShells.${system} = makePackages system true;
        packages.${system} = makePackages system false;
      })
      lib.systems.flakeExposed);
}
