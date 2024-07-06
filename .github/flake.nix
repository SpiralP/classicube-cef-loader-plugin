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
        in
        rec {
          update-cef-version = pkgs.writeShellApplication {
            name = "update-cef-version";
            runtimeInputs = with pkgs; [
              coreutils
            ];
            text = ''
              LATEST_VERSION="$(${lib.getExe get-latest-cef-version})"
              CURRENT_VERSION="$(${lib.getExe get-current-cef-version})"

              test -z "$LATEST_VERSION" && exit 1
              test -z "$CURRENT_VERSION" && exit 1

              if test "$LATEST_VERSION" != "$CURRENT_VERSION"; then
                echo "new CEF version: $LATEST_VERSION"
                ${lib.getExe replace-cef-version} "$LATEST_VERSION"
              fi
            '';
          };

          get-latest-cef-version = pkgs.writeShellApplication {
            name = "get-latest-cef-version";
            runtimeInputs = with pkgs; [
              coreutils
              zx
            ];
            text = ''
              LATEST_VERSION="$(zx ${./get-latest-cef-version.mjs})"
              test -z "$LATEST_VERSION" && exit 1
              echo "$LATEST_VERSION"
            '';
          };

          get-current-cef-version = pkgs.writeShellApplication {
            name = "get-current-cef-version";
            runtimeInputs = with pkgs; [
              coreutils
              ripgrep
            ];
            text = ''
              REGEX='\d+\.\d+\.\d+\+\w+\+chromium-\d+\.\d+\.\d+\.\d+'
              CURRENT_VERSION="$(rg -o "$REGEX" src/cef_binary_updater.rs | head -n1)"
              test -z "$CURRENT_VERSION" && exit 1
              echo "$CURRENT_VERSION"
            '';
          };

          replace-cef-version = pkgs.writeShellApplication {
            name = "replace-cef-version";
            runtimeInputs = with pkgs; [
              coreutils
              gnugrep
              sd
            ];
            text = ''
              NEW_VERSION="$1"
              OLD_VERSION="$(${lib.getExe get-current-cef-version})"

              echo "$OLD_VERSION" "$NEW_VERSION"
              test -z "$OLD_VERSION" && exit 1
              test -z "$NEW_VERSION" && exit 1
              test "$OLD_VERSION" = "$NEW_VERSION" && exit 0

              if ! grep -q "$OLD_VERSION" src/cef_binary_updater.rs; then
                echo "couldn't find old version in src/cef_binary_updater.rs"
                exit 1
              fi
              sd --fixed-strings "$OLD_VERSION" "$NEW_VERSION" src/cef_binary_updater.rs
              if ! grep -q "$NEW_VERSION" src/cef_binary_updater.rs; then
                echo "couldn't find new version in src/cef_binary_updater.rs"
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
