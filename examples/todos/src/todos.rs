use std::{
  collections::BTreeMap,
  fs::File,
  io::{self, BufWriter, Write},
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Default)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Default)]
pub struct TaskId(usize);

impl Todos {
  pub fn new_task(&mut self, label: String) -> TaskId {
    let id = self.next_id;
    self.next_id = self.next_id.next();

    self
      .tasks
      .insert(id, Task { id, label, complete: false });
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
      .ok()
      .and_then(|v| serde_json::from_slice(v.as_slice()).ok())
      .unwrap_or_else(|| Todos { tasks: BTreeMap::new(), next_id: TaskId(0) })
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
