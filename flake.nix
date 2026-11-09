{
  description = "Belief Nexus version 1";

  inputs = {
    base.url = "github:LiGoldragon/build-base";
    # The generator that regenerates the committed emission for its check.
    ethos-zero = {
      url = "github:LiGoldragon/ethos-zero";
      inputs.base.follows = "base";
    };
  };

  # Every crate, third-party or member, is one buildRustCrate derivation.
  # Cargo.nix is generated from Cargo.lock by the build base's crate2nix and
  # committed, so evaluation imports no derivation. After editing a
  # Cargo.toml or Cargo.lock:
  #   nix run github:LiGoldragon/build-base#crate2nix -- generate
  outputs =
    { self, base, ethos-zero }:
    let
      lib = base.inputs.nixpkgs.lib;
      cargoNix = ./Cargo.nix;
      # Every feature of a member on, as `cargo build --all-features` builds it.
      allFeatures =
        workspace: packageId: lib.attrNames (workspace.internal.crates.${packageId}.features or { });
      withAllFeatures =
        { workspace, buildRustCrateForPkgs, crateOverrides }:
        name:
        let
          packageId = workspace.workspaceMembers.${name}.packageId;
        in
        (workspace.internal.builtRustCratesWithFeatures {
          inherit packageId;
          features = allFeatures workspace packageId;
          runTests = false;
          buildRustCrateForPkgsFunc =
            p: (buildRustCrateForPkgs p).override { defaultCrateOverrides = crateOverrides; };
        }).crates.${packageId}.override
          { inherit crateOverrides; };
      workspaces = lib.genAttrs base.lib.systems (
        system:
        let
          context = base.lib.contexts.${system};
          pkgs = context.pkgs;
          composed = base.lib.workspace.${system} { inherit cargoNix; };
          workspace = composed.workspace;
          member = withAllFeatures {
            inherit workspace;
            inherit (composed) buildRustCrateForPkgs;
            inherit (context) crateOverrides;
          };
          daemon = member "belief-nexus-daemon";
          translatorBuild = member "belief-nexus-translator";
          nexus = pkgs.runCommand "belief-nexus-daemon" { } ''
            mkdir -p "$out/bin"
            cp ${daemon}/bin/belief-nexus-daemon "$out/bin/belief-nexus-daemon"
          '';
          translator = pkgs.runCommand "belief-translator" { } ''
            mkdir -p "$out/bin"
            cp ${translatorBuild}/bin/belief-translator "$out/bin/belief-translator"
          '';
          internal = pkgs.runCommand "belief-nexus-internal" { } ''
            mkdir -p "$out/libexec"
            for executable in belief-nexus belief-nexus-meta belief-pair; do
              cp ${translatorBuild}/bin/$executable "$out/libexec/$executable"
            done
          '';
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
        in
        {
          packages = { inherit nexus translator internal pair cli meta-cli default; };
        }
      );
      workspace = base.lib.workspaceOutputs {
        inherit cargoNix;
        memberChecks =
          { pkgs, workspace, check, ... }:
          lib.mapAttrs (
            name: member:
            let
              # The base's own test build, so its dependencies are the base's derivations.
              tests = (check name (allFeatures workspace member.packageId)).tests;
            in
            pkgs.runCommand "check-${name}"
              ({
                passthru = { inherit tests; };
                nativeBuildInputs = [ pkgs.git ];
              }
              // lib.optionalAttrs (name == "belief-nexus-translator") {
                # The translator's pair test launches the built daemon.
                BELIEF_NEXUS_DAEMON = "${workspaces.${pkgs.stdenv.hostPlatform.system}.packages.nexus}/bin/belief-nexus-daemon";
              })
              ''
                cp -r ${tests.src} "$NIX_BUILD_TOP/source"
                chmod -R u+w "$NIX_BUILD_TOP/source"
                cd "$NIX_BUILD_TOP/source"
                mkdir -p "$(cat ${tests}/target-tmpdir)"
                for binary in ${tests}/tests/*; do
                  echo "running $(basename "$binary")"
                  "$binary"
                done
                touch $out
              ''
          ) workspace.workspaceMembers;
        extraChecks =
          { pkgs, workspace, clippy, doc, ... }:
          let
          lintChecks = lib.listToAttrs (
            lib.concatMap (member: [
              { name = "clippy-${member}"; value = clippy member (allFeatures workspace workspace.workspaceMembers.${member}.packageId) [ "-D" "warnings" ]; }
              { name = "doc-${member}"; value = doc member (allFeatures workspace workspace.workspaceMembers.${member}.packageId) [ "-D" "warnings" ]; }
            ]) (lib.attrNames workspace.workspaceMembers)
          );
          in
          lintChecks // {
            emission-generated = pkgs.runCommand "belief-emission-generated" {
            generator = ethos-zero.packages.${pkgs.stdenv.hostPlatform.system}.default;
            declaration = ./crates/emission/emission.ethos;
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
      };
    in
    {
      packages = lib.mapAttrs (system: built: workspace.packages.${system} // built.packages) workspaces;
      inherit (workspace) checks devShells;
    };
}
