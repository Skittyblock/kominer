{
  inputs.nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";

  outputs = {
    self,
    nixpkgs,
  }: let
    systems = [
      "x86_64-linux"
      "aarch64-linux"
      "x86_64-darwin"
      "aarch64-darwin"
    ];
    forAllSystems = function: nixpkgs.lib.genAttrs systems (system: function nixpkgs.legacyPackages.${system});
  in {
    packages = forAllSystems (pkgs: rec {
      default = kominer;

      kominer = pkgs.callPackage ./default.nix {
        rev = self.dirtyRev or self.rev or "dirty";
        publicMode = false;
      };
      kominer-public = pkgs.callPackage ./default.nix {
        rev = self.dirtyRev or self.rev or "dirty";
        publicMode = true;
      };

      all = pkgs.linkFarm "kominer-all" [
        {
          name = "kominer";
          path = kominer;
        }
        {
          name = "kominer-public";
          path = kominer-public;
        }
      ];
    });

    devShells = forAllSystems (pkgs: {
      default = pkgs.callPackage ./shell.nix {};
    });
  };
}
