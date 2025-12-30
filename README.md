# Glacian

**IN PROGRESS**
**None Of The Features Have Been Fully Implemented**

A Simplistic Voxel Game Engine.

# Goals

My goal for creating this engine was to make a fast and efficient library that I can use to make voxel games.
The primary way this will be efficient is by just not including many of the featurs found in actual game engines.
For example, I will only have ~10 render pipelines while a normal game engine may have thousands. I will also only
support rendering Signed Distance Field fonts as they are fast to render. The subprojects like glacian-render are
to create clean API boundaries and to segment platform diffences in smaller chunks rather than in the main crate.
I will also only support procedural terrain generation.

# Try it Out

Go to the github releases tab and download the binary for your platform.

# Important Packages

- ash (vulkan)
- SDL3 (used for windowing and controls)
- wgpu (not implemented yet butr will be very crucial)
- glam (SIMD fast math)
- binary-greedy-meshing
- vk_mem (rust port of AMD's Vulkan Memory Allocator)
- rayon (seamless parallelization of computation)

### Features

- [ ] integrate with the [Neuro SDK](https://github.com/VedalAI/neuro-game-sdk):

  - [Rust Implementation](https://github.com/chayleaf/rust-neuro-sama-game-api):
  - [Randy](https://github.com/VedalAI/neuro-sdk/blob/main/Randy/README.md):

- [ ] add proper error handling
- [ ] [FSR 3.1](https://gpuopen.com/download/FidelityFX_Super_Resolution_3-1_Release-Overview_and_Integration.pdf) or FSR4

### Decisions

- [ ] Decide whether to use procedural fauna
- [ ] Create a global instant submit struct stored by the engine instead of creating it every time a new GPU mesh buffer is created

### Optimizations

- [ ] support multiple queues
- [ ] https://nnethercote.github.io/perf-book/introduction.html

### Plugins

- [ ] Ahead Of Time compilation using wasmtime
- [ ] Create API for entity creation and spawning

## World Building

### Game Summary

A voxel game where the player is a robot exploring a post-nuclear war world.
This world will be frozen and coated in ice with vegetation being rare. One of
the struggles will be finding a consistent source of energy to power yourself.
You start with a nuclear battery which lasts for a very long time but provides
very little power. It will be enough for powering the base but not any upgrades.

### Inspiration

- Lost Terminal: This is inspired by the protagonist Seth who is an artificial
  intelligence trying to piece together the world around him.
