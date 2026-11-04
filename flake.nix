{
  description = "Belief Nexus version 1";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-build = { url = "github:LiGoldragon/rust-build"; inputs.nixpkgs.follows = "nixpkgs"; };
    ethos-zero.url = "github:LiGoldragon/ethos-zero/b8052c2404374265dfcddbde83fef76977f78acd";
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
        buildBinary = name: rust.craneLib.buildPackage {
          pname = name; version = "0.1.0"; src = source; inherit cargoArtifacts;
          cargoExtraArgs = "-p belief-nexus-translator --bin ${name} --all-features";
          doCheck = false;
        };
      in {
        packages = {
          default = buildBinary "belief-translator";
          translator = buildBinary "belief-translator";
          cli = buildBinary "belief-nexus";
          meta-cli = buildBinary "belief-nexus-meta";
        };
        checks = {
          test = rust.craneLib.cargoTest {
            pname = "belief-nexus-test"; version = "0.1.0"; src = source; inherit cargoArtifacts;
            cargoTestExtraArgs = "--workspace --all-features";
            nativeBuildInputs = [ pkgs.git ];
          };
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
