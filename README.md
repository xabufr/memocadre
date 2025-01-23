Todo:

- switch to glow -> glium bugs (texture sampling) OK
   - Text - glyph branch -> try epaint OK
   - DRM/GDM/KMS rendering - OK
   - switch to Vek (Rect, better types...) - OK
   - Smooth shapes (rounded rectangles...)
   - Extra info (photo date, etc...)

- Configuration
   - Transitions
   - Durations - OK
   - Search query - OK
   - Search by person name - OK

- Change from reqwest to a more lightweight http client
- Error handling
- Sleep when nothing to do
- Other effects / transitions
- Other layouts when possible (two photos at the same time)
- Configurable orientation
- HTTP Api ?
- publish
- Background loading / blur with GPU (context sharing)

- bugs:
   - transition blur background from some to none
- refactos:
   - graphic module

Improves:
- GBM ressources:
   - Ask Gemini
   - https://docs.kernel.org/gpu/drm-kms.html
   - https://github.com/ds-hwang/gbm_es2_demo/blob/master/rust/examples/opengl_egl_drm.rs#L275
