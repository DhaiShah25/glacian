let
  pkgs = import <nixpkgs> {};
  unstable = import <nixos-unstable> {config = {allowUnfree = true;};};
in
  pkgs.mkShell {
    buildInputs = [
      pkgs.sdl3
      pkgs.vulkan-validation-layers
      pkgs.vulkan-loader
      pkgs.shader-slang
      pkgs.vulkan-tools-lunarg
      unstable.rustc
      unstable.cargo
      unstable.rust-analyzer
      unstable.clippy
      unstable.rustfmt
      pkgs.vulkan-tools
    ];

    LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath [
      pkgs.sdl3
      pkgs.vulkan-loader
      pkgs.wayland
    ]}";

    VK_LAYER_PATH = "${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";
  }
