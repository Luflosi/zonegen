# SPDX-FileCopyrightText: 2024 Luflosi <zonegen@luflosi.de>
# SPDX-License-Identifier: GPL-3.0-only

{
  description = "Build zonegen";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane.url = "github:ipetkov/crane";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };

    dyndnsd = {
      url = "github:Luflosi/dyndnsd";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.crane.follows = "crane";
      inputs.fenix.follows = "fenix";
      inputs.flake-utils.follows = "flake-utils";
      inputs.advisory-db.follows = "advisory-db";
    };
  };

  outputs = { self, nixpkgs, crane, fenix, flake-utils, advisory-db, ... }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            self.outputs.overlays.zonegen
            self.inputs.dyndnsd.overlays.dyndnsd
          ];
        };

        builder = import ./nix/builder.nix { inherit crane fenix pkgs system; };
        inherit (builder)
          lib
          craneLib
          src
          commonArgs
          craneLibLLvmTools
          cargoArtifacts
          zonegen
        ;
      in
      {
        checks = {
          # Build the crate as part of `nix flake check` for convenience
          inherit zonegen;

          # Run clippy (and deny all warnings) on the crate source,
          # again, reusing the dependency artifacts from above.
          #
          # Note that this is done as a separate derivation so that
          # we can block the CI if there are issues here, but not
          # prevent downstream consumers from building our crate by itself.
          zonegen-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          zonegen-doc = craneLib.cargoDoc (commonArgs // {
            inherit cargoArtifacts;
          });

          # Check formatting
          zonegen-fmt = craneLib.cargoFmt {
            inherit src;
          };

          # Audit dependencies
          zonegen-audit = craneLib.cargoAudit {
            inherit src advisory-db;
          };

          # Audit licenses
          zonegen-deny = craneLib.cargoDeny {
            inherit src;
          };

          # Run tests with cargo-nextest
          # Consider setting `doCheck = false` on `zonegen` if you do not want
          # the tests to run twice
          zonegen-nextest = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
          });

          zonegen-reuse = pkgs.runCommand "run-reuse" {
            src = ./.;
            nativeBuildInputs = with pkgs; [ reuse ];
          } ''
            cd "$src"
            reuse lint --lines
            touch "$out"
          '';

        # NixOS tests don't run on macOS
        } // lib.optionalAttrs (!pkgs.stdenv.isDarwin) {
          zonegen-e2e-test = pkgs.testers.runNixOSTest (import ./nix/e2e-test.nix self);
        };

        packages = {
          inherit zonegen;
          default = self.packages.${system}.zonegen;
        } // lib.optionalAttrs (!pkgs.stdenv.isDarwin) {
          zonegen-llvm-coverage = craneLibLLvmTools.cargoLlvmCov (commonArgs // {
            inherit cargoArtifacts;
          });
        };

        apps.zonegen = flake-utils.lib.mkApp {
          drv = zonegen;
        };
        apps.default = self.apps.${system}.zonegen;

        devShells.zonegen = craneLib.devShell {
          # Inherit inputs from checks.
          checks = self.checks.${system};

          # Additional dev-shell environment variables can be set directly
          # MY_CUSTOM_DEVELOPMENT_VAR = "something else";

          # Extra inputs can be added here; cargo and rustc are provided by default.
          packages = with pkgs; [
            sqlx-cli
          ];
        };
        devShells.default = self.devShells.${system}.zonegen;
      }) // {
        overlays.zonegen = import ./nix/overlay.nix (import ./nix/builder.nix) crane fenix;
        overlays.default = self.overlays.zonegen;
      };
}
