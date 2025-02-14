Todo:

- Configuration
   - More transitions
      - Configurable transitions
      - Fastblur transition?
   - More text properties (font, size, color, position, padding, etc...)

   - Blur (radius, algo...) - OK for now

- Graphics:
   - Splash screen transition
   - Pixel perfect rework - OK?

   - Splash screen - OK for now
   - Handle descriptions from immich (memory lane, etc...)
   - Info placement - OK for now
   - Other layouts when possible (two photos at the same time)

- Technical:
   - More robust worker (network errors, etc...)
   - unit testing where possible
   - Debian packaging
      - config sample generate
   - Just arm deploy
   - Feature flags for GBM, winit, etc...

   - Better immich errors - OK?
   - Direnv -> switch to flake
   - Better init for GBM/winit
   - switch to https://lib.rs/crates/schematic for config? -> not a good library for our use case
     - Test https://docs.rs/foundations
   - from Schematic -> better serde errors OR switch to schematic
   - Error handling - OK? (miette)

   - Sleep when nothing to do ? -> no with zoom effect
   - Investigate text rendering using signed distance fields (SDF)
   - profiling - hard on ARMv6
   - Reduce Gl calls (shader bindings) - OK?


- HTTP Api ?
- publish

Bugs:
  - Handle next page for smartSearch

Improves:
- GBM ressources:
   - Ask Gemini
   - https://docs.kernel.org/gpu/drm-kms.html
   - https://github.com/ds-hwang/gbm_es2_demo/blob/master/rust/examples/opengl_egl_drm.rs#L275
