{
  description = "codex commit message generator";

  nixConfig = {
    extra-substituters = [
      "https://nix.trev.zip"
    ];
    extra-trusted-public-keys = [
      "trev:I39N/EsnHkvfmsbx8RUW+ia5dOzojTQNCTzKYij1chU="
    ];
  };

  inputs = {
    systems.url = "github:spotdemo4/systems";
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    trevpkgs = {
      url = "github:spotdemo4/trevpkgs";
      inputs.systems.follows = "systems";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      trevpkgs,
      ...
    }:
    trevpkgs.libs.mkFlake (
      system: pkgs: {

        # nix develop [#...]
        devShells = {
          default = pkgs.mkShell {
            RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
            shellHook = pkgs.shellhook.ref;
            packages = with pkgs; [
              # rust
              rustc
              cargo

              # lint
              clippy
              cargo-audit
              nixd
              nil

              # format
              rustfmt
              nixfmt
              oxfmt
              treefmt

              # util
              bumper
              fix-hash
            ];
          };

          bump = pkgs.mkShell {
            packages = with pkgs; [
              bumper
            ];
          };

          release = pkgs.mkShell {
            packages = with pkgs; [
              cargo
              flake-release
              rustc
            ];
          };

          update = pkgs.mkShell {
            packages = with pkgs; [
              renovate
              cargo # rust
            ];
          };

          vulnerable = pkgs.mkShell {
            packages = with pkgs; [
              flake-checker # nix
              zizmor # actions
              cargo-audit # rust
            ];
          };
        };

        # nix run [#...]
        apps = pkgs.mkApps {
          dev = "cargo run";
          test = "cargo test";
        };

        # nix build [#...]
        packages = {
          default = pkgs.rustPlatform.buildRustPackage (
            final: with pkgs.lib; {
              pname = "codex-commit";
              version = "0.0.1";

              src = fileset.toSource {
                root = ./.;
                fileset = fileset.unions [
                  ./Cargo.lock
                  ./Cargo.toml
                  ./LICENSE
                  ./README.md
                  ./instructions.md
                  (fileset.fileFilter (file: file.hasExt "rs") ./.)
                ];
              };
              cargoHash = "sha256-iWBMNBZFnM7nC3eUug2hD6WCh/pcOCgqZgRpxRRNlds=";

              nativeCheckInputs = with pkgs; [
                rustfmt
                clippy
              ];
              checkPhase = ''
                cargo fmt --check
                cargo test --offline
                cargo clippy --offline -- -D warnings
              '';

              meta = {
                mainProgram = "codex-commit";
                description = "codex commit message generator";
                license = licenses.mit;
                platforms = platforms.all;
                homepage = "https://trev.zip/llc/rust";
                changelog = "https://trev.zip/llc/rust/releases/tag/v${final.version}";
                downloadPage = "https://trev.zip/llc/rust/releases/releases/tag/v${final.version}";
              };
            }
          );
        };

        # nix build #images.[...]
        images = {
          default = pkgs.mkImage {
            src = self.packages.${system}.default;
            contents = with pkgs; [ dockerTools.caCertificates ];
          };
        };

        # nix fmt
        formatter = pkgs.treefmt.withConfig {
          configFile = ./treefmt.toml;
          runtimeInputs = with pkgs; [
            rustfmt
            nixfmt
            oxfmt
          ];
        };

        # nix flake check
        checks = pkgs.mkChecks {
          rust = self.packages.${system}.default.overrideAttrs {
            dontBuild = true;
            installPhase = ''
              touch $out
            '';
          };

          nix = {
            root = ./.;
            filter = file: file.hasExt "nix";
            packages = with pkgs; [
              nixfmt
            ];
            script = ''
              nixfmt --check "$file"
            '';
          };

          actions = {
            root = ./.forgejo/workflows;
            filter = file: file.hasExt "yaml";
            packages = with pkgs; [
              zizmor
            ];
            script = ''
              zizmor --offline "$file"
            '';
          };

          renovate = {
            root = ./.forgejo;
            fileset = ./.forgejo/renovate.json;
            packages = with pkgs; [
              renovate
            ];
            script = ''
              renovate-config-validator renovate.json
            '';
          };

          config = {
            root = ./.;
            filter = file: file.hasExt "json" || file.hasExt "yaml" || file.hasExt "toml" || file.hasExt "md";
            packages = with pkgs; [
              oxfmt
            ];
            script = ''
              oxfmt --check
            '';
          };
        };
      }
    );
}
