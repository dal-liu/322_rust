{
  pkgs ? import <nixpkgs> { },
}:

pkgs.mkShell {
  buildInputs = [
    pkgs.cargo
    pkgs.gcc
    pkgs.rustc
    pkgs.rustfmt
  ];
}
