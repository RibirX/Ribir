# Canvas

`Canvas` is a vector font and 2d graphic library, the main goal is as a fast render for `Ribir`.


## Design

`Canvas` accept the `PaintCommand` as input, and its work split as two phase, the cpu phase and gpu phase.

**In the cpu phase**, everything painted will be generate to render data in memory. Render data has two part, the triangles and texture. Geometry and texts will convert to triangles, the color and image will store in an atlas texture, and text glyphs store in glyphs texture. These output is easy to use for rendering engine. And can be cached in your application.

**In the gpu phase**, use a rendering engine to submit the output of `cpu phase` to gpu. `Canvas` provide a `wgpu` render as default.

`Canvas` try to batch all painting thing before a `submit` called. If the atlas texture or glyphs texture is fulled, `Canvas` will split to smaller pieces data then submit to `render`, or failed without a `render`. 

If a new submit isn't use new colors, images and glyphs, that means texture not change. `Canvas` will use the textures already in gpu, and not update glyphs texture or atlas texture.
