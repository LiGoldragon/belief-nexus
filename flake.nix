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
      # Its crates are the base's, under the feature set name this build takes.
      withAllFeatures =
        { workspace, binaries }:
        name: binaries name (allFeatures workspace workspace.workspaceMembers.${name}.packageId);
      workspaces = lib.genAttrs base.lib.systems (
        system:
        let
          context = base.lib.contexts.${system};
          pkgs = context.pkgs;
          composed = base.lib.workspace.${system} { inherit cargoNix; };
          workspace = composed.workspace;
          member = withAllFeatures {
            inherit workspace;
            inherit (composed) binaries;
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
            cp ${translatorBuild}/bin/belief-pair "$out/libexec/belief-pair"
          '';
          # The settings a place starts with, which the translator applies
          # through the Nexus at its first start: crates/translator/deployment.datom,
          # its Git the one this flake builds with.
          deployment = builtins.replaceStrings [ "@git@" ] [ "${pkgs.git}/bin/git" ] (
            builtins.readFile ./crates/translator/deployment.datom
          );
          deployedTranslator = pkgs.writeShellScript "belief-translator-deployed" ''
            exec ${translator}/bin/belief-translator ${lib.escapeShellArg deployment}
          '';
          # The pair launcher every translator shares: the place, the Nexus,
          # the translator, the admission (frame deadline, frame capacity,
          # retry delay) and the text budget the translator starts with.
          pair = pkgs.writeShellScriptBin "belief-pair" ''
            if [[ $# -ne 1 ]]; then echo "usage: belief-pair '<Place>'" >&2; exit 64; fi
            exec ${internal}/libexec/belief-pair "$1" \
              ${nexus}/bin/belief-nexus-daemon \
              ${deployedTranslator} \
              70000 8388608 25 100000 64
          '';
          # Each CLI takes its datom alone; its place is the one
          # nexus/belief.datom names under the configuration home.
          cli = pkgs.runCommand "belief-nexus-cli" { } ''
            mkdir -p "$out/bin"
            cp ${translatorBuild}/bin/belief-nexus "$out/bin/belief-nexus"
          '';
          meta-cli = pkgs.runCommand "belief-nexus-meta-cli" { } ''
            mkdir -p "$out/bin"
            cp ${translatorBuild}/bin/belief-nexus-meta "$out/bin/belief-nexus-meta"
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
          { pkgs, workspace, doc, ... }:
          let
          # A committed generation, which begins with what ethos-zero generates
          # from its declaration.
          generated =
            name: declaration: committed:
            pkgs.runCommand "belief-${name}-generated" {
              generator = ethos-zero.packages.${pkgs.stdenv.hostPlatform.system}.default;
              inherit declaration committed;
            } ''
              temporary="$(${pkgs.coreutils}/bin/mktemp -d)"
              "$generator/bin/ethos-zero" "Generate.{ «$declaration» «$temporary» }"
              generated="$(${pkgs.findutils}/bin/find "$temporary" -maxdepth 1 -name '*.rs' -print -quit)"
              ${pkgs.coreutils}/bin/head -c "$(${pkgs.coreutils}/bin/stat -c %s "$generated")" "$committed" > "$temporary/committed-generated.rs"
              ${pkgs.diffutils}/bin/cmp "$generated" "$temporary/committed-generated.rs"
              touch "$out"
            '';
          lintChecks = lib.listToAttrs (
            lib.concatMap (member: [
              { name = "doc-${member}"; value = doc member (allFeatures workspace workspace.workspaceMembers.${member}.packageId) [ "-D" "warnings" ]; }
            ]) (lib.attrNames workspace.workspaceMembers)
          );
          in
          lintChecks // {
            emission-generated = generated "emission" ./crates/emission/emission.ethos ./crates/emission/src/lib.rs;
            public-call-generated = generated "public-call" ./crates/wire/ethos/public-call.ethos ./crates/wire/src/generated/public_call.rs;
            meta-call-generated = generated "meta-call" ./crates/wire/ethos/meta-call.ethos ./crates/wire/src/generated/meta_call.rs;
            translator-generated = generated "translator" ./crates/translator/ethos/translator.ethos ./crates/translator/src/generated/translator.rs;
          };
      };
    in
    {
      packages = lib.mapAttrs (system: built: workspace.packages.${system} // built.packages) workspaces;
      inherit (workspace) checks devShells;
    };
}
