Todo:

- switch to glow -> glium bugs (texture sampling) OK
   - Text - glyph branch -> try epaint OK
   - DRM/GDM/KMS rendering - OK
   - switch to Vek (Rect, better types...) - OK
   - Smooth shapes (rounded rectangles...) - OK
   - Extra info (photo date, etc...) - OK ?

- Configuration
   - Transitions
   - Durations - OK
   - Search query - OK
   - Search by person name - OK

- Graphics:
   - Info placement
   - Zoom effect -> check for background worker first! (otherwise it will be freezed...)
   - Other effects / transitions
   - Other layouts when possible (two photos at the same time)
   - Configurable orientation

- Technical:
   - Compute view matrix once
   - Change from reqwest to a more lightweight http client - OK
   - Error handling
   - Sleep when nothing to do ?
   - Background loading / blur with GPU (context sharing)

- HTTP Api ?
- publish

- bugs:
   - transition blur background from some to none - OK
- refactos:
   - graphic module - OK

Improves:
- GBM ressources:
   - Ask Gemini
   - https://docs.kernel.org/gpu/drm-kms.html
   - https://github.com/ds-hwang/gbm_es2_demo/blob/master/rust/examples/opengl_egl_drm.rs#L275
