{
  description = "Gitwig — a keyboard-driven terminal Git UI, an alternative to SourceTree and gitui";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
  };

  outputs = { self, nixpkgs }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" "x86_64-darwin" "aarch64-darwin" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
      # Single source of truth: the flake never carries its own version string.
      version = (builtins.fromTOML (builtins.readFile ./Cargo.toml)).package.version;
    in
    {
      packages = forAllSystems (pkgs: rec {
        gitwig = pkgs.rustPlatform.buildRustPackage {
          pname = "gitwig";
          inherit version;
          src = self;
          cargoLock.lockFile = ./Cargo.lock;

          # This nixpkgs pin's cargoInstallHook/cargoInstallPostBuildHook pair
          # misfires on darwin (it expects a `release-tmp` staging dir that
          # never materializes, and `target/` is gone by install time). Capture
          # the binaries at postBuild — the moment they exist — instead.
          postBuild = ''
            mkdir -p "$out/bin"
            install -m755 \
              "target/${pkgs.stdenv.hostPlatform.rust.rustcTarget}/release/gitwig" \
              "target/${pkgs.stdenv.hostPlatform.rust.rustcTarget}/release/gtg" \
              -t "$out/bin"
          '';
          installPhase = ''
            runHook preInstall
            test -x "$out/bin/gitwig" && test -x "$out/bin/gtg"
            runHook postInstall
          '';

          # The test suite creates throwaway Git repositories.
          nativeCheckInputs = [ pkgs.git ];

          # The whole suite lives in the library target; the bin targets carry
          # no unit tests and their test binaries don't survive the sandboxed
          # rebuild between build and check phases.
          cargoTestFlags = [ "--lib" ];

          # These tests need the source checkout itself to be a Git repository
          # (they inspect ".") or a real terminal size; neither exists in the
          # build sandbox. They run in CI via `make test`.
          checkFlags = [
            "--skip=app::tests::test_detail_view_sync_on_tab_change_and_refresh"
            "--skip=app::tests::test_file_history_view_flow"
            "--skip=app::tests::test_navigation_actions_direct"
            # Some tests share fixed paths under $TMPDIR; the sandbox's
            # scheduling makes those races bite where `make test` does not.
            "--test-threads=1"
          ];

          meta = {
            description = "Keyboard-driven terminal Git UI, an alternative to SourceTree and gitui";
            homepage = "https://github.com/tareqmy/gitwig";
            changelog = "https://github.com/tareqmy/gitwig/blob/master/CHANGELOG.md";
            license = pkgs.lib.licenses.mit;
            mainProgram = "gitwig";
          };
        };
        default = gitwig;
      });

      devShells = forAllSystems (pkgs: {
        default = pkgs.mkShell {
          packages = with pkgs; [
            rustc
            cargo
            clippy
            rustfmt
            rust-analyzer
            git
            python3 # scripts/generate_changelog.py
          ];
        };
      });
    };
}
