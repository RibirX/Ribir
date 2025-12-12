# Implementation Plan

- [ ] 1. Update FilterPrimitive structure to include composite mode
  - [x] 1.1 Modify FilterPrimitive in gpu/src/lib.rs





    - Replace `dummy: i32` field with `composite: i32`
    - Add documentation for the composite field (0 = Replace, 1 = ExcludeSource)
    - _Requirements: 3.1_
  - [ ]* 1.2 Write property test for composite mode encoding
    - **Property 4: Composite mode encoding round-trip**
    - **Validates: Requirements 3.1**

- [x] 2. Update FilterPhase to store FilterStage instead of FilterType





  - [x] 2.1 Modify FilterPhase struct in gpu/src/gpu_backend.rs


    - Change `filters: Vec<FilterType>` to `filters: Vec<FilterStage>`
    - Update the import to include `FilterStage` and `FilterComposite` from ribir_painter
    - _Requirements: 1.1, 1.2, 1.3_


  - [ ] 2.2 Update primitive_iter() to extract composite mode



    - Modify the iterator to read composite mode from FilterStage
    - Encode FilterComposite::Replace as 0, FilterComposite::ExcludeSource as 1
    - Set the composite field in FilterPrimitive
    - _Requirements: 1.1, 1.2, 3.1_
  - [x] 2.3 Update PaintCommand::Filter handling in draw_command()
    - Ensure filters are passed as Vec<FilterStage> to FilterPhase
    - _Requirements: 1.3_

- [x] 3. Update filter shader to handle composite modes





  - [x] 3.1 Modify FilterPrimitive struct in WGSL shader


    - Add `composite: i32` field in the shader struct (replacing dummy)
    - Ensure alignment matches the Rust struct
    - _Requirements: 3.1_
  - [x] 3.2 Implement composite logic in fragment shader

    - Sample original texture color at current position
    - For ExcludeSource mode (composite == 1): return original color if original alpha > 0
    - For Replace mode (composite == 0): return filtered result (existing behavior)
    - _Requirements: 1.1, 1.2_

- [ ] 4. Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.

- [ ]* 5. Add visual regression tests
  - [ ]* 5.1 Create test for Replace mode filter
    - Use existing painter_backend_eq_image_test! macro
    - Apply a blur filter with Replace mode and verify output
    - _Requirements: 1.1_
  - [ ]* 5.2 Create test for ExcludeSource mode filter
    - Create test image with mixed alpha values
    - Apply blur filter with ExcludeSource mode
    - Verify opaque pixels retain original color, transparent pixels get blur
    - **Property 2: ExcludeSource preserves original where alpha > 0**
    - **Property 3: ExcludeSource applies filter where alpha == 0**
    - **Validates: Requirements 1.2**

- [ ] 6. Final Checkpoint - Ensure all tests pass
  - Ensure all tests pass, ask the user if questions arise.
