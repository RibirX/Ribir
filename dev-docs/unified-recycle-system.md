# RFC: 统一 Reuse 设计

## 1. 动机

当前 Ribir 的复用能力在运行时层面已经基本统一，但公开 API 仍然分裂为两条路径：

- `Reusable`: 面向所有权的手动复用
- `ReuseId` + `LocalWidgets` / `GlobalWidgets`: 面向 scope 的声明式复用

这带来两个问题：

1. 用户心智不统一。开发者需要先判断自己该走哪条路径。
2. `ReuseId` 的定义一致性完全靠自觉维护。相同 key 可能在不同位置绑定了不同 widget 结构，框架目前只能相信 key 是对的。

这份 RFC 的目标不是重写底层 preserve/rehost 机制，而是统一**用户侧的 Reuse 概念**。

## 2. Breaking Change

这是一次明确的 breaking change。

下面这些旧公开概念将被删除，不保留长期兼容层：

- `ReuseId`
- `LocalId`
- `GlobalId`
- `LocalWidgets`
- `GlobalWidgets`

同时包含一个明确的字段重命名：

- builtin DSL 字段: `reuse_id` -> `reuse`
- Rust setter: `with_reuse_id(...)` -> `with_reuse(...)`

新的公开模型统一为：

- `ReuseScope`
- `Reuse`
- `ReuseKey`
- `key.resolve()`
- `key.leave(ctx)`
- `Reusable`

## 3. 核心模型

### 3.1 `ReuseScope`

`ReuseScope` 是统一的复用边界与生命周期容器。

- 框架会在 root 默认提供一个隐式 `ReuseScope`
- 显式 `ReuseScope` 只表示新的复用边界
- 公开 API 不再区分 local scope / global scope

### 3.2 `ReuseKey`

`ReuseKey` 是统一的身份类型，但 key 在创建时要显式声明查找意图：

- `ReuseKey::local(...)`
- `ReuseKey::global(...)`

这里的 `local/global` 不再是两种 scope 类型，而是两种 **key lookup policy**。

#### `ReuseKey::local(...)`

- 只在最近的 `ReuseScope` 中查找
- miss 时也只在最近的 `ReuseScope` 中建立
- 适合 dynamic children、局部命名节点

#### `ReuseKey::global(...)`

- 从当前 `ReuseScope` 开始，沿外层 scope 一直查到 root
- 命中时复用最近命中的绑定
- 全链 miss 时，在 root scope 中建立
- 适合跨页面、跨局部 scope 的共享单例节点

### 3.3 `Reuse`

`Reuse` 只有两种形态：

- `@Reuse { reuse: key, @Child { ... } }`
- `@Reuse { reuse: key }`

它们都遵循 `ReuseKey` 自身携带的 lookup policy。

带 child 的形式语义是 `resolve_or_build`。  
不带 child 的形式语义是 `resolve_only`。

为了让 resolve-only 更轻量，提供语法糖：

```rust
@header.resolve()
```

它等价于：

```rust
@Reuse { reuse: header }
```

此外，FatObj 上的 builtin host 字段命名统一为 `reuse:`。  
这份设计会保留这个 DSL 入口，只替换它接受的值类型与语义：

- 旧入口：`reuse_id:`
- 新入口：`reuse:`
- 旧语义：字段值接受旧的 `ReuseId`
- 新语义：字段值接受 `ReuseKey`

也就是说，用户在 DSL 中仍然写：

```rust
@Item { reuse: some_key, ... }
```

变化点在于 `some_key` 不再是 `ReuseId`，而是显式的 `ReuseKey::local(...)` 或 `ReuseKey::global(...)`。

### 3.4 `leave`

除了 `resolve()`，`ReuseKey` 还提供对称的 imperative API：

```rust
key.leave(ctx)
```

它的目标总是“当前上下文里，这个 key 会 resolve 到的 binding”。

- `local` key: 只作用于最近 `ReuseScope` 中的命中
- `global` key: 作用于 outward lookup 链上最近命中的 binding

`leave` 的公开语义是“让这个 binding 离开当前 scope 解析面”：

- 如果 target 当前还 live，它会继续活到自然 dispose，但同 key 不再能在该 scope 上重新 resolve
- 如果 target 当前已经 cached，它会立刻从该 scope 的 registry 中移除

这不是强制销毁 API；它只表达 scope-level eviction。

### 3.5 `defs`

`ReuseScope` 支持通过 `defs` 预注册局部 definition：

```rust
@ReuseScope {
  defs: [
    reuse_def(header, || header_widget()),
    reuse_def(toolbar, || toolbar_widget()),
  ],
  @Column {
    @header.resolve()
    @toolbar.resolve()
  }
}
```

`defs` 的语义是：

- 只注册到当前 `ReuseScope`
- 只接受 `ReuseKey::local(...)`
- 注册的是 factory，不是立即 build 的 widget
- `defs` 使用数组承载 definition；单个 definition 通过 `reuse_def(...)` helper 构造为统一类型

`ReuseKey::global(...)` 不参与 `defs`。  
global key 只能通过 `resolve_or_build` 首次建立，并在全链 miss 时落到 root。

### 3.6 `Reusable`

`Reusable` 继续保留，但只作为高级特例，用于 tooltip / overlay / follow 这类 owner-driven 场景。

它不是主路径，不参与 `ReuseScope` 的统一语义。

## 4. 用户侧 API 草案

### A. Dynamic Children

dynamic children 是 `local key` 的主路径。

```rust
@ReuseScope {
  @ {
    items.iter().map(|item| @Reuse {
      reuse: ReuseKey::local(item.id),
      @Item {
        title: item.title.clone(),
      }
    })
  }
}
```

### B. Local Named Reuse

```rust
fn header_widget() -> Widget<'static> {
  text! { text: "Persistent Header" }.into_widget()
}

let header = ReuseKey::local("header");

@ReuseScope {
  defs: [
    reuse_def(header, || header_widget()),
  ],
  @Column {
    @header.resolve()
  }
}
```

### C. Global Shared Reuse

```rust
let nav_bar = ReuseKey::global("nav_bar");

// 首次出现时，若全链 miss，则在 root scope 建立
@Reuse {
  reuse: nav_bar,
  @NavBar {}
}

// 其他位置直接引用
@nav_bar.resolve()
```

这里的 `@NavBar {}` 是 fallback builder。  
如果 lookup 链上已经命中现有绑定，这段 child 不会参与构建。

### D. Scope Leave

```rust
let key = ReuseKey::global("nav_bar");

on_tap: move |e| {
  key.leave(e);
}
```

这里的 `e` 只是提供当前 provider 上下文。  
leave 命中的 target 与 `key.resolve()` 的命中规则一致。

## 5. 运行时语义

### 5.1 Local Key

对于 `ReuseKey::local(...)`：

- `key.resolve()` / `@Reuse { reuse: key }`
  只查最近 `ReuseScope`
- `@Reuse { reuse: key, @Child { ... } }`
  只查最近 `ReuseScope`
- lookup miss 时：
  - resolve-only 报错
  - resolve-or-build 在最近 `ReuseScope` 中建立

### 5.2 Global Key

对于 `ReuseKey::global(...)`：

- `key.resolve()` / `@Reuse { reuse: key }`
  从当前 `ReuseScope` 开始，向外查到 root
- `@Reuse { reuse: key, @Child { ... } }`
  也从当前 `ReuseScope` 开始，向外查到 root
- lookup miss 时：
  - resolve-only 报错
  - resolve-or-build 在 root `ReuseScope` 中建立

这意味着：

- `global` key 的 outward lookup 是显式的，不再靠隐式 scope 规则决定
- `global` key 命中现有绑定时，当前位置 child 只是 fallback，不会参与构建

### 5.3 Shadowing

查找总是遵循“最近命中优先”。

也就是说：

- `local` key 只看最近 `ReuseScope`
- `global` key 按当前 -> 外层 -> root 的顺序查找
- 一旦命中某一层，立即停止

这就是该设计中的遮蔽语义。

### 5.4 Registration

注册规则与 key policy 对齐：

- `local` key: 注册在最近 `ReuseScope`
- `global` key: 只有全链 miss 时才注册到 root `ReuseScope`

如果 `global` key 在向外查找过程中已经命中，就说明它已经找到了复用点，不需要再次注册。

### 5.5 Leave

- `leave` 总是针对当前上下文里该 key 最近 resolve 到的 binding
- live target leave 后不会立刻被强制销毁；它只是不再属于当前 scope 的可复用 binding
- cached target leave 后会立刻从 registry 中移除
- local/global 的差异只体现在 target 的命中规则，不体现在 leave 语义本身

### 5.6 Definition 约束

同一个 `ReuseScope` 内，同一个 `local` key 只能有一个 definition source。

允许的来源只有两种：

- `defs`
- 首次命中的 local `resolve_or_build`

如果两者同时为同一个 local key 提供定义，Debug 下应报错。

`global` key 不参与 `defs`，因此不存在这类 scope-local definition 冲突。

### 5.7 生命周期约束

- 同一个 key 在一个 scope / window 内最多只允许一个 live instance
- 实例缓存与回收继续复用现有 preserve-based 机制
- 不允许跨 window 复用

## 6. 实现指导

### 6.1 不维护 tree parent

即便 `global` key 需要向外查找，也不建议在 widget/tree 结构上维护显式 parent 链。

更合适的方向是：

- 继续基于 provider 机制表达 `ReuseScope` 可见性
- outward lookup 通过 provider 上下文获取外层可见 scope
- 不把 scope 父子关系做成额外的树维护负担

### 6.2 单一 Registry

每个 `ReuseScope` 内部可以维护一个统一 registry，而不是分别暴露 defs/instances 两套公开概念。

实现上每个 key 至少需要区分：

- 是否已有 factory
- 是否已有 live/cached instance

但这属于内部状态，不应继续外溢成更多公开用户概念。

## 7. 旧 API 替换关系

- `ReuseId` -> `ReuseKey`
- `LocalId::number(...)` / `LocalId::string(...)` -> `ReuseKey::local(...)`
- `GlobalId::new(...)` -> `ReuseKey::global(...)`
- `LocalWidgets` / `GlobalWidgets` -> `ReuseScope`
- builtin DSL `reuse_id: ...` -> `reuse: ...`
- builtin DSL `reuse: ...` 的值类型从 `ReuseId` 变为 `ReuseKey`
- Rust setter `with_reuse_id(...)` -> `with_reuse(...)`
- old lookup-only scope access -> `key.resolve()`

## 8. 架构优势

- **显式优于隐式**: 复用查找策略由 `ReuseKey::local/global` 直接表达，不再隐藏在 scope 规则里。
- **统一公开面**: 公开 API 收敛为 `ReuseScope` + `ReuseKey` + `Reuse` + `Reusable`。
- **保留局部性**: dynamic children 继续天然走 `local` 路径，不会意外命中外层 scope。
- **支持跨 scope 共享**: `global` key 提供显式的 outward lookup 与 root fallback。
- **与现有底层对齐**: 仍然复用现有 preserve/rehost 机制，只重构公开语义。
