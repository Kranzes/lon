{
  lib,
  rustPlatform,
  makeBinaryWrapper,
  nix,
  nix-prefetch-git,
  git,
  clippy,
  rustfmt,
}:

let
  cargoToml = builtins.fromTOML (builtins.readFile ../../rust/lon/Cargo.toml);
in
rustPlatform.buildRustPackage (finalAttrs: {
  pname = cargoToml.package.name;
  inherit (cargoToml.package) version;

  src = lib.sourceFilesBySuffices ../../rust/lon [
    ".rs"
    ".toml"
    ".lock"
    ".nix"
  ];

  cargoLock = {
    lockFile = ../../rust/lon/Cargo.lock;
  };

  nativeBuildInputs = [ makeBinaryWrapper ];

  postInstall = ''
    wrapProgram $out/bin/lon --prefix PATH : ${
      lib.makeBinPath [
        nix
        nix-prefetch-git
        git
      ]
    }
  '';

  stripAllList = [ "bin" ];

  passthru.tests = {
    clippy = finalAttrs.finalPackage.overrideAttrs (
      _: previousAttrs: {
        pname = previousAttrs.pname + "-clippy";
        nativeCheckInputs = (previousAttrs.nativeCheckInputs or [ ]) ++ [ clippy ];
        checkPhase = "cargo clippy";
      }
    );
    fmt = finalAttrs.finalPackage.overrideAttrs (
      _: previousAttrs: {
        pname = previousAttrs.pname + "-rustfmt";
        nativeCheckInputs = (previousAttrs.nativeCheckInputs or [ ]) ++ [ rustfmt ];
        checkPhase = "cargo fmt --check";
      }
    );

  };

  meta = with lib; {
    homepage = "https://github.com/nikstur/lon";
    license = licenses.mit;
    maintainers = with lib.maintainers; [ nikstur ];
    mainProgram = "lon";
  };
})
