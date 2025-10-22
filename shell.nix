{
  pkgs ? import <nixpkgs> { },
}:

pkgs.mkShell {
  buildInputs = [
    pkgs.cargo
    pkgs.gcc
    pkgs.gnumake
    pkgs.rustc
    pkgs.rustfmt
  ];
}
