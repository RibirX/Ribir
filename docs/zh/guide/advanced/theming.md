# 主题

Ribir 提供了一个强大且灵活的主题系统，旨在帮助您构建一致且美观的 UI。其颜色系统深受 Material Design 原则启发，但完全可以自定义以适应任何设计语言。

在核心上，Ribir 中的 `Theme` 是以下内容的集合：
- **调色板**: 一个全面的颜色系统。
- **排版**: 一组语义文本样式。
- **Classes**: 一个强大的机制，将样式逻辑与 Widget 结构分离。

当前主题始终在构建上下文中可用。您可以使用 `Theme::of(ctx)` 访问它。

## 1. 调色板（颜色）

`Palette` 结构定义了应用程序的颜色方案。它通过 `Brightness` 枚举支持浅色和深色模式。

Ribir 的调色板使用语义命名（如 `primary`、`secondary`、`surface`、`error`）而不是描述性命名（如 `red`、`blue`）。这确保了当主题更改时（例如，从浅色切换到深色模式），您的 UI 会正确适应。

### 使用颜色

```rust no_run
use ribir::prelude::*;

fn example() -> Widget<'static> {
    fn_widget! {
        let palette = Palette::of(BuildCtx::get());
        @Container {
            size: Size::new(100., 100.),
            background: palette.primary(), // 访问主色
        }
    }.into_widget()
}
```

## 2. 排版（文本样式）

Ribir 的排版系统将文本样式组织成语义类别，如 `Display`、`Headline`、`Title`、`Label` 和 `Body`。每个类别都有 `Large`、`Medium` 和 `Small` 变体。

此结构允许您在应用程序中定义一致的排版层次。

### 访问文本样式

您可以使用 `TypographyTheme::of(ctx)` 访问排版主题。

```rust no_run
use ribir::prelude::*;

fn text_style_example() -> Widget<'static> {
    fn_widget! {
        let typography = TypographyTheme::of(BuildCtx::get());

        @Column {
            @Text {
                text: "Main Title",
                text_style: typography.display_large.text.clone(),
            }
            @Text {
                text: "Subtitle",
                text_style: typography.title_medium.text.clone(),
            }
            @Text {
                text: "Body text goes here...",
                text_style: typography.body_medium.text.clone(),
            }
        }
    }.into_widget()
}
```

## 3. Classes（样式）

Ribir 最强大的功能之一是其 `Class` 系统。Ribir 中的 `Class` 不只是属性集合（如 CSS 类）；它是**转换 Widget 的函数**。

这允许一个类：
- 设置属性（例如，颜色、填充）。
- 包装 Widget（例如，添加边框或背景容器，添加新的元素等）。
- 添加行为（例如，事件监听器）。

### Class 的使用
#### 步骤 1：定义类名

使用 `class_names!` 宏定义您 Widget 样式的全局唯一键。

```rust ignore
use ribir::prelude::*;

class_names! {
    /// MyCard 的默认类
    MY_CARD,
    /// MyCard 标题的类
    MY_CARD_TITLE
}
```

#### 步骤 2：在 Widget 中使用类

在您 Widget 的 `compose` 方法中，使用这些类名应用来自当前主题的样式。

```rust ignore
#[derive(Declare)]
pub struct MyCard;

impl Compose for MyCard {
    fn compose(this: impl StateWriter<Value = Self>) -> Widget<'static> {
        fn_widget! {
            @Stack {
                // 从主题应用主卡样式
                class: MY_CARD,
                @Text {
                    text: "Card Title",
                    // 从主题应用标题样式
                    class: MY_CARD_TITLE,
                }
            }
        }.into_widget()
    }
}
```

#### 步骤 3：提供样式

我们可以通过 Provider 为 MyCard 提供样式。

```rust ignore
fn main() {
    providers! {
      // 在 Provider 上提供该样式
      providers: [
        Class::provider(MY_CARD, move |w| {
          fn_widget! {
            @Container {
              size: Size::new(100., 100.),
              radius: Radius::all(12.),
              clamp: BoxClamp::fixed_size(Size::splat(48.)),
              @ { w }
            }
          }.into_widget()
        }),
        Class::provider(MY_CARD_TITLE, style_class!{
          line_height: 24.,
          foreground: Color::RED, // 设置文本颜色
        }),
      ],
      @MyCard {  }
    }.into_widget()
}
```

**注意**: 对于添加内建属性的样式，Ribir 提供 `style_class!` 宏，用于快速生成样式。

### 主题中的 Classes

在主题中，您可以为整个应用程序的组件提供一些默认样式。例如，`themes/material` 为 Widgets 组件库提供 Material classes，使 Widgets 组件库像 Material Design 一样运行。
```rust ignore
use ribir_core::prelude::Classes;
//...

pub fn theme() -> Classes {
  let mut classes = Classes::default();

  buttons_cls::init(&mut classes); // Button styles
  badge_cls::init(&mut classes); // Badge styles
  // ..

  Theme {
    classes,
    //...
  }
}


## 4. 图标

Ribir 中的图标与主题分开管理，使用全局 `svg_registry` 模块。这为在应用程序中按名称注册和检索 SVG 图标提供了集中系统。

### 使用 SVG 注册表管理图标

```rust ignore
use ribir::prelude::*;

fn register_icons() {
    // 注册一个带名称的 SVG
    let my_svg = Svg::parse_from_bytes(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24">...</svg>"#.as_bytes(),
        true,  // inherit_fill
        false  // inherit_stroke
    ).unwrap();

    svg_registry::register("my_icon", my_svg);
}

fn icon_example() -> Widget<'static> {
    fn_widget! {
        // 使用注册的图标，如果未找到则回退到默认值
        @Icon { @ { svg_registry::get_or_default("my_icon") } }
    }.into_widget()
}
```

SVG 注册表提供以下函数：
- `svg_registry::register(name, svg)`: 使用特定名称注册 SVG
- `svg_registry::get(name)`: 检索命名 SVG（返回 Option）
- `svg_registry::get_or_default(name)`: 检索命名 SVG 或回退到默认值