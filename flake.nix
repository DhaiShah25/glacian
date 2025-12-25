{
  description = "A very basic flake";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs?ref=nixos-unstable";
  };

  outputs = {
    self,
    nixpkgs,
  }: let
    pkgs = nixpkgs.legacyPackages."x86_64-linux";
  in {
    devShells."x86_64-linux".default = pkgs.mkShell {
      buildInputs = with pkgs; [
        wayland
        sdl3
        cmake

        vulkan-validation-layers
        vulkan-loader
        shader-slang
        vulkan-tools-lunarg
        vulkan-tools
      ];
      nativeBuildInputs = with pkgs; [
        pkg-config
      ];
      LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath [pkgs.wayland pkgs.vulkan-loader pkgs.sdl3]}";
      VK_LAYER_PATH = "${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";
    };
  };
}
