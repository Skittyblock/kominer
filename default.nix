{
  lib,
  rustPlatform,
  rev ? "dirty",
}: let
  p = (lib.importTOML ./Cargo.toml).package;
in
  rustPlatform.buildRustPackage {
    pname = p.name;
    inherit (p) version;

    src = lib.fileset.toSource {
      root = ./.;
      fileset = lib.fileset.intersection (lib.fileset.fromSource (lib.sources.cleanSource ./.)) (
        lib.fileset.unions [
          ./Cargo.toml
          ./Cargo.lock
          ./src
          ./templates
        ]
      );
    };

    cargoLock.lockFile = ./Cargo.lock;

    buildInputs = [];
    nativeBuildInputs = [];

    postInstall = ''
      mkdir -p $out/share/${p.name}
      cp -r ${./static} $out/share/${p.name}/static
    '';

    env = {
      BUILD_REV = rev;
    };

    meta = {
      inherit (p) description homepage;
      license = lib.licenses.mit;
      maintainers = with lib.maintainers; [skitty];
      mainProgram = p.name;
    };
  }
