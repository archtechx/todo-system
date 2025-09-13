{
  description = "An intuitive system for organizing TODOs in code";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
      in
        {
          packages.default = pkgs.rustPlatform.buildRustPackage {
            name = "todos";
            src = pkgs.lib.cleanSource ./.;
            cargoLock.lockFile = ./Cargo.lock;

            meta = with pkgs.lib; {
              description = "An intuitive system for organizing TODOs in code";
              homepage = "https://github.com/archtechx/todo-system";
              license = licenses.mit;
            };
          };

          devShells.default = pkgs.mkShell {
            buildInputs = with pkgs; [
              cargo
              rustc
              clippy
            ];
          };
      }
    );
}
