{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-23.11";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay.url = "github:oxalica/rust-overlay";
  };

  outputs = {self, ...} @ inputs:
    inputs.flake-utils.lib.eachDefaultSystem (
      system: let
        overlays = [(import inputs.rust-overlay)];
        pkgs = import inputs.nixpkgs {
          inherit system overlays;
        };
        rust = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;

        nativeBuildInputs = with pkgs; [
          pkg-config
          openssl.dev
        ];

        PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
      in rec {
        packages = rec {
          # Build with `nix build`
          default = pdxindoorsoccer-ical;

          pdxindoorsoccer-ical = let
            cargoToml = builtins.fromTOML (builtins.readFile ./Cargo.toml);
          in
            # TODO: this doesn't match the developer cargo version
            pkgs.rustPlatform.buildRustPackage {
              pname = "pdxindoorsoccer-ical";
              version = cargoToml.package.version;

              inherit nativeBuildInputs;
              inherit PKG_CONFIG_PATH;

              src = ./.;
              release = true;

              cargoLock = {
                lockFile = ./Cargo.lock;
              };

              meta = with pkgs.lib; {
                description = "A small utility to create ical files from a webpage with a soccer schedule";
                homepage = "https://github.com/dlo9/pdxindoorsoccer-ical";
                license = cargoToml.package.license;
              };
            };
        };

        apps = rec {
          # Run with `nix run`
          apps.default = pdxindoorsoccer-ical;

          pdxindoorsoccer-ical = {
            type = "app";
            program = "${packages.default}/bin/pdxindoorsoccer-ical";
          };
        };

        # Enter with `nix develop`
        devShells.default = pkgs.mkShell {
          inherit nativeBuildInputs;
          inherit PKG_CONFIG_PATH;

          buildInputs = with pkgs; [
            rust
            cargo-deny
            codespell
          ];

          shellHook = ''
            # Setup git hooks
            ln -srf hooks/* .git/hooks/
          '';

          RUST_BACKTRACE = 1;
        };

        formatter = pkgs.alejandra;
      }
    );
}
