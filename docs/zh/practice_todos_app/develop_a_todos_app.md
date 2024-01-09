# Ribir 实践： 完整开发一个 Todos 应用

本教程将通过构建一个简单的 Todos 应用来向你展示一个 Ribir 应用的开发方式，同时帮你巩固 Ribir 的基本概念和使用方法。

该应用将允许你添加、删除、编辑和标记任务，并提供自动保存功能。

> 你将了解：
>
> - 如何用 Ribir 推荐的方式开发设计一个 Todos 引用

## 前提条件

为了完成本教程，我们假设你：

- 了解 [Rust](https://www.rust-lang.org/learn) 语言的基础知识和语法
- 完成了前置教程 [快速上手](../zh/快速上手) 系列


## 最终效果展示

<img src="/static/img/todos-demo.gif" width=640px/>

## 代码结构

作为一个 GUI 框架，Ribir 最重要的一个目标就是让你在应用设计之初，可以专注于数据结构和算法（业务逻辑）的抽象，而完全不用考虑 UI。UI 则作为一个完全独立的模块开发，两者之间通过前者定义的 API 完成连接。

因此，在 Ribir 仓库中，你会发现几乎所有非纯粹的界面展示的例子都有这样两个主要的文件：一个和应用同名的 `xxx.rs` 文件，实现了应用的核心数据和逻辑；一个 `ui.rs` 文件实现了对核心数据的 UI 描述。另外，还有一个 `main.rs` 文件作为应用的入口。

在本教程中，我们也用同样的方式来组织我们的 Todos 应用：
  
```text
- src
  - main.rs
  - todos.rs
  - ui.rs
```

## 内核开发

Ribir 不会一开始就考虑做控件的划分、层级结构的组织，UI 状态的管理等。Ribir 会推荐你先抽象好应用的核心数据结构和逻辑，设计定义好 API，再基于你的数据和视觉效果来组织你的 UI。

当然，如果是多人开发，上面这些工作可以是并行展开的。因为你需要独自完成全本章教程，所以让我们按顺序一步步来。第一步先来完成核心数据结构部分的开发，并完全不去考虑 UI 的事情。

```rust ignore
// todos.rs

use serde::{Deserialize, Serialize};
use std::{
  collections::BTreeMap,
  fs::File,
  io::{self, BufWriter, Write},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Todos {
  tasks: BTreeMap<TaskId, Task>,
  next_id: TaskId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
  id: TaskId,
  pub complete: bool,
  pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TaskId(usize);

impl Todos {
  pub fn new_task(&mut self, label: String) -> TaskId {
    let id = self.next_id;
    self.next_id = self.next_id.next();
    self.tasks.insert(id, Task { id, label, complete: false });
    
    id
  }

  pub fn remove(&mut self, id: TaskId) { self.tasks.remove(&id); }

  pub fn get_task(&self, id: TaskId) -> Option<&Task> { self.tasks.get(&id) }

  pub fn get_task_mut(&mut self, id: TaskId) -> Option<&mut Task> { self.tasks.get_mut(&id) }

  pub fn all_tasks(&self) -> impl Iterator<Item = TaskId> + '_ { self.tasks.keys().copied() }
}

impl Task {
  pub fn id(&self) -> TaskId { self.id }
}

impl Todos {
  pub fn load() -> Self {
    std::fs::read(Self::store_path())
      .map(|v| serde_json::from_slice(v.as_slice()).unwrap())
      .unwrap_or_else(|_| Todos {
        tasks: BTreeMap::new(),
        next_id: TaskId(0),
      })
  }

  pub fn save(&self) -> Result<(), io::Error> {
    let file = File::create(Self::store_path())?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(&mut writer, self)?;
    writer.flush()?;
    Ok(())
  }

  fn store_path() -> std::path::PathBuf { std::env::temp_dir().join("ribir_todos.json") }
}

impl TaskId {
  pub fn next(&self) -> Self { Self(self.0 + 1) }
}
```


`Todos` 内核主要由 `Todos`, `Task` 和 `TaskId` 三个类型组成，其中 `Todos` 是一个包含了所有任务的列表，`Task` 是一个任务的结构体，`TaskId` 是任务的唯一标识符。`Todos` 提供了对任务的增删改查的方法，并提供了保存到文件的能力。通常情况下，你还需要编写完备的单元测试来保证你的代码的正确性。

这部分工作与你平时写无界面的 Rust 代码的方式没有什么不同，你可以按照自己的习惯来组织代码，只要最后能够提供完整能力的 API 即可。在 Ribir 应用的的设计理念中，这部分工作非常重要，但却不是本教程的重点，如果你熟悉 Rust 语法，你应该能够轻易理解，这里就不再赘述了。


> Tips
>
> 基于这样一个结构，完成这部分工作后，你可以轻易将你的核心部分变成一个库,并以此创建一个 CLI 应用，来给你的用户提供更好的开发体验和更多的使用场景。

现在你的应用，已经有了完备的逻辑，但是还没有任何界面。下一步，让我们用 Ribir 来为它构建一个界面。

## 描述 UI

在正式进入 UI 开发之前，我们先对照原型图划分几个主要部分，以方便后文的交流：

<img src="/static/img/todos-widgets.png" width=830px/>

1. Title 标题区，展示应用的名称
2. Input 区，输入任务内容，按回车键添加任务
3. Task Tabs，任务选项卡，分为 All, Active 和 Completed 三个选项卡，分别展示对应任务列表
4. Task，单个任务的展示，提供编辑，标记完成和删除功能。

### 用 Ribir 搭建出整体结构

我们已经在 [内核开发](#内核开发) 中定义好 `Todos` 类型作为根数据结构，现在可以直接通过 `Compose` 从它开始对整个 UI 进行描述了。在此之前，你需要先在 `main.rs` 中引入 `todos.rs` 和 `ui.rs`，并添加一个 `main` 函数作为应用入口：

```rust ignore
//  main.rs

mod todos;
mod ui;

use ribir::prelude::*;
use std::time::Duration;

fn main() {
  let todos = State::value(todos::Todos::load());

  // save changes to disk every 5 seconds .
  let save_todos = todos.clone_reader();
  
  watch!($todos;)
    .debounce(Duration::from_secs(5), AppCtx::scheduler())
    .subscribe(move |_| {
      if let Err(err) = save_todos.read().save() {
        log::error!("Save tasks failed: {}", err);
      }
    });

  App::run(fn_widget! { todos })
}
```

在 `main.rs` 中，先创建了一个 `State` 来保存 `Todos` 数据，并将它当做根 widget 传递给 `App::run` 方法，这样应用就可以运行起来了。

同时对 `todos` 的变更进行了监听，并将其每隔 5 秒钟保存到磁盘上。当然，你的应用现在还没有任何交互，无法对 `todos` 进行修改，所以保存逻辑不会触发，但很快当你添加了交互，就能用的上这个自动保存的功能了。

注意到 `watch!($todo;)` 中的 `;` 号了吗？ 这是故意的，因为不想接收 `todos` 的变更结果，而只想知道它发生了变化，因为我们要在订阅函数中去读取它的最新值去保存。

接下来，在 `ui.rs` 中添加如下代码，来将 `Todos` 描述为一个 widget：

```rust ignore
// ui.rs
use ribir::prelude::*;

impl Compose for Todos {
  fn compose(this: impl StateWriter<Value = Self>) -> impl WidgetBuilder {
    fn_widget! {
      @Column {
        align_items: Align::Center,
        item_gap: 12.,
        @H1 { text: "Todo" }
      }
    }
  }
}
```

现在，当你通过 `cargo run` 运行时，你将看到窗口上面仅有一个标题 "Todo"。上面代码中，我们将 `Column` 作为 `Todo` 的根 widget，它是一个 `Render` 类型的 widget，能够将它的孩子按照垂直方向排列，并提供了一些相关属性，这里我们设置了 `align_items` 为 `Align::Center`，表示将孩子们在垂直方向上居中对齐，`item_gap` 为 `12.`，表示孩子之间的间隔为 12 个逻辑像素。

下一步，我们先往 `Column` 中添加一个空的任务选项卡，撑起我们整个结构：

```rust ignore
@Tabs {
  @Tab {
    @TabItem { @{ Label::new("ALL") } }
    @TabPane {
      @{ fn_widget!{ @Text { text: "Coming Soon!" } }}
    }
  }
  @Tab {
    @TabItem { @{ Label::new("ACTIVE") } }
    @TabPane {
      @{ fn_widget!{ @Text { text: "Coming Soon!" } } }
    }
  }
  @Tab {
    @TabItem { @{ Label::new("DONE") } }
    @TabPane {
      @{ fn_widget!{ @Text { text: "Coming Soon!" } }}
    }
  }
}
```

同样 `Tabs` 也是 Ribir widgets 库为我们提供的，它是一个 `ComposeChild` widget，并且规定了它的孩子必须是 `Tab` 类型。因为，我们现在还没有准备好 `Tab` 中要展示的内容，所以用了一个 “Coming soon!” 的 `Text` 来占位。不过，在 `TabPane` 中，我们没有直接使用 `Text` 控件，而是用了一个函数 widget 来作为孩子，这是因为 `Tabs` 规定了 `TabPane` 的内容必须是一个 `GenWidget`, 因为它只想构建活动 `Tab` 对应的内容，而不是所有 `Tab`。而一个支持多次调用的函数 widget 可以转换成 `GenWidget`。

### 增加任务录入能力

现在，我们来添加录入数据的能力: 在 `Column` 中添加一个 `Input`，响应回车按钮将 `Input` 中的内容作为任务添加到 `Todos` 中。 等等，我们要怎么在一个 `Input` 中的事件回调中，访问 `Input` 自己呢？

```rust ignore
@Input {
  on_key_down: move |e| {
    if e.key_code() == &PhysicalKey::Code(KeyCode::Enter) {
      // 如何获得 Input 自己？
    }
  }
}
```

好在，Ribir 非常易于和 Rust 交互，还记得在[组合 widget](../get_started/quick_start.md#compose-widget--描述你的数据结构) 中讲到的通过变量声明孩子吗？

```rust ignore

@ {
  let input = @Input {};
  @$input {
    on_key_down: move |e| {
    if e.key_code() == &PhysicalKey::Code(KeyCode::Enter)
      && !$input.text().is_empty() {
        $this.write().new_task($input.text().to_string());
        $input.write().set_text("");
      }
    },
    @{ Placeholder::new("What do you want to do ?") }
  }
}

```

现在，将上面的代码添加到 `Column` 的 `Tabs` 之前。你就可以通过这个输入框录入新的任务了。

### 添加任务列表

目前，`Tab` 中还没有任何内容展示，现在我们就来添加它。

三个选项卡的虽然内容不一样，但都有同样的展示结构，因此你可以将它们抽象成一个 widget。因为没有对应的数据结构，所以你可以用一个函数控件来实现它，假设这个函数被命名为 `task_list`。

第二个可以抽象的 widget 是 `Task`，它虽然有自己的数据结构，但我们却不打算通过 `Compose` 来把它描述为 widget 。因为我们想将对 `Task` 的删除功能也实现在这个 widget 中，而仅凭`Task` 自己是无法实现删除的。所以，也抽象一个函数控件，方便获取上下文，假设这个函数被命名为 `task_item`。

先来看 `task_list` 的实现：

```rust ignore
// ui.rs

...

fn task_lists(this: &impl StateWriter<Value = Todos>, filter: fn(&Task) -> bool) -> GenWidget {
  let this = this.clone_writer();
  fn_widget! {
    @VScrollBar {
      @Lists {
        @ { pipe!($this;).map(move |_| {
          // 这里故意写一行不会执行的代码, 告诉 Ribir 
          // 当前闭包需要捕获 `this` 的 Writer 而不是 Reader
          let _hint_capture_writer = || $this.write();
          
          let mut widgets = vec![];
          for id in $this.all_tasks() {
            if $this.get_task(id).map_or(false, filter) {
              let task = this.split_writer(
                move |todos| todos.get_task(id).unwrap(),
                move |todos| todos.get_task_mut(id).unwrap(),
              );
              widgets.push(task_item(task));
            }
          }
          widgets
        }) }
      }
    }
  }
  .into()
}
```

这个函数控件用 `Lists` 来呈现整个列表的，并通过 `pipe!($this;).map(move |_| { ... })` 监听 `this` 的修改，确保任务列表的内容会随着 `this` 的变化而变化，最后通过一个 `VScrollBar` 来提供滚动能力。

注意到状态分裂这一行了吗？
  
```rust ignore
let task = this.split_writer(
  move |todos| todos.get_task(id).unwrap(),
  move |todos| todos.get_task_mut(id).unwrap(),
);
```

它从 `this` 中分裂出一个 `Task` 的 `Writer`, 并把它传递给 `task_item` 函数控件，这样在 `task_item` 控件中就可以直接修改 `Task` 数据而不影响整个 `Todos` 的界面了。

在`task_lists`中，有一个 tricky 的地方，你一定也注意到了：

```rust ignore
let _hint_capture_writer = || $this.write();
```

为何需要这一行代码？因为 Ribir 在解析 move 闭包时会根据闭包内是否使用了 `$this` 的使用情况，来帮助你在闭包前自动对 this 的 reader 或 writer 进行捕获，避免你需要手动进行 clone_reader 或 clone_writer。当 move 闭包中只使用了读引用（$this），就捕获 Reader，如果使用了写引用（`$this.write()` 或 `$this.silent()`），则捕获 Writer。而上面的闭包中，完全没有用到的 `$this` 的写引用，但却需要通过 `this` 分裂一个子 `Writer` —— 只有 Writer 才能分裂子 `Writer`。因此故意写下这一行，来强制 Ribir 捕获 `this` 的 `Writer`。

再来看 `task_item` 的实现：

```rust ignore
// ui.rs

...

fn task_item<S>(task: S) -> impl WidgetBuilder
where
  S: StateWriter<Value = Task> + 'static,
  S::OriginWriter: StateWriter<Value = Todos>,
{
  let todos = task.origin_writer().clone_writer();

  fn_widget! {
    let id = $task.id();
    let checkbox = @Checkbox { checked: pipe!($task.complete) };
    watch!($checkbox.checked)
      .distinct_until_changed()
      .subscribe(move |v| $task.write().complete = v);

    @ListItem {
      @{ HeadlineText(Label::new($task.label.clone())) }
      @Leading {
        @{ CustomEdgeWidget(checkbox.widget_build(ctx!())) }
      }
      @Trailing {
        cursor: CursorIcon::Pointer,
        on_tap: move |_| $todos.write().remove(id),
        @{ svgs::CLOSE }
      }
    }
  }
}
```

在这个函数控件中，有意思的的地方在于，并没有通过参数来传递 `Todos`, 而是规定了 `Task` 必须是从 `Todos` 中分裂出来的，这样你就可以反向拿到 `Todos` 的 `Writer` ，从而实现删除功能。

接着，用 `Checkbox` 来展示任务是否完成，并监听它的变化，将变化同步回 `Task` 中。

最后，用 `ListItem` 来展示一个完整任务，将 `Checkbox`, 删除按钮和任务内容组合在一起。`ListItem` 也是 Ribir widgets 库提供的一个 widget，并规定了自己的孩子类型，这里用到了 `HeadlineText` 来展示标题， `Leading` 表示头部内容，`Trailing` 表示尾部内容。

现在，在 `Todos` 的 `compose` 中找到 `TabPane` 并用 `task_lists` 来替换掉原来的 "coming soon!" 吧：

```rust ignore
// ui.rs

...

@Tabs {
  @Tab {
    @TabItem { @{ Label::new("ALL") } }
    // new
    @TabPane { @{ task_lists(&this, |_| true) } }
  }
  @Tab {
    @TabItem { @{ Label::new("ACTIVE") } }
    // new
    @TabPane { @{ task_lists(&this, |t| !t.complete )} }
  }
  @Tab {
    @TabItem { @{ Label::new("DONE") } }
    // new
    @TabPane { @{ task_lists(&this, |t| t.complete )} }
  }
}

...
```

### 增加对单个任务的编辑功能

你的 Todos 应用已经基本完成了，不过还差最后一步: 增加双击任务进行编辑内容的功能。

通过双击来记录任务的编辑状态，当一个任务不是编辑状态时，展示 `task_item`, 否则展示一个 `Input`。

回到 `task_lists` 中，做如下修改：

```rust ignore

fn_widget! {
    // new: 新增一个状态，记录编辑任务的 Id
    let editing = Stateful::new(None);

    @VScrollBar {
      @Lists {
        @ { pipe!($this;).map(move |_| {
          let _hint_capture_this = || $this.write();
          let mut widgets = vec![];

          for id in $this.all_tasks() {
            if $this.get_task(id).map_or(false, cond) {
              let task = this.split_writer(
                move |todos| todos.get_task(id).unwrap(),
                move |todos| todos.get_task_mut(id).unwrap(),
              );
              // new: 如果任务处于编辑状态，则显示输入框，否则显示任务项
              let item = pipe!(*$editing == Some($task.id()))
                .value_chain(|s| s.distinct_until_changed().box_it())
                .map(move |b|{
                  if b {
                    let input = @Input { auto_focus: true };
                    $input.write().set_text(&$task.label);
                    @$input {
                      on_key_down: move |e| {
                        let key = e.key_code();
                        if key == &PhysicalKey::Code(KeyCode::Escape) {
                          *$editing.write() = None;
                        } else if e.key_code() == &PhysicalKey::Code(KeyCode::Enter) {
                          let label = $input.text().to_string();
                          if !label.is_empty() {
                            $task.write().label = label;
                            *$editing.write() = None;
                          }
                        }
                      }
                    }.widget_build(ctx!())
                  } else {
                    let item = task_item(task.clone_writer());
                    @$item {
                      on_double_tap: move |_| *$editing.write() = Some(id)
                    }.widget_build(ctx!())
                  }
                });
              widgets.push(item);
            }
          }
          widgets
        }) }
      }
    }
  }
  .into()
```

到这里，你的 Todos 应用已经完成了，你可以运行它，添加、删除、标记任务，双击进行编辑，甚至可以关闭应用，再次打开时，你的任务列表还在，因为你的数据会自动将任务保存到磁盘上。

通过这个教程，你应该发现了 Ribir 的一些特点，Ribir 不强调 UI 的状态管理，而是通过 API 对数据进行直接操作，UI 则自动响应数据的变化。而状态，只是将数据转换为可被侦听的一个包装。这样的设计，让你可以专注于数据结构和算法和 API 的设计，而 UI 则可以直接使用 API 来展示和操作数据。消去中间层，也消去了由这些中间层带来的复杂性。

## 完善样式和动画

在上面的教程中，你已经完成了一个完整的 Todos 应用，但是它的样式和交互还不够漂亮和现代，如果你想进一步完善你的应用，你可以到[完善样式和动画](./improving_styles_and_animations.md)继续 Todos 应用的旅程。

## 完整代码

```rust ignore
// main.rs
mod todos;
mod ui;
use ribir::prelude::*;
use std::time::Duration;

fn main() {
  let todos = State::value(todos::Todos::load());

  // save changes to disk every 5 seconds .
  let save_todos = todos.clone_reader();
  
  watch!($todos;)
    .debounce(Duration::from_secs(5), AppCtx::scheduler())
    .subscribe(move |_| {
      if let Err(err) = save_todos.read().save() {
        log::error!("Save tasks failed: {}", err);
      }
    });

  App::run(fn_widget! { todos })
}

// todos.rs
use serde::{Deserialize, Serialize};
use std::{
  collections::BTreeMap,
  fs::File,
  io::{self, BufWriter, Write},
};

#[derive(Debug, Serialize, Deserialize)]
pub struct Todos {
  tasks: BTreeMap<TaskId, Task>,
  next_id: TaskId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
  id: TaskId,
  pub complete: bool,
  pub label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct TaskId(usize);

impl Todos {
  pub fn new_task(&mut self, label: String) -> TaskId {
    let id = self.next_id;
    self.next_id = self.next_id.next();

    self.tasks.insert(id, Task { id, label, complete: false });
    id
  }

  pub fn remove(&mut self, id: TaskId) { self.tasks.remove(&id); }

  pub fn get_task(&self, id: TaskId) -> Option<&Task> { self.tasks.get(&id) }

  pub fn get_task_mut(&mut self, id: TaskId) -> Option<&mut Task> { self.tasks.get_mut(&id) }

  pub fn all_tasks(&self) -> impl Iterator<Item = TaskId> + '_ { self.tasks.keys().copied() }
}

impl Task {
  pub fn id(&self) -> TaskId { self.id }
}

impl Todos {
  pub fn load() -> Self {
    std::fs::read(Self::store_path())
      .map(|v| serde_json::from_slice(v.as_slice()).unwrap())
      .unwrap_or_else(|_| Todos {
        tasks: BTreeMap::new(),
        next_id: TaskId(0),
      })
  }

  pub fn save(&self) -> Result<(), io::Error> {
    let file = File::create(Self::store_path())?;
    let mut writer = BufWriter::new(file);
    serde_json::to_writer(&mut writer, self)?;
    writer.flush()?;
    Ok(())
  }

  fn store_path() -> std::path::PathBuf { std::env::temp_dir().join("ribir_todos.json") }
}

impl TaskId {
  pub fn next(&self) -> Self { Self(self.0 + 1) }
}


// ui.rs

use crate::todos::{Task, Todos};
use ribir::prelude::{svgs, *};
use std::time::Duration;

impl Compose for Todos {
  fn compose(this: impl StateWriter<Value = Self>) -> impl WidgetBuilder {
    fn_widget! {
      @Column {
        align_items: Align::Center,
        item_gap: 12.,
        @H1 { text: "Todo" }
        @ {
          let input = @Input {};
          @$input {
            on_key_down: move |e| {
            if e.key_code() == &PhysicalKey::Code(KeyCode::Enter)
              && !$input.text().is_empty() {
                $this.write().new_task($input.text().to_string());
                $input.write().set_text("");
              }
            },
            @{ Placeholder::new("What do you want to do ?") }
          }
        }
        @Tabs {
          @Tab {
            @TabItem { @{ Label::new("ALL") } }
            @TabPane { @{ task_lists(&this, |_| true) } }
          }
          @Tab {
            @TabItem { @{ Label::new("ACTIVE") } }
            @TabPane { @{ task_lists(&this, |t| !t.complete )} }
          }
          @Tab {
            @TabItem { @{ Label::new("DONE") } }
            @TabPane { @{ task_lists(&this, |t| t.complete )} }
          }
        }
      }
    }
  }
}

fn task_lists(this: &impl StateWriter<Value = Todos>, cond: fn(&Task) -> bool) -> GenWidget {
  let this = this.clone_writer();
  fn_widget! {
    let editing = Stateful::new(None);

    @VScrollBar {
      @Lists {
        @ { pipe!($this;).map(move |_| {
          let _hint_capture_this = || $this.write();
          let mut widgets = vec![];

          for id in $this.all_tasks() {
            if $this.get_task(id).map_or(false, cond) {
              let task = this.split_writer(
                move |todos| todos.get_task(id).unwrap(),
                move |todos| todos.get_task_mut(id).unwrap(),
              );
              let item = pipe!(*$editing == Some($task.id()))
                .value_chain(|s| s.distinct_until_changed().box_it())
                .map(move |b|{
                  if b {
                    let input = @Input { auto_focus: true };
                    $input.write().set_text(&$task.label);
                    @$input {
                      on_key_down: move |e| {
                        let key = e.key_code();
                        if key == &PhysicalKey::Code(KeyCode::Escape) {
                          *$editing.write() = None;
                        } else if e.key_code() == &PhysicalKey::Code(KeyCode::Enter) {
                          let label = $input.text().to_string();
                          if !label.is_empty() {
                            $task.write().label = label;
                            *$editing.write() = None;
                          }
                        }
                      }
                    }.widget_build(ctx!())
                  } else {
                    let item = task_item(task.clone_writer());
                    @$item {
                      on_double_tap: move |_| *$editing.write() = Some(id)
                    }.widget_build(ctx!())
                  }
                });
              widgets.push(item);
            }
          }
          widgets
        }) }
      }
    }
  }
  .into()
}

fn task_item<S>(task: S) -> impl WidgetBuilder
where
  S: StateWriter<Value = Task> + 'static,
  S::OriginWriter: StateWriter<Value = Todos>,
{
  let todos = task.origin_writer().clone_writer();

  fn_widget! {
    let id = $task.id();
    let item = @ListItem {};
    let checkbox = @Checkbox { checked: pipe!($task.complete) };
    watch!($checkbox.checked)
      .distinct_until_changed()
      .subscribe(move |v| $task.write().complete = v);

    @$item {
      @{ HeadlineText(Label::new($task.label.clone())) }
      @Leading {
        @{ CustomEdgeWidget(checkbox.widget_build(ctx!())) }
      }
      @Trailing {
        cursor: CursorIcon::Pointer,
        on_tap: move |_| $todos.write().remove(id),
        @{ svgs::CLOSE }
      }
    }
  }
}

```