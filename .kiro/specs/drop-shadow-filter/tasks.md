# Implementation Plan

- [x] 1. 扩展 FilterStage 添加 offset 字段





  - [x] 1.1 在 FilterStage 结构体中添加 `offset: [f32; 2]` 字段


    - 修改 `painter/src/filter.rs` 中的 `FilterStage` 结构体
    - 添加 `#[serde(default)]` 确保向后兼容
    - _Requirements: 1.1, 3.3_

  - [x] 1.2 更新所有创建 FilterStage 的地方，初始化 offset 为 [0., 0.]

    - 更新 `Filter::grayscale`, `Filter::blur` 等方法
    - _Requirements: 1.4_


  - [x] 1.3 添加 `Filter::offset(dx, dy)` 方法





    - 设置最后一个 stage 的 offset 值
    - _Requirements: 3.2_
  - [ ]* 1.4 编写 property test: offset 设置正确
    - **Property 5: Offset API 设置正确**
    - **Validates: Requirements 3.2**

- [-] 2. 在 Painter 层应用 Transform 到 Offset




  - [ ] 2.1 修改 `Painter::filter_path()` 方法

    - 在生成 `PaintCommand::Filter` 前，将 filter stages 中的 offset 应用 transform
    - 使用 `transform.transform_vector()` 转换 offset
    - 修改 `painter/src/painter.rs`
    - _Requirements: 1.1, 4.2_
  - [ ]* 2.2 编写 property test: offset 正确传递到 FilterPrimitive
    - **Property 1: Offset 正确传递到 FilterPrimitive**
    - **Validates: Requirements 1.1, 4.2**







- [x] 3. 修改 GPU 层支持新的 offset 字段







  - [x] 3.1 修改 FilterPrimitive 结构体添加 offset 字段

    - 在 `gpu/src/lib.rs` 中的 `FilterPrimitive` 结构体添加 `pub offset: [f32; 2]`
    - 保留 `sample_offset` 用于定位原图在 texture 中的位置
    - 新增 `offset` 用于 filter 效果的偏移
    - _Requirements: 1.1, 4.2_

  - [x] 3.2 修改 GPU backend 的 primitive_iter() 方法


    - 在 `gpu/src/gpu_backend.rs` 中更新 `FilterPhase::primitive_iter()`
    - `sample_offset` 保持 [0., 0.]（当前实现中原图在 texture 的 (0,0) 位置）
    - 将 `FilterStage.offset` 传递给 `FilterPrimitive.offset`
    - _Requirements: 1.1, 4.2_


  - [x] 3.3 修改 filter shader 支持新的 offset 字段

    - 在 `gpu/src/wgpu_impl/shaders.rs` 中更新 FilterPrimitive 结构
    - 原图采样使用 `sample_offset`（定位 texture 中的原图）
    - filter 卷积采样使用 `sample_offset + offset`（定位 + 效果偏移）
    - composite 判断使用 `sample_offset` 采样原图
    - _Requirements: 1.1, 2.2_

- [ ]* 4. Checkpoint - 确保基础 offset 功能正常
  - Ensure all tests pass, ask the user if questions arise.

- [x] 5. 实现 drop-shadow API

  - [x] 5.1 添加 `shadow_color_matrix()` 辅助函数
    - 创建将任意颜色转换为阴影颜色的 ColorFilterMatrix
    - _Requirements: 2.1_
  - [x] 5.2 添加 `Filter::drop_shadow()` 构造方法
    - 组合 color filter + blur + offset + ExcludeSource
    - _Requirements: 2.1, 2.2, 3.1_
  - [ ]* 5.3 编写 property test: drop-shadow 组合正确
    - **Property 4: Drop-shadow 组合正确**
    - **Validates: Requirements 2.1, 2.2**

- [ ]* 6. 添加序列化测试
  - [ ]* 6.1 编写 property test: FilterStage 序列化 round-trip
    - **Property 6: FilterStage 序列化 Round-trip**
    - **Validates: Requirements 3.3**

- [ ]* 7. 添加视觉回归测试
  - [ ]* 7.1 添加 offset blur filter 的视觉测试
    - 使用 `painter_backend_eq_image_test!` 宏
    - _Requirements: 1.1, 1.3_
  - [ ]* 7.2 添加 drop-shadow 效果的视觉测试
    - 测试不同 offset、blur radius、shadow color 组合
    - _Requirements: 2.1, 2.2, 2.3, 2.4_

- [ ]* 8. Final Checkpoint - 确保所有测试通过
  - Ensure all tests pass, ask the user if questions arise.
