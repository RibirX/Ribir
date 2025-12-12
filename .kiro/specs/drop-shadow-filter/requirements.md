# Requirements Document

## Introduction

本功能为 Ribir 的 Filter 系统添加 offset（偏移）能力，并基于此实现 drop-shadow（投影阴影）效果。

核心思路：
1. 在 `FilterPrimitive` 中添加 `offset` 参数，用于 filter 时的采样偏移
2. Drop-shadow 通过组合现有能力实现：blur（模糊）+ offset（偏移）+ ExcludeSource（排除源合成）

现有系统已具备 blur 和 ExcludeSource composite 能力，只需添加 offset 能力即可实现 drop-shadow。

## Glossary

- **Drop-Shadow**: 一种视觉效果，在源图像后面创建一个模糊的、偏移的阴影副本
- **Offset**: Filter 采样时的 X/Y 方向位移量，正值表示阴影向右下偏移
- **Blur Radius**: 阴影的模糊半径，控制阴影的柔和程度
- **FilterComposite::ExcludeSource**: 现有的合成模式，将 filter 结果只应用到源 alpha=0 的区域
- **FilterStage**: Filter 链中的单个步骤，包含 filter 类型、合成操作符和偏移量
- **FilterPrimitive**: GPU 端的 filter 参数数据结构，传递给 shader

## Requirements

### Requirement 1

**User Story:** As a developer, I want the filter system to support sample offset, so that drop-shadow and other offset-based effects can be implemented.

#### Acceptance Criteria

1. WHEN a filter stage specifies an offset (dx, dy) THEN the GPU shader SHALL sample the source texture at position (x - dx, y - dy) for each output pixel at (x, y).
2. WHEN the offset causes sampling outside texture bounds THEN the system SHALL return transparent pixels (alpha = 0).
3. WHEN offset is combined with convolution filters (blur) THEN the system SHALL apply the offset to the convolution sampling.
4. WHEN offset is zero THEN the system SHALL maintain existing filter behavior without performance impact.

### Requirement 2

**User Story:** As a developer using Ribir, I want to apply a drop-shadow effect to widgets, so that I can create depth and visual hierarchy in my UI.

#### Acceptance Criteria

1. WHEN a drop-shadow filter is applied with offset (dx, dy) and blur radius THEN the GPU backend SHALL render a blurred shadow displaced by (dx, dy) pixels from the source.
2. WHEN a drop-shadow filter is applied THEN the GPU backend SHALL render the original source content on top of the shadow (using ExcludeSource composite).
3. WHEN a drop-shadow filter is created with zero blur radius THEN the system SHALL render a sharp shadow without blur.
4. WHEN a drop-shadow filter is created with zero offset THEN the system SHALL render the shadow directly behind the source.

### Requirement 3

**User Story:** As a developer, I want a convenient API to create drop-shadow filters, so that I can easily add shadow effects without manually composing multiple filter stages.

#### Acceptance Criteria

1. WHEN creating a drop-shadow filter THEN the system SHALL provide a `Filter::drop_shadow(offset_x, offset_y, blur_radius, color)` constructor method.
2. WHEN setting offset on a filter THEN the system SHALL provide a `Filter::offset(dx, dy)` method that sets the offset on the last filter stage.
3. WHEN the drop-shadow filter is serialized and deserialized THEN the system SHALL preserve all parameters (offset, blur radius, color) for round-trip consistency.

### Requirement 4

**User Story:** As a developer, I want the drop-shadow implementation to be efficient, so that rendering performance is acceptable for interactive UIs.

#### Acceptance Criteria

1. WHEN rendering drop-shadow THEN the system SHALL reuse existing filter infrastructure (blur, composite) without additional texture copies.
2. WHEN the offset parameter is passed to GPU THEN the system SHALL encode it in the existing FilterPrimitive structure.
3. WHEN filters without offset are rendered THEN the system SHALL maintain existing rendering performance.

