# Design Document: Drop-Shadow Filter with Offset Support

## Overview

本设计为 Ribir 的 Filter 系统添加 offset（偏移）能力，并基于此实现 drop-shadow 效果。

**核心思路：**
1. 在 `FilterStage` 中添加 `offset` 字段
2. 修改 shader 逻辑，正确处理 offset 和 composite 的关系
3. 在 `Filter` API 中添加设置 offset 的方法

**Drop-shadow 实现：**
```rust
Filter::drop_shadow(dx, dy, blur_radius, shadow_color) = 
  Filter::color(shadow_color_matrix)  // 转换为阴影颜色
    .with(Filter::blur(blur_radius))  // 模糊
    .offset(dx, dy)                   // 偏移
    .composite_op(ExcludeSource)      // 排除源
```

## Architecture

```
FilterStage (painter/src/filter.rs)
    │ 新增 offset: [f32; 2] 字段
    │
    ▼
FilterPhase::primitive_iter() (gpu/src/gpu_backend.rs)
    │ 将 FilterStage.offset 传递给 FilterPrimitive.offset（新字段）
    │ sample_offset 保持原有用途（定位原图在 texture 中的位置）
    │
    ▼
FilterPrimitive (gpu/src/lib.rs)
    │ sample_offset: 定位原图在 texture 中的位置（保留原有用途）
    │ offset: 新增字段，用于 filter 效果回写时的偏移
    │
    ▼
WGSL Shader (gpu/src/wgpu_impl/shaders.rs)
    │ 修改逻辑：
    │ - 原图采样使用 sample_offset（定位到 texture 中的正确区域）
    │ - filter 效果偏移使用新的 offset 字段
    │ - composite 判断在当前位置采样原图
    │
    ▼
渲染结果：偏移后的 filter 效果 + 正确的 composite
```

### 关键概念区分

**sample_offset vs offset：**

| 字段 | 用途 | 说明 |
|------|------|------|
| `sample_offset` | 定位原图在 texture 中的位置 | 原图被放到 texture 的某个位置，采样时需要加上这个偏移才能读取到正确的像素 |
| `offset` | filter 效果的偏移 | 用于 drop-shadow 等效果，表示 filter 结果相对于原图的位移 |

### Shader 逻辑详解

**修正后的逻辑：**
```wgsl
@fragment
fn fs_main(f: VertexOutput) -> @location(0) vec4<f32> {
  let base = f.pos.xy;
  
  // 1. Composite 判断：在当前位置采样原图
  //    使用 sample_offset 定位到 texture 中的正确区域
  let original_color = tex_sample(original_tex, base + filter_primitive.sample_offset);
  
  // 2. Filter 采样：使用 sample_offset + offset
  //    - sample_offset: 定位到 texture 中的原图区域
  //    - offset: filter 效果的偏移（如 drop-shadow 的 dx, dy）
  for (卷积循环) {
    let sample_pos = pos + filter_primitive.sample_offset + filter_primitive.offset;
    let color = tex_sample(original_tex, sample_pos);
    // ... 卷积计算 ...
  }
  
  // 3. Composite 逻辑
  if composite == ExcludeSource && original_color.a > 0.0 {
    return original_color;  // 原图有内容的地方显示原图
  }
  return filtered;  // 否则显示偏移后的 filter 结果（阴影）
}
```

**效果：**
- 在原图有内容的位置 → 显示原图
- 在原图透明的位置 → 显示偏移后的模糊阴影

## Components and Interfaces

### 1. FilterStage 修改 (painter/src/filter.rs)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterStage {
  pub typ: FilterType,
  pub composite: FilterComposite,
  pub offset: [f32; 2],  // 新增：采样偏移 [dx, dy]
}
```

### 2. Filter API 扩展 (painter/src/filter.rs)

```rust
impl Filter {
  /// 设置最后一个 filter stage 的偏移量
  pub fn offset(mut self, dx: f32, dy: f32) -> Self {
    if let Some(stage) = self.stages.last_mut() {
      stage.offset = [dx, dy];
    }
    self
  }

  /// 创建 drop-shadow filter
  /// - offset: 阴影偏移 (dx, dy)
  /// - blur_radius: 模糊半径
  /// - shadow_color: 阴影颜色
  pub fn drop_shadow(offset: (f32, f32), blur_radius: f32, shadow_color: Color) -> Self {
    // 1. 创建阴影颜色矩阵
    let shadow_matrix = shadow_color_matrix(shadow_color);
    
    // 2. 组合 filter stages
    Filter::color(shadow_matrix)
      .with(Filter::blur(blur_radius))
      .offset(offset.0, offset.1)
      .composite_op(FilterComposite::ExcludeSource)
  }
}

/// 创建将任意颜色转换为指定阴影颜色的矩阵
fn shadow_color_matrix(color: Color) -> ColorFilterMatrix {
  let [r, g, b, a] = color.into_f32_components();
  ColorFilterMatrix {
    matrix: [
      0.0, 0.0, 0.0, 0.0,  // R: 忽略输入
      0.0, 0.0, 0.0, 0.0,  // G: 忽略输入  
      0.0, 0.0, 0.0, 0.0,  // B: 忽略输入
      0.0, 0.0, 0.0, a,    // A: 保留输入 alpha × 阴影 alpha
    ],
    base_color: Some(Color::from_f32_rgba(r, g, b, 0.0)),
  }
}
```

### 3. Painter 层应用 Transform 到 Offset (painter/src/painter.rs)

**关键：在 painter 生成 filter 命令时就将 offset 应用 transform**

offset 是在用户坐标系中定义的（如 `drop_shadow((5.0, 5.0), ...)`），但 GPU 渲染是在设备坐标系中进行的。在 painter 生成 `PaintCommand::Filter` 时，就将 offset 转换到设备坐标系，这样 GPU backend 可以无感知地直接使用。

```rust
// painter/src/painter.rs - filter_path 方法
pub fn filter_path(&mut self, path: PaintPath, filter: Filter) -> &mut Self {
  // ... 现有代码 ...
  
  let transform = *self.transform();
  let path_bounds = transform.outer_transformed_rect(&p_bounds);
  
  // 将 filter stages 中的 offset 应用 transform
  let filters: Vec<FilterStage> = filter
    .into_vec()
    .into_iter()
    .map(|mut stage| {
      // 使用 transform_vector 将 offset 从用户坐标系转换到设备坐标系
      let offset = transform.transform_vector(Vector2D::new(stage.offset[0], stage.offset[1]));
      stage.offset = [offset.x, offset.y];
      stage
    })
    .collect();
  
  self.commands.push(PaintCommand::Filter { path, path_bounds, transform, filters });
  self
}
```

**注意：** 使用 `transform_vector` 而不是 `transform_point`，因为 offset 是一个方向向量，不应该受到平移的影响。

### 4. GPU Backend 修改 (gpu/src/gpu_backend.rs)

GPU backend 需要区分 `sample_offset` 和 `offset`：

```rust
fn primitive_iter(&self) -> impl Iterator<Item = (FilterPrimitive, Vec<f32>)> + '_ {
  // ... 现有代码 ...
  
  Some((
    FilterPrimitive {
      sample_offset: [0.; 2],  // 保留原有用途：定位原图在 texture 中的位置
                               // 当前 filter 实现中原图就在 (0,0) 位置，所以是 [0, 0]
      offset: filter.offset,   // 新增：filter 效果的偏移（已在 painter 层转换过）
      mask_offset: [0.; 2],
      composite,
      color_matrix,
      base_color: base_color.map_or_else(|| [0.; 4], |c| c.into_f32_components()),
      kernel_size: [width as i32, height as i32],
      mask_head: self.mask_head,
    },
    matrix,
  ))
}
```

### 5. FilterPrimitive 结构修改 (gpu/src/lib.rs)

新增 `offset` 字段：

```rust
#[repr(C, packed)]
#[derive(AsBytes, PartialEq, Clone, Copy, Debug)]
pub struct FilterPrimitive {
  /// The origin of the image placed in texture.
  /// 用于定位原图在 texture 中的位置
  pub sample_offset: [f32; 2],
  /// Filter effect offset for drop-shadow etc.
  /// 用于 filter 效果的偏移（如 drop-shadow 的 dx, dy）
  pub offset: [f32; 2],  // 新增
  /// The origin of the mask layer in the texture.
  pub mask_offset: [f32; 2],
  // ... 其他字段保持不变 ...
}
```

### 6. Shader 修改 (gpu/src/wgpu_impl/shaders.rs)

**关键修改：**
1. FilterPrimitive 结构新增 `offset` 字段
2. 原图采样使用 `sample_offset`（定位 texture 中的原图）
3. filter 卷积采样使用 `sample_offset + offset`（定位 + 偏移）

```wgsl
struct FilterPrimitive {
  /// The origin of the image placed in texture.
  sample_offset: vec2<f32>,
  /// Filter effect offset for drop-shadow etc.
  offset: vec2<f32>,  // 新增
  /// The origin of the mask layer in the texture.
  mask_offset: vec2<f32>,
  // ... 其他字段 ...
}

@fragment
fn fs_main(f: VertexOutput) -> @location(0) vec4<f32> {
  let base = f.pos.xy;
  let alpha = sample_mask(filter_primitive, base);
  
  // Composite 判断：在当前位置采样原图
  // 使用 sample_offset 定位到 texture 中的正确区域
  let original_color = tex_sample(original_tex, base + filter_primitive.sample_offset);
  
  let kernel_size = filter_primitive.kernel_size;
  let x_radius = f32(kernel_size.x >> 1);
  let y_radius = f32(kernel_size.y >> 1);
  var origin = vec4<f32>(0., 0., 0., 0.);
  var sum = vec4<f32>(0., 0., 0., 0.);
  
  for (var i: u32 = 0; i < kernel_size.x; i++) {
    for (var j: u32 = 0; j < kernel_size.y; j++) {
      let pos = base + vec2<f32>(f32(i) - x_radius, f32(j) - y_radius);
      let index = j * kernel_size.x + i;
      let weight = filter_primitive.kernel_matrix[index / 4][index % 4];

      // Filter 采样：sample_offset（定位原图）+ offset（效果偏移）
      let sample_pos = pos + filter_primitive.sample_offset + filter_primitive.offset;
      let color = tex_sample(original_tex, sample_pos);
      
      if (i == u32(x_radius) && j == u32(y_radius)) {
        origin = color;
      }
      sum = sum + (color * weight);
    }
  }
  
  if alpha < 0.5 {
    return origin;
  }
  
  let filtered = sum * filter_primitive.color_matrix + filter_primitive.base_color;
  
  // Composite 逻辑：使用当前位置的 original_color 判断
  if filter_primitive.composite == 1 && original_color.a > 0.0 {
    return original_color;
  }
  
  return filtered;
}
```

## Data Models

### FilterStage 结构 (painter 层)

| 字段 | 类型 | 描述 |
|------|------|------|
| typ | FilterType | Filter 类型（Color 或 Convolution） |
| composite | FilterComposite | 合成模式（Replace 或 ExcludeSource） |
| offset | [f32; 2] | filter 效果偏移 [dx, dy]，默认 [0, 0] |

### FilterPrimitive 结构 (GPU 层)

| 字段 | 类型 | 描述 |
|------|------|------|
| sample_offset | [f32; 2] | 定位原图在 texture 中的位置（保留原有用途） |
| offset | [f32; 2] | filter 效果偏移 [dx, dy]（新增） |
| mask_offset | [f32; 2] | mask 层在 texture 中的偏移 |
| kernel_size | [i32; 2] | 卷积核大小 |
| mask_head | i32 | mask 层链表头索引 |
| composite | i32 | 合成模式 |
| base_color | [f32; 4] | 基础颜色 |
| color_matrix | [f32; 16] | 颜色矩阵 |

### Offset 语义

- `offset = [dx, dy]` 表示阴影向右下偏移
- 在 shader 中，对于输出像素 (x, y)，filter 采样位置为 (x + sample_offset.x + dx, y + sample_offset.y + dy)
- `sample_offset` 用于定位原图在 texture 中的位置
- `offset` 用于 filter 效果的偏移（如 drop-shadow）
- 正的 offset 会让 filter 效果向右下移动，产生右下方向的阴影效果

## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system-essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*

### Property Reflection

分析 prework 中的可测试属性：
- 1.1 和 4.2 都测试 offset 传递，可以合并为一个属性
- 1.3 测试 offset 与 blur 组合，是独立的属性
- 1.4 测试零 offset 兼容性，是独立的属性
- 2.1 和 2.2 测试 drop-shadow 组合，可以合并
- 3.2 测试 offset API，是独立的属性
- 3.3 测试序列化 round-trip，是独立的属性

### Property 1: Offset 正确传递到 FilterPrimitive

*For any* FilterStage with offset [dx, dy], when converted to FilterPrimitive, the `offset` field (not `sample_offset`) SHALL equal [dx, dy].

**Validates: Requirements 1.1, 4.2**

### Property 2: Offset 与 Blur 组合正确

*For any* Filter combining blur and offset, the resulting FilterPrimitive SHALL contain both the convolution kernel and the offset value.

**Validates: Requirements 1.3**

### Property 3: 零 Offset 保持兼容性

*For any* FilterStage with offset [0, 0], the filter behavior SHALL be identical to a FilterStage without offset field.

**Validates: Requirements 1.4**

### Property 4: Drop-shadow 组合正确

*For any* drop-shadow filter with parameters (dx, dy, blur_radius, color), the resulting Filter SHALL contain:
1. A color filter stage converting to shadow color
2. A blur filter stage (if blur_radius > 0)
3. The last stage with offset [dx, dy] and ExcludeSource composite

**Validates: Requirements 2.1, 2.2**

### Property 5: Offset API 设置正确

*For any* Filter, calling `.offset(dx, dy)` SHALL set the offset of the last filter stage to [dx, dy].

**Validates: Requirements 3.2**

### Property 6: FilterStage 序列化 Round-trip

*For any* FilterStage with offset, serializing then deserializing SHALL produce an equivalent FilterStage.

**Validates: Requirements 3.3**

## Error Handling

| 错误条件 | 处理策略 |
|----------|----------|
| offset 导致采样越界 | Shader 返回透明像素（现有 textureSampleLevel 行为） |
| 空 Filter 调用 offset() | 无操作，返回原 Filter |
| blur_radius <= 0 | 跳过 blur stage，只应用颜色和偏移 |

## Testing Strategy

### Property-Based Testing

**Library**: `proptest` (Rust property-based testing library)

**Configuration**: 每个 property test 运行至少 100 次迭代。

**Test Tag Format**: `**Feature: drop-shadow-filter, Property {number}: {property_text}**`

### Unit Tests

1. 测试 `Filter::offset()` API 设置 offset
2. 测试 `Filter::drop_shadow()` 生成正确的 filter stages
3. 测试 FilterStage 序列化/反序列化

### Visual Regression Tests

使用现有的 `painter_backend_eq_image_test!` 宏进行视觉回归测试：
1. 测试带 offset 的 blur filter
2. 测试 drop-shadow 效果
3. 测试 drop-shadow 与其他 filter 组合
