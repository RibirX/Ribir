---
sidebar_position: 1
---

# 自定义 Widget

自定义 Widget 是任何 Ribir 应用程序的构建块。它们让您能够将复杂的 UI 功能、状态和行为封装到可复用的组件中，这些组件可以组合在一起形成更大的应用程序。

## 理解 Widget 类型

在 Ribir 中，您可以创建两种主要类型的自定义 Widget：

1. **Compose Widget**: 高级 Widget，使用 `fn_widget!` 宏通过组合其他 Widget 来构建 UI
2. **Render Widget**: 低级 Widget，直接处理布局和绘制

## 创建 Compose Widget

最常见的自定义 Widget 类型是 `Compose` Widget。这些 Widget 本身不绘制任何内容；相反，它们通过组合其他现有 Widget 来创建新内容。

### 基本结构

要创建自定义 `Compose` Widget，您需要实现 `Compose` trait：

```rust no_run
use ribir::prelude::*;

#[derive(Declare)]
pub struct DocWelcomeCard;

impl Compose for DocWelcomeCard {
    fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
        fn_widget! {
            @Column {
                @Text { text: "Welcome!" }
                @Button { @Text { text: "Click me" } }
            }
        }.into_widget()
    }
}
```


### 使用自定义 Widget

定义好自定义 Widget 后，您就可以像使用任何内置 Widget 一样在 `fn_widget!` DSL 中使用它：

```rust ignore
use ribir::prelude::*;

fn main() {
    App::run(fn_widget! {
        @DocWelcomeCard {}
    });
}
```

### 为自定义 Widget 添加属性

您可以通过向结构体添加字段为自定义 Widget 添加属性。这些字段可以在 DSL 中使用与内置 Widget 相同的语法进行初始化：

```rust no_run
use ribir::prelude::*;

#[derive(Declare)]
pub struct DocUserCard {
    name: String,
    email: String,
    #[declare(default)]
    is_online: bool,
}

impl Compose for DocUserCard {
    fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
        fn_widget! {
            @Container {
                padding: EdgeInsets::all(16.),
                border: Border::all(BorderSide::new(1., Color::GRAY.into())),
                @Column {
                    @Text {
                        text: pipe!($read(this).name.clone()),
                    }
                    @Text {
                        text: pipe!($read(this).email.clone()),
                    }
                    @Row {
                        @Text {
                            text: pipe!($read(this).is_online).map(|v| if v { "Online" } else { "Offline" }),
                            foreground: pipe!($read(this).is_online).map(|v| if v { Color::GREEN } else { Color::GRAY }),
                        }
                        @Container {
                            size: Size::new(10., 10.),
                            margin: EdgeInsets::horizontal(8.),
                            background: pipe!($read(this).is_online).map(|v| if v { Color::GREEN } else { Color::GRAY }),
                            radius: Radius::all(5.),
                        }
                    }
                }
            }
        }.into_widget()
    }
}

// 用法：
fn example() -> Widget<'static> {
    fn_widget! {
        @DocUserCard {
            name: "John Doe".to_string(),
            email: "john@example.com".to_string(),
            is_online: true,
        }
    }.into_widget()
}
```

> **注意:** 使用 `#[derive(Declare)]` 时，字段默认是**必需的**。任何没有 `#[declare(default)]` 或 `#[declare(skip)]` 属性的字段在声明 Widget 时都必须提供。带有 `#[declare(default)]` 的字段是可选的，而 `#[declare(skip)]` 会将字段从 Builder 中完全排除，直接采用默认值。

## 创建 ComposeChild Widget

一些 Widget 被设计为包装或修改单个子 Widget。这些实现 `ComposeChild` trait 而不是 `Compose`。

```rust no_run
use ribir::prelude::*;

#[derive(Declare, Clone)]
pub struct DocCardDecorator {
    #[declare(default)]
    elevation: f32,
}

impl<'a> ComposeChild<'a> for DocCardDecorator {
    type Child = Widget<'a>;

    fn compose_child(this: impl StateWriter<Value = Self>, child: Self::Child) -> Widget<'a> {
        fn_widget! {
            @Container {
                padding: EdgeInsets::all(16.),
                background: Color::WHITE,
                // 基于 elevation 添加阴影
                transform: pipe!($read(this).elevation).map(|e| {
                    Transform::scale(1. - e * 0.01, 1. - e * 0.01)
                }),
                @ { child }
            }
        }.into_widget()
    }
}

// 用法：
fn example() -> Widget<'static> {
    fn_widget! {
        @DocCardDecorator {
            elevation: 4.,
            @Text { text: "This text is inside a card" }
        }
    }.into_widget()
}
```
## 理解子项系统

Ribir 对父子关系有严格的类型系统,以确保编译时的类型安全:

- **SingleChild**: 接受恰好一个子项的 Widget（如 `Padding`、`Container`）
- **MultiChild**: 接受多个子项的 Widget（如 `Row`、`Column`）


例如我们的 Container Widget 是可以接受单个孩子的：
```rust ignore
use ribir::prelude::*;

#[derive(Declare, SingleChild)]
pub struct Container {
    pub size: Size,
}

impl Render for Container {
    fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
        let size = clamp.clamp(self.size);
        ctx.perform_single_child_layout(BoxClamp::max_size(size));
        size
    }

    #[inline]
    fn size_affected_by_child(&self) -> bool { false }
}

// 用法：
fn example() -> Widget<'static> {
    fn_widget! {
        @Container {
            size: Size::new(100., 100.),
            @Text { text: "Hello" } // 可以接受Child Widget
        }
    }
}
```

### 基于模板的子项组合

模板提供编译时类型安全的 Widget 组合。`#[derive(Template)]` 宏启用**自动类型推断**，让您无需显式类型构造函数或字段名就能编写子项。

#### 自动类型推断

使用模板时，Ribir 会自动推断：
- **枚举变体**基于子项类型（通过 `RFrom` trait）
- **结构体字段**基于子项类型（通过 `ComposeWithChild` trait）

这意味着您可以编写 `@{ child }`，Ribir 会自动确定它在模板结构中的位置。

#### 枚举模板：变体推断

枚举模板自动将子项转换为适当的变体：

```rust ignore
use ribir::prelude::*;

// 定义一个具有不同变体类型的枚举模板
#[derive(Template)]
enum ContentType {
    Text(CowArc<str>),
    Number(i32),
}

#[derive(Declare)]
struct MyWidget;

impl<'a> ComposeChild<'a> for MyWidget {
    type Child = ContentType;

    fn compose_child(_: impl StateWriter<Value = Self>, _child: Self::Child) -> Widget<'a> {
        Void{}.into_widget()
    }
}

// 用法 - 自动变体推断：
let text_widget = fn_widget! {
    @MyWidget {
        @{ "Hello" }  // 自动成为 ContentType::Text
    }
};

let number_widget = fn_widget! {
    @MyWidget {
        @{ 42 }  // 自动成为 ContentType::Number
    }
};
```

`#[derive(Template)]` 宏为每个变体生成 `RFrom` 实现，启用基于子项类型的自动转换。

#### 结构模板：字段推断

结构模板自动按类型匹配子项到字段，**无论声明顺序如何**：

```rust ignore
use ribir::prelude::*;

// 为演示定义自定义类型
struct TypeA;
struct TypeB;
struct TypeC;

#[derive(Template)]
struct StructTemplate {
    a: TypeA,
    b: Option<TypeB>,
    c: Option<TypeC>,
}

#[derive(Declare)]
struct MyContainer;

impl ComposeChild<'static> for MyContainer {
    type Child = StructTemplate;

    fn compose_child(_: impl StateWriter<Value = Self>, _child: Self::Child) -> Widget<'static> {
        Void{}.into_widget()
    }
}

// 用法 - 与顺序无关的字段匹配：
let widget = fn_widget! {
    @MyContainer {
        @{ TypeC }  // 通过类型匹配到 'c' 字段
        @{ TypeA }  // 通过类型匹配到 'a' 字段
        @{ TypeB }  // 通过类型匹配到 'b' 字段
    }
};

// 可选字段可以省略：
let minimal = fn_widget! {
    @MyContainer {
        @{ TypeA }  // 只有必需字段
    }
};
```

宏为每个字段生成带有特定类型标记的 `ComposeWithChild` 实现，从而实现自动字段分配。

#### 现实世界示例：列表 Widget

`List` Widget 演示了实用的模板用法：

```rust ignore
// 简化自 widgets/src/list.rs
#[derive(Template)]
pub enum ListChild<'c> {
    StandardItem(PairOf<'c, ListItem>),
    CustomItem(PairOf<'c, ListCustomItem>),
    Divider(FatObj<Stateful<Divider>>),
}

impl<'c> ComposeChild<'c> for List {
    type Child = Vec<ListChild<'c>>;
    // ...
}

// 用法 - 自动变体推断：
let list = fn_widget! {
    @List {
        @ListItem { /* ... */ }      // 自动成为 ListChild::StandardItem
        @ListCustomItem { /* ... */ } // 自动成为 ListChild::CustomItem
        @Divider {}                   // 自动成为 ListChild::Divider
    }
};
```

#### 何时需要显式语法

当类型唯一时，自动推断有效。当以下情况时使用显式语法：

1. **多个字段具有相同类型** - 使用 `#[template(field)]` 属性：
```rust ignore
#[derive(Template)]
struct TwoTexts {
    #[template(field)]
    first: CowArc<str>,
    #[template(field)]
    second: CowArc<str>,
}

// 必须使用显式字段分配：
let widget = fn_widget! {
    @MyWidget {
        @TwoTexts {
            first: "First text",
            second: "Second text",
        }
    }
};
```

2. **非 Widget 模板字段**（使用 `#[template(field)]` 属性）：
```rust ignore
struct TypeA;

#[derive(Template)]
struct ConfigTemplate {
    #[template(field = 5usize)]  // 默认值
    count: usize,
    #[template(field)]           // 必需字段
    name: CowArc<str>,
    item: TypeA,                 // 子项字段（通过类型自动匹配）
}

// 可以重写默认值或省略以使用默认值：
let widget = fn_widget! {
    @MyWidget {
        @ConfigTemplate {
            count: 10usize,  // 重写默认值
            name: "test",    // 必需字段
            @{ TypeA }       // 通过类型匹配的子项
        }
    }
};

// 使用默认值：
let widget2 = fn_widget! {
    @MyWidget {
        @ConfigTemplate {
            name: "test",  // count 使用 5 的默认值
            @{ TypeA }
        }
    }
};
```

此模板系统通过智能类型推断确保类型安全的 Widget 组合，同时最大限度地减少样板代码。

## 高级：创建 Render Widget

对于需要处理自己的布局和绘制的 Widget（如绘制自定义形状或复杂交互），您实现 `Render` trait：

```rust ignore
use ribir::prelude::*;

// 这是一个简单的示例 - 更复杂的 Render Widget 将
// 实现自定义布局和绘制逻辑
#[derive(Declare)]
pub struct DocCustomShape {
    #[declare(default)]
    color: Color,
    #[declare(default)]
    size: Size,
}

impl Render for DocCustomShape {
    fn perform_layout(&self, clamp: BoxClamp, ctx: &mut LayoutCtx) -> Size {
        // 根据约束和我们期望的尺寸返回尺寸
        clamp.clamp(self.size)
    }

    fn paint(&self, ctx: &mut PaintingCtx) {
        // 自定义绘制逻辑
        let rect = Rect::from_size(ctx.box_rect().unwrap().size);
        ctx
          .painter()
          .rect(&rect)
          .set_fill_brush(self.color)
          .fill();
    }
}
```

## 最佳实践

1. **使用 `#[derive(Declare)]`**: 此宏会生成您的 Widget 与 `@` 语法配合使用所需的 Builder 模式
2. **字段要求**: 字段默认是必需的。使用 `#[declare(default)]` 表示可选字段，或使用 `#[declare(skip)]` 从构建器中排除它们。
3. **状态封装**: 保持 Widget 状态封装,避免全局状态
4. **可复用性**: 设计可复用和可组合的 Widget
5. **性能**: 留意 `perform_layout` 和 `paint` 方法中的昂贵操作

## 总结

自定义 Widget 是任何 Ribir 应用程序的基础。通过理解 `Compose` 和 `Render` Widget 之间的区别，以及如何正确定义和使用状态，您可以创建强大、可复用的组件，充分利用 Ribir 声明式 UI 系统的全部功能。