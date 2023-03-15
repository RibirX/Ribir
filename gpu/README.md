# ribir_gpu

`ribir_gpu` is a gpu painter backend for `ribir_painter`. `ribir_gpu` accept `PaintCommand` as input and convert them to triangles and texture then submit to the graphics library to draw.

## Tessellator: generate triangles and texture

There are some awesome library we use to generate triangles.

- use `ttf-parser` to parse font
- use `rustybuzz` to shape text.
- use `lyon` to generate triangles for path.
## Replace graphics library

`wgpu` is provide as the default graphics library to render the result of `Tessellator`. Enable is across the `wgpu_gl` feature. 

## Cache mechanism

Vertex generation is cached pre `PaintCommand` and retain if it cache hit by next frame.

For text, a high level cache may have, the glyph is may cached as an alpha mode bitmap in atlas texture. For all cache miss glyphs, tessellator will count the duplicate glyphs pre frame, and generate their vertexes once and draw them in atlas texture convert to new vertexes.

## Parallel and not serial guarantee

Notice `ribir_gpu` is design as parallel render with ribir, so `PaintCommand` submit to `ribir_gpu` will not be processed immediately.

`ribir_painter` will processed the `PaintCommand` parallel and try to merge the result as much as possible in a max allowed memory before submit to gpu.

