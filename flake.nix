{
  description = "Engine for a two player game of scout";

  inputs = {
    flake-utils.url = "github:numtide/flake-utils";
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        # Import nixpkgs with the overlay
        pkgs = import nixpkgs {
          inherit system;
          # Ensure your overlay.nix is compatible or remove if not needed for the dev env
          overlays = [];
          # Allow non-free packages like PyTorch potentially needs (e.g., MKL)
          config.allowUnfree = true;
        };

        # Create a Python 3.12 environment with PyTorch
        # Using withPackages ensures pytorch is available directly for this python interpreter
        pythonEnv = pkgs.python312.withPackages (ps: [
          ps.pytorch # Or ps.pytorchWithCuda if you need GPU support
          # Add other python packages here if needed, e.g.: ps.numpy, ps.pip
        ]);

      in {
        # Keep your default package definition if you still build 'scout'
        packages.default = pkgs.scout;

        # Define the development shell
        devShell = pkgs.mkShell {
          # List the packages needed in the environment
          buildInputs = [
            # Rust Toolchain
            pkgs.cargo
            pkgs.rustc
            pkgs.rustfmt
            pkgs.rust-analyzer

            # Python environment (includes Python 3.12 + PyTorch)
            pythonEnv

            # Add any other system-level tools you need
            pkgs.git
          ];

           RUST_SRC_PATH = "${pkgs.rust.packages.stable.rustPlatform.rustLibSrc}";

          shellHook = ''
            # Prepend text to the existing PS1 to indicate the active environment
		export PS1="\[\033[0;36m\](scout-dev)\[\033[0m\] \u@\h:\w\$ "
          '';
        };
      });
}
