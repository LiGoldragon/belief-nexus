{
  description = "Belief Nexus version 1";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-build = { url = "github:LiGoldragon/rust-build"; inputs.nixpkgs.follows = "nixpkgs"; };
    ethos-zero.url = "github:LiGoldragon/ethos-zero";
  };
  outputs = { self, nixpkgs, flake-utils, rust-build, ethos-zero }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = import nixpkgs { inherit system; };
        rust = rust-build.lib.${system}.fromPkgs pkgs;
        source = rust.cleanSource {
          root = ./.;
          extraFilters = [
            (path: type: type == "regular" && pkgs.lib.hasSuffix ".ethos" (toString path))
            (path: type: type == "regular" && pkgs.lib.hasSuffix ".sh" (toString path))
          ];
        };
        cargoArtifacts = rust.craneLib.buildDepsOnly { src = source; cargoExtraArgs = "--workspace --all-features"; };
        buildBinary = package: name: rust.craneLib.buildPackage {
          pname = name; version = "0.1.0"; src = source; inherit cargoArtifacts;
          cargoExtraArgs = "-p ${package} --bin ${name} --all-features";
          doCheck = false;
        };
      in {
        packages = rec {
          nexus = buildBinary "belief-nexus-daemon" "belief-nexus-daemon";
          translator = buildBinary "belief-nexus-translator" "belief-translator";
          internal = rust.craneLib.buildPackage {
            pname = "belief-nexus-internal"; version = "0.1.0"; src = source; inherit cargoArtifacts;
            cargoExtraArgs = "-p belief-nexus-translator --bin belief-nexus --bin belief-nexus-meta --bin belief-pair --all-features";
            doCheck = false;
            postInstall = ''
              mkdir -p "$out/libexec"
              for executable in belief-nexus belief-nexus-meta belief-pair; do
                mv "$out/bin/$executable" "$out/libexec/$executable"
              done
              rmdir "$out/bin"
            '';
          };
          pair = pkgs.writeShellScriptBin "belief-pair" ''
            if [[ $# -ne 1 ]]; then echo "usage: belief-pair '<Place>'" >&2; exit 64; fi
            exec ${internal}/libexec/belief-pair "$1" \
              "{ «${nexus}/bin/belief-nexus-daemon» «${translator}/bin/belief-translator» belief.sock belief-meta.sock belief-translator.sock belief-meta-translator.sock belief.memory { 8388608 70000 } Login.$(${pkgs.coreutils}/bin/id -u) [ RepositoryPolling.100 SubscriberCapacity.64 OrdinaryPeers.[] MetaPeers.[] Layout.{ Codex { .belief generations current .codex/config.toml model_instructions_file instructions.md .agents/skills } } Layout.{ Claude { .belief generations current CLAUDE.md unused-for-claude CLAUDE.md .claude/skills } } Placement.{ Spirit SystemPrompt } Placement.{ Intent SystemPrompt } Placement.{ Vision Skills } Placement.{ Knowledge Skills } Placement.{ Role Skills } GenerationRetention.KeepAll NexusFrameCapacity.8388608 NexusTransportTimeout.70000 CallTimeout.70000 OutputLimit.8388608 TextBudget.{ 100000 64 } TranslatorFrameCapacity.8388608 TranslatorTransportTimeout.70000 GitExecutable.«${pkgs.git}/bin/git» ] }"
          '';
          cli = pkgs.writeShellScriptBin "belief-nexus" ''
            if [[ $# -ne 2 ]]; then echo "usage: belief-nexus <metaflow> '<datom>'" >&2; exit 64; fi
            exec ${internal}/libexec/belief-nexus "$1" "$2" belief-translator.sock
          '';
          meta-cli = pkgs.writeShellScriptBin "belief-nexus-meta" ''
            if [[ $# -ne 2 ]]; then echo "usage: belief-nexus-meta <metaflow> '<datom>'" >&2; exit 64; fi
            exec ${internal}/libexec/belief-nexus-meta "$1" "$2" belief-meta-translator.sock
          '';
          default = pkgs.symlinkJoin { name = "belief-nexus-v1"; paths = [ nexus translator pair cli meta-cli ]; };
        };
        checks = {
          test = rust.craneLib.cargoTest {
            pname = "belief-nexus-test"; version = "0.1.0"; src = source; inherit cargoArtifacts;
            cargoTestExtraArgs = "--workspace --all-features";
            nativeBuildInputs = [ pkgs.git ];
          };
          daemon-memory-default = rust.craneLib.buildPackage {
            pname = "belief-nexus-daemon-memory-default"; version = "0.1.0";
            src = source; inherit cargoArtifacts;
            cargoExtraArgs = "-p belief-nexus-daemon --bin belief-nexus-daemon";
            doCheck = false;
          };
          daemon-memory-all-features = rust.craneLib.buildPackage {
            pname = "belief-nexus-daemon-memory-all-features"; version = "0.1.0";
            src = source; inherit cargoArtifacts;
            cargoExtraArgs = "-p belief-nexus-daemon --bin belief-nexus-daemon --all-features";
            doCheck = false;
          };
          memory-generated = pkgs.runCommand "belief-memory-generated" {
            generator = ethos-zero.packages.${system}.default;
            declaration = ./memory.ethos;
            committed = ./crates/daemon/src/memory.rs;
          } ''
            temporary="$(${pkgs.coreutils}/bin/mktemp -d)"
            "$generator/bin/ethos-zero" "Generate.{ «$declaration» «$temporary» }"
            generated="$(${pkgs.findutils}/bin/find "$temporary" -maxdepth 1 -name '*.rs' -print -quit)"
            ${pkgs.diffutils}/bin/cmp "$generated" "$committed"
            touch "$out"
          '';
          emission-generated = pkgs.runCommand "belief-emission-generated" {
            generator = ethos-zero.packages.${system}.default;
            declaration = ./emission.ethos;
            committed = ./crates/emission/src/lib.rs;
          } ''
            temporary="$(${pkgs.coreutils}/bin/mktemp -d)"
            "$generator/bin/ethos-zero" "Generate.{ «$declaration» «$temporary» }"
            generated="$(${pkgs.findutils}/bin/find "$temporary" -maxdepth 1 -name '*.rs' -print -quit)"
            ${pkgs.coreutils}/bin/head -c "$(${pkgs.coreutils}/bin/stat -c %s "$generated")" "$committed" > "$temporary/committed-generated.rs"
            ${pkgs.diffutils}/bin/cmp "$generated" "$temporary/committed-generated.rs"
            touch "$out"
          '';
        };
        devShells.default = pkgs.mkShell { packages = [ rust.toolchain pkgs.git ]; };
      });
}
