Todo:

- switch to glow -> No benefits
   - Text - glyph branch -> try epaint
   - DRM/GDM/KMS rendering - OK
   - Smooth shapes (rounded rectangles...)
   - Extra info (photo date, etc...)
- Configuration
   - Transitions
   - Durations
   - Search query
   - Search by person name
- Error handling
- Sleep when nothing to do
- Other effects / transitions
- Other layouts when possible (two photos at the same time)
- Configurable orientation
- HTTP Api ?
- publish

- bugs:
   - transition blur background from some to none
- refactos:
   - graphic module

Improves:
- GBM ressources:
   - Ask Gemini
   - https://docs.kernel.org/gpu/drm-kms.html
   - https://github.com/ds-hwang/gbm_es2_demo/blob/master/rust/examples/opengl_egl_drm.rs#L275
