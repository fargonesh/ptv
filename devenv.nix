{
  pkgs,
  lib,
  config,
  ...
}:
{
  # https://devenv.sh/languages/
  languages.rust = {
    enable = true;
    channel = "nightly";
    components = [
      "rustc"
      "cargo"
      "clippy"
      "rustfmt"
      "rust-analyzer"
    ];
  };

  # https://devenv.sh/packages/
  packages = [
    # Packages used for flake-parts modules:
    pkgs.nil
    pkgs.mold
    pkgs.lld
    pkgs.statix
    pkgs.deadnix
    pkgs.alejandra
  ];

  # See full reference at https://devenv.sh/reference/options/
}
