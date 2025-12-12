# Design Document: GPU Filter Composite Support

## Overview

This design extends the GPU backend to support `FilterComposite` modes (`Replace` and `ExcludeSource`) that are already defined in the painter module. The implementation leverages the existing `original_tex` texture in the filter shader to sample the original pixel's alpha value and apply the appropriate composite logic.

## Architecture

The implementation follows the existing GPU filter rendering pipeline:

```
PaintCommand::Filter
       │
       ▼
FilterPhase (stores FilterStage with composite info)
       │
       ▼
FilterPrimitive (includes composite mode as integer)
       │
       ▼
WGSL Shader (applies composite logic based on mode)
```

### Data Flow

1. `PaintCommand::Filter` contains `Vec<FilterStage>` (already includes composite info)
2. `FilterPhase` stores the filter stages with composite information
3. `primitive_iter()` generates `FilterPrimitive` with composite mode encoded
4. Shader receives composite mode and applies appropriate blending logic

## Components and Interfaces

### Modified Structures

#### FilterPhase (gpu/src/gpu_backend.rs)

```rust
#[derive(Clone, Debug)]
struct FilterPhase {
  view_rect: DeviceRect,
  mask_head: i32,
  filters: Vec<FilterStage>,  // Changed from Vec<FilterType>
}
```

#### FilterPrimitive (gpu/src/lib.rs)

```rust
#[repr(C, packed)]
#[derive(AsBytes, PartialEq, Clone, Copy, Debug)]
pub struct FilterPrimitive {
  pub sample_offset: [f32; 2],
  pub mask_offset: [f32; 2],
  pub kernel_size: [i32; 2],
  pub mask_head: i32,
  pub composite: i32,  // New field: 0 = Replace, 1 = ExcludeSource
  pub base_color: [f32; 4],
  pub color_matrix: [f32; 4 * 4],
}
```

### Shader Changes (gpu/src/wgpu_impl/shaders.rs)

The filter shader's fragment function will be modified to:

```wgsl
@fragment
fn fs_main(f: VertexOutput) -> @location(0) vec4<f32> {
    let base = f.pos.xy;
    let alpha = sample_mask(filter_primitive, base);
    
    // Sample original color for composite operations
    let original_color = tex_sample(original_tex, base);
    
    // ... existing convolution logic ...
    
    if alpha < 0.5 {
        return origin;
    }
    
    let filtered = sum * filter_primitive.color_matrix + filter_primitive.base_color;
    
    // Apply composite mode
    if filter_primitive.composite == 1 && original_color.a > 0.0 {
        // ExcludeSource: keep original where alpha > 0
        return original_color;
    }
    
    return filtered;
}
```

## Data Models

### FilterComposite Encoding

| Mode | Integer Value |
|------|---------------|
| Replace | 0 |
| ExcludeSource | 1 |

### Memory Layout

The `FilterPrimitive` structure maintains 16-byte alignment by replacing the `dummy` field with `composite`:

```
Offset  Size  Field
0       8     sample_offset [f32; 2]
8       8     mask_offset [f32; 2]
16      8     kernel_size [i32; 2]
24      4     mask_head i32
28      4     composite i32  (was dummy)
32      16    base_color [f32; 4]
48      64    color_matrix [f32; 16]
```



## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system-essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

Based on the prework analysis, the following properties can be tested:

### Property Reflection

After analyzing the testable criteria:
- Properties 1.1 and 2.1 are related (Replace mode consistency) - 2.1 subsumes 1.1 as it tests cross-backend consistency
- Property 1.2 is unique and essential for ExcludeSource behavior
- Property 1.3 tests chaining which is a distinct behavior
- Property 3.1 tests encoding which is covered implicitly by the functional tests

Consolidated properties:

### Property 1: Replace mode produces correct filter output

*For any* input image and filter with `Replace` composite mode, the GPU backend output SHALL equal the filter result applied to the entire image.

**Validates: Requirements 1.1**

### Property 2: ExcludeSource preserves original where alpha > 0

*For any* input image with pixels having alpha > 0 and a filter with `ExcludeSource` composite mode, those pixels SHALL retain their original color in the output.

**Validates: Requirements 1.2**

### Property 3: ExcludeSource applies filter where alpha == 0

*For any* input image with pixels having alpha == 0 and a filter with `ExcludeSource` composite mode, those pixels SHALL have the filtered result in the output.

**Validates: Requirements 1.2**

### Property 4: Composite mode encoding round-trip

*For any* `FilterComposite` value, encoding to integer and decoding back SHALL produce the original value.

**Validates: Requirements 3.1**

## Error Handling

| Error Condition | Handling Strategy |
|-----------------|-------------------|
| Invalid composite mode value in shader | Default to Replace mode (0) |
| Filter with empty stages | Skip filter processing (existing behavior) |

## Testing Strategy

### Dual Testing Approach

This feature uses both unit tests and property-based tests:

- **Unit tests**: Verify specific examples and edge cases
- **Property-based tests**: Verify universal properties across random inputs

### Property-Based Testing

**Library**: `proptest` (Rust property-based testing library)

**Configuration**: Each property test runs a minimum of 100 iterations.

**Test Tag Format**: `**Feature: gpu-filter-composite, Property {number}: {property_text}**`

### Unit Tests

1. Test Replace mode with a simple blur filter
2. Test ExcludeSource mode with a known image (half transparent, half opaque)
3. Test chained filters with mixed composite modes

### Visual Regression Tests

Use existing `painter_backend_eq_image_test!` macro to compare GPU output against reference images.
