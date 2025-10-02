let
  pkgs = import <nixpkgs> {};
in
  pkgs.mkShell {
    buildInputs = with pkgs; [
      sdl3
      vulkan-validation-layers
      vulkan-loader
      shader-slang
      vulkan-tools-lunarg
      vulkan-tools
      cargo-zigbuild
      cmake
    ];

    LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath [
      pkgs.sdl3
      pkgs.vulkan-loader
      pkgs.wayland
    ]}";

    VK_LAYER_PATH = "${pkgs.vulkan-validation-layers}/share/vulkan/explicit_layer.d";
  }
