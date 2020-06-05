use super::DeviceSize;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Array2D<T> {
  array: Vec<T>,
  rows: u32,
  columns: u32,
}

impl<T> Array2D<T> {
  pub fn fill_from(rows: u32, columns: u32, value: T) -> Self
  where
    T: Clone,
  {
    Self {
      rows,
      columns,
      array: vec![value; (rows * columns) as usize],
    }
  }

  #[inline]
  pub fn rows(&self) -> u32 { self.rows }

  #[inline]
  pub fn columns(&self) -> u32 { self.columns }

  #[inline]
  pub fn size(&self) -> DeviceSize { DeviceSize::new(self.columns(), self.rows()) }

  /// Use `data` to fill a sub range of this 2d array, start from `row_start`
  /// row and `col_start` column with `rows` rows. `data`'s len should greater
  /// than `columns`, and rows decide by `data.len()` an `columns`.
  pub fn copy_from_slice(&mut self, mut row_start: u32, col_start: u32, columns: u32, data: &[T])
  where
    T: Copy,
  {
    let mut offset = 0;
    while offset < data.len() && row_start < self.rows {
      let column_end = offset + columns as usize;
      self[row_start][col_start as usize..(col_start + columns) as usize]
        .copy_from_slice(&data[offset..column_end]);
      offset = column_end;
      row_start += 1;
    }
  }

  #[inline]
  pub fn data(&self) -> &[T] { &self.array }

  #[inline]
  pub fn raw_data(&self) -> &[u8] {
    let bytes = std::mem::size_of::<T>();
    unsafe {
      std::slice::from_raw_parts(
        self.array.as_slice() as *const _ as *const u8,
        self.array.len() * bytes,
      )
    }
  }

  #[inline]
  pub fn fill(&mut self, v: T)
  where
    T: Clone,
  {
    self.array.fill(v);
  }
}

impl<T> std::ops::Index<u32> for Array2D<T> {
  type Output = [T];
  fn index(&self, index: u32) -> &Self::Output {
    let array_offset = (index * self.columns) as usize;
    &self.array[array_offset..array_offset + self.columns as usize]
  }
}

impl<T> std::ops::IndexMut<u32> for Array2D<T> {
  fn index_mut(&mut self, index: u32) -> &mut Self::Output {
    let array_offset = (index * self.columns) as usize;
    &mut self.array[array_offset..array_offset + self.columns as usize]
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  #[test]
  fn copy_from_slice() {
    let mut array = Array2D::fill_from(8, 8, 0);

    array.copy_from_slice(0, 0, 2, &[00, 01]);
    assert_eq!(&array[0][0..4], &[00, 01, 0, 0]);

    array.copy_from_slice(7, 3, 1, &[73, 74]);
    assert_eq!(array[7][3], 73);

    array.copy_from_slice(3, 4, 2, &[34, 35, 44, 45]);
    assert_eq!(&array[3][4..], &[34, 35, 0, 0]);
    assert_eq!(&array[4][4..], &[44, 45, 0, 0]);
  }
}
