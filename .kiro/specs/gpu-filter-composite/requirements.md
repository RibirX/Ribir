# Requirements Document

## Introduction

This feature extends the GPU backend to support the `FilterComposite` type that has already been implemented in the painter module. The `FilterComposite` enum defines how filter results are composited with the original source content. Currently, the GPU backend only supports the default `Replace` behavior, where the filter result completely replaces the original content. This feature adds support for the `ExcludeSource` composite mode, which applies the filter result only to areas where the source alpha is 0.

The GPU filter shader already has access to the original texture (`original_tex`), which enables sampling the original pixel's alpha value to implement the `ExcludeSource` composite mode.

## Glossary

- **FilterComposite**: An enum defining how filter results are combined with original content. Values include `Replace` (default) and `ExcludeSource`.
- **ExcludeSource**: A composite mode where the filter result is only applied to areas where the original source alpha is 0.
- **FilterStage**: A single step in a filter chain containing the filter type and composite operator.
- **FilterPrimitive**: The GPU-side data structure representing filter parameters passed to shaders.
- **WGSL**: WebGPU Shading Language used for GPU shader programs.
- **original_tex**: The texture containing the original content before filter application, already available in the filter shader.

## Requirements

### Requirement 1

**User Story:** As a developer using Ribir, I want to apply filters with different composite modes on the GPU, so that I can create visual effects like outer glows that only affect transparent areas.

#### Acceptance Criteria

1. WHEN a filter with `FilterComposite::Replace` is applied THEN the GPU backend SHALL replace the original content with the filter result completely.
2. WHEN a filter with `FilterComposite::ExcludeSource` is applied THEN the GPU backend SHALL apply the filter result only to pixels where the original source alpha is 0.
3. WHEN multiple filter stages with different composite modes are chained THEN the GPU backend SHALL apply each stage's composite mode independently in sequence.

### Requirement 2

**User Story:** As a developer, I want the GPU filter composite implementation to be consistent with the painter's filter composite behavior, so that rendering results are predictable across different backends.

#### Acceptance Criteria

1. WHEN a filter with `ExcludeSource` composite is rendered on GPU THEN the system SHALL produce visually equivalent results to the painter's software implementation.

### Requirement 3

**User Story:** As a developer, I want the filter composite mode to be efficiently passed to the GPU shader, so that rendering performance is not significantly impacted.

#### Acceptance Criteria

1. WHEN the composite mode is passed to the GPU THEN the system SHALL encode the composite mode as an integer field in the FilterPrimitive structure.
2. WHEN the shader processes the composite mode THEN the system SHALL sample the original texture alpha and use conditional logic based on the composite mode value.
3. WHEN rendering filters with `Replace` mode THEN the system SHALL maintain the existing filter rendering performance.
