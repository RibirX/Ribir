---
sidebar_position: 5
---

# 数据共享与事件

Ribir 提供了在 Widget 树中共享数据和处理 Widget 间通信的机制。

- **Provider**: 向下传递数据（父级 -> 子级/后代）。
- **自定义事件**: 向上传递消息（子级 -> 父级/祖先）。

## Provider（数据向下）

`Provider` 系统让您能够与所有后代 Widget 共享数据，而无需通过层级结构的每一层传递 props。

### 提供数据

您可以使用 `Providers` Widget 或 `providers!` 宏（或 `providers` 属性）来提供数据。

#### 使用 `providers` 字段（推荐）

Ribir 中的大多数 Widget 声明时都支持内置属性（参照 [内置属性和 FatObj](./built_in_attributes_and_fat_obj.md)） `providers` 。这会比用 `Providers` Widget 包装更简洁。

```rust ignore
use ribir::prelude::*;

struct Theme {
    primary: Color,
}

fn app() -> Widget<'static> {
    fn_widget! {
        let theme = Stateful::new(Theme { primary: Color::RED });

        @Column {
            // 在 Widget 上直接提供数据
            providers: [Provider::new(theme)],
            @Text { text: "Hello" }
        }
    }.into_widget()
}
```

#### 使用 `Providers` Widget

或者，您可以使用 `Providers` Widget 包装子树。

```rust ignore
use ribir::prelude::*;

struct UserInfo {
    name: String,
}

fn app() -> Widget<'static> {
    fn_widget! {
        let user = Stateful::new(UserInfo { name: "Alice".to_string() });

        @Providers {
            providers: [
                // 共享一个状态写入器，允许后代读取和修改 'user'
                // 第二个参数 `None` 意味着我们手动管理脏通知或不需要布局更新
                Provider::writer(user, None)
            ],
            @Column {
                @ChildWidget {}
            }
        }
    }.into_widget()
}
```

### 消费数据

后代 Widget 可以使用 `Provider::of`（读取）或 `Provider::write_of`（写入）访问提供的数据。

```rust ignore
fn child_widget() -> Widget<'static> {
    fn_widget! {
        @Text {
            // 从最近的 UserInfo 提供者读取
            text: pipe! {
                let user = Provider::of::<UserInfo>(BuildCtx::get())?;
                format!("Hello, {}", user.name)
            }
        }
    }.into_widget()
}
```

### Provider 的类型

| 方法 | 描述 | 访问 |
|---|---|---|
| `Provider::new(value)` | 共享一个不可变值。 | `Provider::of` |
| `Provider::reader(state)` | 共享一个只读状态。 | `Provider::of`, `Provider::state_of`（读取器） |
| `Provider::writer(state, phase)` | 共享一个可变状态。`phase` 控制布局脏标记。 | `Provider::of`, `Provider::write_of`, `Provider::state_of`（写入器） |
| `Provider::watcher(state)` | 共享一个监视器（可观察对象）。 | `Provider::of`, `Provider::state_of`（监视器） |

## Variant（灵活的数据消费）

`Variant` 提供了对 `Provider` 的快捷访问，能够自动处理静态值和动态状态（监视器）。它通过统一的 API 简化了这两种情况下提供数据的消费。

### 与直接 Provider 访问的区别

直接使用 `Provider` 时，您需要知道数据是：
- 静态值（`Provider::of`）
- 状态读取器/写入器（`Provider::state_of`）
- 监视器（`Provider::state_of` 与监视器类型）

`Variant` 会自动检测提供者类型并为您处理响应性：
- 首先检查**监视器提供者**（响应式）
- 回退到**值提供者**（静态）

### 使用 Variant 的优势
1. **统一的 API**：一个方法（`Variant::new`）同时适用于静态和动态提供者
2. **自动响应性**：如果提供者是一个观察器（Watcher），当值发生变化时，您的组件会自动更新
3. **便捷的映射**：使用 `map()` 在保持响应性的同时转换值
4. **内置回退机制**：通过 `new_or()`、`new_or_default()` 或 `new_or_else()` 提供默认值

### 基本用法

```rust ignore
use ribir::prelude::*;

fn themed_box() -> Widget<'static> {
    fn_widget! {
        // 自动从提供者获取颜色（静态或反应性）
        let color = Variant::<Color>::new(BuildCtx::get()).unwrap();

        @Container {
            size: Size::new(100., 100.),
            // 如果祖先提供 Color 的写入器，这将对变化做出反应
            background: color,
        }
    }.into_widget()
}
```

### 使用回退

```rust ignore
fn themed_text() -> Widget<'static> {
    fn_widget! {
        // 如果可用则使用主题颜色，否则使用红色
        let color = Variant::<Color>::new_or(
            BuildCtx::get(),
            Color::from_rgb(255, 0, 0)
        );

        @Text {
            text: "Hello",
            foreground: color,
        }
    }.into_widget()
}
```

### 映射值

```rust ignore
fn subtle_background() -> Widget<'static> {
    fn_widget! {
        let color = Variant::<Color>::new_or_default(BuildCtx::get());

        // 转换颜色的同时保持反应性
        let subtle_color = color.map(|c| c.with_alpha(0.1));

        @Container {
            background: subtle_color,
            // ... 其他属性
        }
    }.into_widget()
}
```

### 特殊颜色助手

对于 `Variant<Color>`，有用于根据主题的亮度组访问不同色调的内置方法：

```rust ignore
let primary = Variant::<Color>::new(BuildCtx::get()).unwrap();

// 基于主题的亮度组获取不同色调
let base = primary.into_base_color(BuildCtx::get());
let container = primary.into_container_color(BuildCtx::get());
let on_color = primary.on_this_color(BuildCtx::get());
let on_container = primary.on_this_container_color(BuildCtx::get());
```

这些方法在自动调整亮度的同时，能保持对源 `Variant` 变化的响应，让您可以轻松构建出风格一致、适配主题的 UI。

## 自定义事件（消息向上）

自定义事件允许 Widget 调度一个事件，该事件向上冒泡到 Widget 树。祖先可以监听这些事件，而无需显式向下传递回调。

### 定义事件

定义一个结构来保存您的事件数据。

```rust ignore
struct MyCustomEvent {
    message: String,
}
```

### 分发事件

使用 `ctx.window().bubble_custom_event(...)` 从 Widget 分发事件。

```rust ignore
fn child_emitter() -> Widget<'static> {
    fn_widget! {
        @Button {
            on_tap: move |e| {
                let event = MyCustomEvent { message: "Clicked!".to_string() };
                // 从此 Widget 冒泡事件 (e.id)
                e.window().bubble_custom_event(e.id, event);
            },
            // 按钮的内容（TemplateChild），这里直接传入字符串作为文本
            @{ "Emit Event" }
        }
    }.into_widget()
}
```

### 监听事件

祖先可以使用 `on_custom_concrete_event` 监听特定的自定义事件。

```rust ignore
fn parent_listener() -> Widget<'static> {
    fn_widget! {
        @Column {
            // 监听从 Widget 冒泡的 MyCustomEvent
            on_custom_concrete_event: |e: &mut CustomEvent<MyCustomEvent>| {
                println!("Received: {}", e.message);

                // 停止事件进一步向上冒泡
                e.stop_propagation();
            },
            @child_emitter {}
        }
    }.into_widget()
}
```

### 监听任何自定义事件

您还可以使用 `on_custom_event` 监听所有冒泡的自定义事件，但您将收到一个 `RawCustomEvent`，如果需要，您需要手动向下转换。

```rust ignore
@Container {
    on_custom_event: |e: &mut RawCustomEvent| {
        println!("Something happened!");
    }
}
```