{ pkgs }:

rec {
  lon = pkgs.callPackage ./lon.nix { };
  lonTests = pkgs.callPackage ./lon-tests.nix { inherit lon; };
}
