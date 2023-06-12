---
sidebar_position: 1
---

# How to build a Todo Application with Ribir

In this tutorial, we will learn how to build a Todo application using Ribir. By following this tutorial in its entirety, you will be able to complete a Todo application like this by yourself:

![todo example](./img/todo_example.png)

This will be an exciting journey. Let's go!

## Define the Application data structure

The data structure is the core of the application. In Ribir the interface is just the presentation around the data structure, but not interfering with the data structure. So we need to define the Todo data structure first.

Now let's define the simplest Todo application data structure. I call
this structure `TodoMVP`. As a simplest Todo application it has a list
of what it uses to save the collection of todo tasks at least. In
Rust, we can use the `Vec` structure to describe a list, and then, the
list must be able to hold many tasks. A single task needs two fields, `done` and `description`. The `done` field is of type `bool` to describe whether or not the task is completed and the `description` field is of type `String` to describe task specific information.

We can try to do it in `main.rs`.

```rust
struct Task {
  done: bool,
  description: String,
}
struct TodoMVP {
  tasks: Vec<Task>,
}
fn main() {
  let todo_data = TodoMVP {
    tasks: vec![
      Task {
        done: false,
        description: "Complete how to build an application with Ribir".to_string(),
      }
    ]
  };
}
```

At this point our basic data structure is built.

## Build the base view using Ribir

We have a base data structure. Now we use Ribir to build the user interface part of the Todo app. The basic Todo application needs to have two features, input and display. In the input part we can use an `Input` widget to enter a description for a new task. For the display part we can use a `List` widget to render the task list. We start with adding `Input` at the top and the `List` below the input. To accomplish that we use a `Column` widget, which is a basic vertical arrangement layout widget.

We call the user interface of Todo `todo_widget`.

```rust
use ribir::prelude::*;
fn main() {
  // ...
  let todo_widget = widget! {
    Column {
      Input { Placeholder::new("What do you want to do?") }
      Lists {
        ListItem {
          Leading {
            Checkbox { checked: false }
          }
          HeadlineText::new("Complete how to build an application with Ribir")
        }
      }
    }
  };
  app::run(todo_widget);
}
```

Executing `cargo run` launches the app and shows a window with the `Input` and the `Lists`.

## Combine data and view

Now we have Todo data and view. How do we combine them? Actually, our data is the core, the view is just the presentation of the data. So we need to modify the view code.

We use the `Compose` trait to combine data and view.

```rust
// ...
impl Compose for TodoMVP {
  fn compose(this: StateWidget<Self>) -> Widget {
    widget! {
      states { this: this.into_stateful() }
      Column {
        Input { Placeholder::new("What do you want to do?") }
        Lists {
          ListItem {
            Leading {
              Checkbox { checked: false }
            }
            HeadlineText::new("Complete how to build an application with Ribir")
          }
        }
      }
    }
  }
}
fn main() {
  let todo_data = TodoMVP {
    tasks: vec![
      Task {
        done: false,
        description: "Complete how to build an application with Ribir".to_string(),
      }
    ]
  }.into_stateful();
  app::run(todo_data.into());
}
```

The tasks list data is not static. We need to render it dynamically. We can use `DynWidget` to represent dynamic widgets.

```rust
use ribir::prelude::*;
#[derive(Clone)]
struct Task {
  done: bool,
  description: String,
}
struct TodoMVP {
  tasks: Vec<Task>,
}
impl TodoMVP {
  fn task(task: Task) -> Widget {
    widget! {
      ListItem {
        Leading {
          Checkbox { checked: task.done }
        }
        HeadlineText::new(task.description.clone())
      }
    }
  }
}
impl Compose for TodoMVP {
  fn compose(this: StateWidget<Self>) -> Widget {
    widget! {
      states { this: this.into_stateful() }
      Column {
        Input { Placeholder::new("What do you want to do?") }
        Lists {
          DynWidget {
            dyns: {
              let tasks = this.tasks.clone();
              tasks
                .into_iter()
                .map(move |task| {
                  TodoMVP::task(task)
                })
            }
          }
        }
      }
    }
  }
}
fn main() {
  let todo_data = TodoMVP {
    tasks: vec![
      Task {
        done: false,
        description: "Complete how to build an application with Ribir".to_string(),
      }
    ]
  }.into_stateful();
  app::run(todo_data.into());
}
```

Data and views are now connected.

## Add and delete task

As mentioned the core features are adding and deleting tasks. We already have `Input` but can not yet submit a new task. For that we add a button. For `Input` and `Button` we use `Row` to layout them horizontally. We need an event to act upon to invoke the submit functionality. We choose the `tap` event to trigger it.

```rust
widget! {
  // ...
  Column {
    Row {
      Input {
        id: input,
        Placeholder::new("What do you want to do?")
      }
      Button {
        tap: move |_| {
          let description = input.text().to_string();
          this.tasks.push(Task {
            description,
            done: false,
          });
          input.set_text(String::default().into());
        },
        ButtonText::new("ADD")
      }
    }
    // ...
  }
  // ...
}
```

Now we can add new task to our list. Next we want to delete tasks. We should have an `Icon` to dispatch delete events. We will wrap the icon in a `ListItem`. `ListItem` has `Trailing` to put `Widget` in the trail of list items.

```rust
// ...
impl TodoMVP {
  fn task(this: StateRef<Self>, idx: usize, task: Task) -> Widget {
    let this = this.clone_stateful();
    widget! {
      states { this }
      ListItem {
        Leading {
          Checkbox {
            id: checkbox,
            checked: task.done
          }
        }
        HeadlineText::new(task.description.clone())
        Trailing {
          Icon {
            tap: move |_| { this.tasks.remove(idx); },
            svgs::CLOSE
          }
        }
      }
      finally {
        let_watch!(checkbox.checked)
          .subscribe(move |v| this.silent().tasks[idx].done = v);
      }
    }
  }
}
// ...
```

## Use `Tab` to categorize tasks

Suppose we want to archive a task to distinguish from it being actually completed or it just becoming obsolete. Ribir provides tabs that we can use to easily distiguish between "all", "activity", and "done".

```rust
// ...
Tabs {
  Tab {
    TabHeader {
      Text { text: "ALL" }
    }
    TabPane {
      Lists {
        DynWidget {
          dyns: {
            let tasks = this.tasks.clone();
            tasks
              .into_iter()
              .enumerate()
              .filter(move |(_, _)| true)
              .map(move |(idx, task)| {
                TodoMVP::task(this, idx, task)
              })
          }
        }
      }
    }
  }
  Tab {
    TabHeader {
      Text { text: "ACTIVE" }
    }
    TabPane {
      Lists {
        DynWidget {
          dyns: {
            let tasks = this.tasks.clone();
            tasks
              .into_iter()
              .enumerate()
              .filter(move |(_, task)| !task.done)
              .map(move |(idx, task)| {
                TodoMVP::task(this, idx, task)
              })
          }
        }
      }
    }
  }
  Tab {
    TabHeader {
      Text { text: "DONE" }
    }
    TabPane {
      Lists {
        DynWidget {
          dyns: {
            let tasks = this.tasks.clone();
            tasks
              .into_iter()
              .enumerate()
              .filter(move |(_, task)| task.done)
              .map(move |(idx, task)| {
                TodoMVP::task(this, idx, task)
              })
          }
        }
      }
    }
  }
}
// ...
```

Yeah, we've done it. But we have a problem - too verbose code. In `TabPane` most of the logic is repeated. Let's improve that. We can abstract and create shared panel logic.

```rust
// ...
impl TodoMVP {
  // ...
  fn pane(this: StateRef<Self>, cond: fn(&Task) -> bool) -> Widget {
    let this = this.clone_stateful();
    widget! {
      states { this }
      Lists {
        DynWidget {
          dyns: {
            let tasks = this.tasks.clone();
            tasks
              .into_iter()
              .enumerate()
              .filter(move |(_, task)| cond(task))
              .map(move |(idx, task)| {
                TodoMVP::task(this, idx, task)
              })
          }
        }
      }
    }
  }
}
Tabs {
  Tab {
    TabHeader {
      Text { text: "ALL" }
    }
    TabPane {
      Self::pane(this, |_| true)
    }
  }
  Tab {
    TabHeader {
      Text { text: "ACTIVE" }
    }
    TabPane {
      Self::pane(this, |task| !task.done)
    }
  }
  Tab {
    TabHeader {
      Text { text: "DONE" }
    }
    TabPane {
      Self::pane(this, |task| task.done)
    }
  }
}
// ...
```

Now, this code looks much more concise.

## Use `Scrollbar` to support scrollable

If the tasks list grows, there might not be enough room to show all tasks at once. We need a scrollable view to show let the user  decide what to show. Ribir has a built-in widget `Scrollbar`. A scrollbar can have horizontal or vertical direction. We want scroll in a vertical direction, so we use `VScrollbar`.

It's straightforward like this:

```rust
// ...
impl TodoMVP {
  // ...
  fn pane(this: StateRef<Self>, cond: fn(&Task) -> bool) -> Widget {
    let this = this.clone_stateful();
    widget! {
      states { this }
      VScrollbar {
        Lists {
          DynWidget {
            dyns: {
              let tasks = this.tasks.clone();
              tasks
                .into_iter()
                .enumerate()
                .filter(move |(_, task)| cond(task))
                .map(move |(idx, task)| {
                  TodoMVP::task(this, idx, task)
                })
            }
          }
        }
      }
    }
  }
}
// ...
```

## Adding transition animation

Ribir is a modern GUI library. Let's add some animation to demonstrate that. We add a task show animation when it is mounted.

```rust
impl TodoMVP {
  fn task(this: StateRef<Self>, idx: usize, task: Task) -> Widget {
    let this = this.clone_stateful();
    widget! {
      states { this, mount_idx: Stateful::new(0) }
      ListItem {
        id: item,
        transform: Transform::default(),
        mounted: move |_| {
          *mount_idx += 1;
          mount_animate.run()
        },
        Leading {
          Checkbox {
            id: checkbox,
            checked: task.done
          }
        }
        HeadlineText::new(task.description.clone())
        Trailing {
          Icon {
            tap: move |_| { this.tasks.remove(idx); },
            svgs::CLOSE
          }
        }
      }
      Animate {
        id: mount_animate,
        transition: Transition {
          delay: Some(Duration::from_millis(100).mul_f32((*mount_idx + 1) as f32)),
          duration: Duration::from_millis(150),
          easing: easing::EASE_IN,
          repeat: None,
        },
        prop: prop!(item.transform),
        from: Transform::translation(-400., 0. ),
      }
      finally {
        let_watch!(checkbox.checked)
          .subscribe(move |v| this.silent().tasks[idx].done = v);
      }
    }
  }
}
```

We did it 🎉
