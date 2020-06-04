#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Array2D<T> {
  array: Vec<T>,
  rows: usize,
  columns: usize,
}

impl<T> Array2D<T> {
  pub fn fill_from(rows: usize, columns: usize, value: T) -> Self
  where
    T: Clone,
  {
    Self {
      rows,
      columns,
      array: vec![value; rows * columns],
    }
  }

  #[inline]
  pub fn rows(&self) -> usize { self.rows }

  #[inline]
  pub fn columns(&self) -> usize { self.columns }

  /// Use `data` to fill a sub range of this 2d array, start from `row_start`
  /// row and `col_start` column with `rows` rows. `data`'s len should greater
  /// than `columns`, and rows decide by `data.len()` an `columns`.
  pub fn copy_from_slice(
    &mut self,
    mut row_start: usize,
    col_start: usize,
    columns: usize,
    data: &[T],
  ) where
    T: Copy,
  {
    let mut offset = 0;
    while offset < data.len() && row_start < self.rows {
      let column_end = offset + columns;
      self[row_start][col_start..col_start + columns].copy_from_slice(&data[offset..column_end]);
      offset = column_end;
      row_start += 1;
    }
  }

  #[inline]
  pub fn data(&self) -> &[T] { &self.array }
}

impl<T> std::ops::Index<usize> for Array2D<T> {
  type Output = [T];
  fn index(&self, index: usize) -> &Self::Output {
    let array_offset = index * self.columns;
    &self.array[array_offset..array_offset + self.columns]
  }
}

impl<T> std::ops::IndexMut<usize> for Array2D<T> {
  fn index_mut(&mut self, index: usize) -> &mut Self::Output {
    let array_offset = index * self.columns;
    &mut self.array[array_offset..array_offset + self.columns]
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
