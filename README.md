# Llama link

## Setup Llama.cpp Server

### Manual

- Clone [https://github.com/ggerganov/llama.cpp/]
- read [./llama.cpp/docs/build.md](https://github.com/ggerganov/llama.cpp/blob/master/docs/build.md) to build
- Run. e.g. `./build/bin/llama-server -m ./models/7B/ggml-model-f16.gguf --prompt "Once pick an action" --json-schema '{}'`


### NixOs

Options:
1. Use the package https://search.nixos.org/options?channel=unstable&from=0&size=50&sort=relevance&type=packages&query=llama-cpp

2. Use the flake at [./llama.cpp/flake.nix](https://github.com/ggerganov/llama.cpp/blob/master/flake.nix). E.g.
```nix
{
  description = "My CUDA-enabled llama.cpp development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    llama-cpp.url = "github:ggerganov/llama.cpp";
  };

  outputs = { self, nixpkgs, flake-parts, llama-cpp }@inputs:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [ "x86_64-linux" "aarch64-linux" ];

      perSystem = { config, self', inputs', pkgs, system, ... }: {
        devShells.default = pkgs.mkShell {
          buildInputs = [
            llama-cpp.packages.${system}.cuda

            pkgs.cudatoolkit
            pkgs.gcc
            pkgs.cmake
          ];

          shellHook = ''
            export CUDA_PATH=${pkgs.cudatoolkit}
            export LD_LIBRARY_PATH=${pkgs.cudatoolkit}/lib:$LD_LIBRARY_PATH
          '';
        };
      };
    };
}
```