{
  pkgs ? import <nixpkgs> { },
}:

pkgs.mkShell {
  buildInputs = [
    pkgs.rustc
    pkgs.rustfmt
    pkgs.cargo
  ];
}
