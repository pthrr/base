{
  description = "Model-checking tools for the base workspace";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  inputs.creusot.url = "github:creusot-rs/creusot/v0.13.0";

  outputs = { nixpkgs, creusot, ... }:
    let
      systems = [ "x86_64-linux" "aarch64-linux" ];
      forAllSystems = f: nixpkgs.lib.genAttrs systems (system: f nixpkgs.legacyPackages.${system});
    in
    {
      devShells = forAllSystems (pkgs:
        {
          default = pkgs.mkShellNoCC {
            packages = [ pkgs.go-task pkgs.m4 pkgs.mcrl2 ];
          };
        }
        // nixpkgs.lib.optionalAttrs (pkgs.stdenv.hostPlatform.system == "x86_64-linux") {
          proof = pkgs.mkShellNoCC {
            packages = [ pkgs.go-task creusot.packages.x86_64-linux.free ];
          };
        });
    };
}
