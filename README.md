Todo:

- Configuration
   - Blur (radius, algo...) - OK for now
   - Transitions
   - Other searches - Ok
   - Multiple sources - Ok

- Graphics:
   - Configurable orientation - OK
   - Better texts (date format)
   - Info placement - OK for now
   - Zoom effect -> check for background worker first! (otherwise it will be freezed...)
   - Other effects / transitions -> fadeout-fadein
   - Other layouts when possible (two photos at the same time)

- Technical:
   - Error handling - OK?
   - Better immich errors - OK?
   - Sleep when nothing to do ?
   - Background loading / blur with GPU (context sharing)
   - Investigate text rendering using signed distance fields (SDF)
   - Direnv -> switch to flake
   - Better init for GBM/winit
   - Refactor application.rs
   - Write unit tests
   - switch to https://lib.rs/crates/schematic for config?

- HTTP Api ?
- publish

Bugs:
  - Handle next page for smartSearch

Improves:
- GBM ressources:
   - Ask Gemini
   - https://docs.kernel.org/gpu/drm-kms.html
   - https://github.com/ds-hwang/gbm_es2_demo/blob/master/rust/examples/opengl_egl_drm.rs#L275
