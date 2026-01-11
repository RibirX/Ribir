# Ribir Interactive Widget Design Standard (v2.4)

This document defines the standard design paradigm for all interactive widgets in the Ribir framework.

## 1. Core Philosophy & Data Flow

### 1.1 The Golden Rule: Single Source of Truth
*   **Pipe is Authority**: The UI state is always a projection of the data source (Pipe).
*   **Strict Control**: User interactions trigger *events* (intent) but do not directly update the UI. The UI only updates when the Pipe emits a new value.

### 1.2 Data Flow Architecture
Ribir follows a strict unidirectional data flow with one specific exception for optimistic updates.

```
┌────────────────────────────────────────────────────────┐
│                     Model                              │
│                       ↑                                │
│                  Code Handler                          │
│                       ↑                                │
│               on_change(v)                             │
│          (only from user interaction)                  │
│                       │                                │
│  ┌────────────────────┴────────────────────────┐       │
│  │                Widget                        │      │
│  │                                              │      │
│  │  Path A: Pipe emit ──────→ UI updates        │      │
│  │  Path B: User interaction ──→ on_change      │      │
│  │  Path C: Direct write ─────→ UI updates      │      │
│  │                                              │      │
│  └──────────────────────────────────────────────┘      │
└────────────────────────────────────────────────────────┘
```

**The Three Paths:**
1.  **Path A (Standard Update)**: Data changes -> Pipe emits -> UI updates.
2.  **Path B (User Intent)**: User interacts -> `on_change` fires. **No UI change happens yet.**
3.  **Path C (Escape Hatch)**: Developer calls `$write(widget).val = x` -> UI updates immediately (bypassing Pipe). Used for optimistic UI.

> [!IMPORTANT]
> **Path C Usage Guidelines:**
> - **When to use**: Only for optimistic UI scenarios where you need instant feedback before async operations complete.
> - **Always reconcile**: After async completes, update the model so Pipe emits the confirmed value. This ensures eventual consistency.
> - **Don't abuse**: Overusing Path C leads to state desynchronization and debugging nightmares. Prefer Path A whenever possible.

---

## 2. Events & Interaction Models

Ribir distinguishes between "changing" a value and "submitting" it.

### 2.1 Event Definitions

| Event | Trigger | Behavior |
| :--- | :--- | :--- |
| **`on_change`** | User interaction (drag, type, click) | Fires **frequently** (every tick/keystroke). Represents live intent. |
| **`on_submit`** | Explicit commit (Enter, Blur) | Fires **once** on completion. Represents finalized data. |

### 2.2 Widget Categories

**A. Immediate Feedback Widgets**
*   **Examples**: `Slider`, `Checkbox`, `Switch`, `Tabs`.
*   **Behavior**: Value has meaning at every intermediate step.
*   **Primary Event**: `on_change`.

**B. Submit-Confirm Widgets**
*   **Examples**: `Input`, `TextArea`.
*   **Behavior**: Value is often in-progress until committed (e.g., typing a password).
*   **Events** (choose based on use case):
    - `on_submit`: For committing finalized data (form submission, search query).
    - `on_change`: For real-time feedback (live validation, search-as-you-type, password strength).

### 2.3 Rx Streams (Underlying Implementation)
Rx streams are the underlying implementation for all widget events. `on_change` and `on_submit` are convenience sugar built on top of these streams.

Accessing the raw stream enables advanced Rx operators like throttling and debouncing:
```rust
// on_change is sugar for:
slider.change_stream().subscribe(move |v| ...);
```

See [4.5 Throttling & Debouncing](#45-throttling--debouncing) for practical examples.

---

## 3. The Controlled Protocol

This protocol defines how controlled widgets synchronize with data.

### 3.1 The Cycle
1.  **User Action**: User drags Slider to `75`.
2.  **Event**: `on_change(75)` triggers. UI is *still* at old value.
3.  **Handler**: Developer logic runs (validates `75`, updates Model).
4.  **Data**: Model updates -> Pipe emits `75`.
5.  **Render**: UI updates to `75`.

> **Note**: This all happens in the **same frame**. To the user, it feels instant.

### 3.2 Rejection (Validation)
If the handler decides *not* to update the model (e.g., value is out of bounds), the Pipe never emits. The UI stays at the old value (or snaps back if it was an optimistic update).

### 3.3 State Patterns

| Pattern | Data Binding | Description |
| :--- | :--- | :--- |
| **Controlled** | `value: pipe!(...)` | Standard. UI follows Pipe. Handler updates data. |
| **Two-Way** | `value: TwoWay::new(...)` | Sugar. Auto-generates read pipe and write handler. |
| **Uncontrolled** | `value: constant` | Widget manages its own state (rare in complex apps). |

---

## 4. Implementation Patterns

### 4.1 Basic Controlled Widget
```rust
let slider = @Slider {
    value: pipe!($read(data).volume),
    on_change: move |v| {
        // Business logic here
        if v <= 100.0 { $write(data).volume = v; } 
    }
};
```

### 4.2 Two-Way Binding (Sugar)
Use for simple fields with no side effects.
```rust
@Slider { value: TwoWay::new(data.volume) }
```

**Widget Definition**: To support `TwoWay`, widget authors mark fields with `#[declare(event = on_xxx(EventType))]`:
```rust
#[derive(Declare)]
pub struct Slider {
    #[declare(event = on_value_change(SliderChangedEvent))]
    pub value: f32,
}
```

This generates on the Declarer:
- An `on_value_change` field and setter method
- In `finish()`, the handler is connected via `on_custom::<SliderChangedEvent>`

**Logic Flow**: When `TwoWay::new(source)` is passed to an event-enabled field:
1. A read pipe is auto-generated: `pipe!($read(source).clone())`
2. An event handler is auto-generated to write back to source on change
3. The field accepts three initialization modes:
   - `value: 50.0` → Uncontrolled
   - `value: pipe!(...)` → Controlled (one-way)
   - `value: TwoWay::new(...)` → Two-Way (auto-sync)

**Behavior Details**:
- **Source changes**: When the source `StateWriter` is modified externally (not via UI), the widget automatically updates on the next frame.
- **Performance**: Changes trigger a single update cycle, same as manual `pipe!` + handler.

**Avoid when**: You need validation, type conversion, or side effects (logging/analytics). There is no intermediate form—if you need any custom behavior, use the explicit `pipe!` + event handler pattern.

### 4.3 Type Conversion & Live Validation
Handling mismatched types (String input -> Number model).

> [!NOTE]
> **Events are interaction-only**: Widget events like `on_change` and `on_submit` are triggered **exclusively by user interaction** (typing, clicking, dragging). API calls like `$write(input).set_text()` do **not** fire these events. This design prevents infinite loops and makes the "escape hatch" safe to use.

```rust
@Input {
    value: pipe!($read(data).age.to_string()),
    on_change: move |s| {
        match s.parse::<u32>() {
            Ok(v) => $write(data).age = v, // Valid: update model
            Err(_) => {
                // Invalid: Force UI update to show raw input, but don't touch model.
                // Safe: set_text() does NOT trigger on_change (no loop).
                $write(input).set_text(&s); 
            }
        }
    }
}
```

### 4.4 Optimistic UI (Async/Heavy Operations)
When the data update is slow (network request, heavy computation), update the UI immediately using **Direct Property Write**.

```rust
@Slider {
    value: pipe!($read(data).cloud_setting),
    on_change: move |v| {
        let old_value = $read(data).cloud_setting;
        
        // 1. Optimistic: Update UI instantly
        $write(slider).value = v;
        
        // 2. Async: Do the heavy lifting
        spawn(async move {
            match api.update(v).await {
                Ok(confirmed) => {
                    // 3a. Success: Reconcile with server value
                    $write(data).cloud_setting = confirmed;
                }
                Err(_) => {
                    // 3b. Failure: Rollback to previous value
                    $write(data).cloud_setting = old_value;
                }
            }
        });
    }
}
```

### 4.5 Throttling & Debouncing
Prevent event storms using Rx operators.
```rust
// Search-as-you-type (Debounce)
input.change_stream()
    .debounce(Duration::from_millis(300))
    .subscribe(move |s| search_api(s));
```

### 4.6 Common Pitfalls

**❌ Double Update in Optimistic UI**
```rust
on_change: move |v| {
    $write(slider).value = v;  // Optimistic update
    $write(data).volume = v;   // Model update → Pipe emits → redundant UI update
}
```
**Fix**: Choose one path. Use optimistic write only when the model update is async/heavy; otherwise, just update the model directly.

**❌ Forgetting to Reconcile Optimistic State**
```rust
on_change: move |v| {
    $write(slider).value = v;  // UI shows new value
    spawn(async move {
        api.update(v).await;   // ← No model update after async!
    });
}
```
**Fix**: Always update the model after the operation completes (see [4.4](#44-optimistic-ui-asyncheavy-operations)).

**❌ Validation Logic in Two Places**
```rust
@Slider {
    value: pipe!(clamp($read(data).vol, 0.0, 100.0)),  // Clamping here
    on_change: move |v| {
        if v <= 100.0 { $write(data).vol = v; }        // ...and here
    }
}
```
**Fix**: Validate in one place only—preferably in the handler.

---

### 4.7 Advanced Declare Patterns

#### Validation
To ensure widget consistency at creation time, add `#[declare(validate)]` to your struct. This forces the `declare!` macro to call `declare_validate()` before finishing.

```rust
#[derive(Declare)]
#[declare(validate)]
pub struct Range {
    pub min: f32,
    pub max: f32,
}

impl Range {
    fn declare_validate(&self) -> Result<(), &'static str> {
        if self.min > self.max {
             Err("min cannot be greater than max")
        } else {
             Ok(())
        }
    }
}
```

#### Custom Update Logic (Setters)
By default, when a bound pipe emits a value, Ribir performs a direct field assignment: `widget.field = value`.
If a field update requires side effects (e.g., recalculating layout, clamping values), use `#[declare(setter = method_name)]` to redirect the update to a method.

```rust
#[derive(Declare)]
pub struct Slider {
    #[declare(setter = set_value)]
    pub value: f32,
}

impl Slider {
    // This is called whenever the pipe updates 'value'
    fn set_value(&mut self, v: f32) {
        self.value = v.clamp(0.0, 100.0);
        // ... trigger other updates ...
    }
}
```

You can also specify a type if the setter accepts a transformed value: `#[declare(setter = set_color(Color))]`.

---

## 5. Best Practices

### 5.1 Complex Widget Update Strategy

For complex container widgets like `Tabs`, `List`, and `Menu`, widget implementers **should not** provide dynamic item manipulation APIs (e.g., `addItem()`, `removeItem()`).

**Update Strategy**: Use `pipe!` to wrap the **entire widget**. When the underlying data changes, the whole widget re-renders. Ribir's reconciliation engine, powered by `key`/`reuse_id`, handles efficient diffing and instance reuse automatically.

```rust
// ✅ Correct: Pipe wraps the entire widget
@pipe! {
    @Tabs {
        reuse_id: "tabs_xxx",
        @ {
            $read(data).tabs.iter().map(|tab| @Tab {
                key: tab.id,  // Enables efficient reconciliation
                label: tab.name.clone(),
                @ { tab.content.clone() }
            })
        }
    }
}
```

> [!NOTE]
> **`reuse_id` vs `key`**:
> - `reuse_id`: The **framework-level** reconciliation mechanism. Use it to preserve widget identity and internal state across re-renders.
> - `key`: A **widget-level** convenience prop that some widgets may provide. It maps to `reuse_id` internally for easier usage in dynamic lists.

**Benefits**:
- **Simpler mental model**: No imperative add/remove APIs to learn.
- **Consistency**: Widget state always reflects data state.
- **Automatic optimization**: Framework handles diffing, reordering, and instance reuse.

### Widget Reuse (`reuse_id`)
In dynamic lists (`pipe! { @List }`), Ribir recreates widgets by default when data changes. This destroys focus, selection, and scroll state.

**Solution**: Use `reuse_id` with a stable identifier.

```rust
@pipe! {
    @List {
        reuse_id: "list_xxx",
        @ {
            $read(data).items.iter().map(|item| @ListItem {
                reuse_id: item.id, // Keeps the widget instance alive
                content: item.name,
            })
        }
    }
}
```
*   **Without `reuse_id`**: Focus is lost every time the list updates.
*   **With `reuse_id`**: Focus, cursor position, and animation state are preserved.