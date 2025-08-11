use std::ops::{Index, IndexMut};

pub trait SegArrayObject: Clone + std::fmt::Debug + Default {}

impl<T: Clone + std::fmt::Debug + Default> SegArrayObject for T {}

#[derive(Debug, Clone)]
pub struct SegArray<T: SegArrayObject> {
    count: u32,
    used_segments: u32,
    segments: [Option<Box<[T]>>; 32],
}

impl<T: SegArrayObject> Default for SegArray<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: SegArrayObject> SegArray<T> {
    pub fn new() -> Self {
        Self {
            count: 0,
            used_segments: 0,
            segments: Default::default(),
        }
    }

    pub fn len(&self) -> usize {
        self.count as usize
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn append(&mut self, value: T) {
        let new_count = self.count + 1;
        match self.grow(new_count) {
            Ok(()) => {
                self.count = new_count;
                self[new_count - 1] = value;
            }
            Err(e) => {
                panic!("Failed to grow: {e:?}")
            }
        }
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.count == 0 {
            return None;
        }

        let idx = self.count - 1;
        let res = self[idx].clone();
        self.count = idx;

        Some(res)
    }

    // TODO: actual error types
    fn grow(&mut self, new_count: u32) -> Result<(), ()> {
        let new_segment_count = Self::segment_count_for_capacity(new_count);
        let old_segment_count = self.used_segments;
        if new_segment_count <= old_segment_count {
            return Ok(());
        }

        for i in old_segment_count..new_segment_count {
            debug_assert!(i < 32);
            self.segments[i as usize] = Some({
                let mut v = Vec::new();
                v.resize_with(1 << i, T::default);
                v.into_boxed_slice()
            });
        }
        self.used_segments = new_segment_count;

        Ok(())
    }

    fn segment_index(index: u32) -> u32 {
        (index + 1).ilog2()
    }

    fn segment_slot(index: u32, segment_index: u32) -> u32 {
        index + 1 - (1 << (segment_index))
    }

    fn segment_count_for_capacity(capacity: u32) -> u32 {
        ilog2_ceil(capacity + 1)
    }
}

impl<T: SegArrayObject> Index<u32> for SegArray<T> {
    type Output = T;

    fn index(&self, index: u32) -> &Self::Output {
        if index >= self.count {
            panic!("Index out of bounds (out of range)")
        }
        let seg_idx = Self::segment_index(index);
        let seg_slot = Self::segment_slot(index, seg_idx);
        let seg = self.segments[seg_idx as usize]
            .as_ref()
            .expect("Index out of bounds (segment doesn't exist)");
        &seg[seg_slot as usize]
    }
}

impl<T: SegArrayObject> IndexMut<u32> for SegArray<T> {
    fn index_mut(&mut self, index: u32) -> &mut Self::Output {
        if index >= self.count {
            panic!("Index out of bounds (out of range)")
        }
        let seg_idx = Self::segment_index(index);
        let seg_slot = Self::segment_slot(index, seg_idx);
        let seg = self.segments[seg_idx as usize]
            .as_mut()
            .expect("Index out of bounds (segment doesn't exist)");
        &mut seg[seg_slot as usize]
    }
}

impl<T: SegArrayObject> IntoIterator for SegArray<T> {
    type Item = T;
    type IntoIter = SegArrayIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        SegArrayIterator {
            array: self,
            idx: 0,
        }
    }
}

pub struct SegArrayIterator<T: SegArrayObject> {
    array: SegArray<T>,
    idx: u32,
}

impl<T: SegArrayObject> Iterator for SegArrayIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if (self.idx as usize) < self.array.len() {
            let res = self.array[self.idx].clone();
            self.idx += 1;
            Some(res)
        } else {
            None
        }
    }
}

fn ilog2_ceil(x: u32) -> u32 {
    assert!(x != 0);
    let l2 = x.ilog2();
    if 1 << l2 == x {
        l2
    } else {
        l2 + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple() {
        let mut arr: SegArray<i32> = SegArray::new();
        arr.append(1);
        assert_eq!(arr[0], 1);
    }

    #[test]
    fn basic_usage() {
        let mut arr: SegArray<i32> = SegArray::new();

        for i in 0..100 {
            arr.append(i);
            assert_eq!(arr.len(), (i + 1).try_into().unwrap());
        }

        for i in 0..100 {
            assert_eq!(arr[i], i.try_into().unwrap());
        }

        assert_eq!(arr.pop(), Some(99));
        assert_eq!(arr.len(), 99);

        for (x, item) in arr.into_iter().enumerate() {
            assert_eq!(item, x.try_into().unwrap());
        }
    }

    #[test]
    fn test_empty_array() {
        let mut arr: SegArray<u8> = SegArray::new();
        assert!(arr.is_empty());
        assert_eq!(arr.len(), 0);
        assert_eq!(arr.pop(), None);
        assert_eq!(arr.into_iter().next(), None);
    }

    #[test]
    fn test_segment_growth_and_indexing() {
        let mut arr: SegArray<i32> = SegArray::new();
        // The capacity of segments are 1, 2, 4, 8...
        // Total capacities are 1, 3, 7, 15, 31...
        let num_elements = 35;
        for i in 0..num_elements {
            arr.append(i);
            // Verify all previous elements are still correct after each append
            for j in 0..=i {
                assert_eq!(
                    arr[j as u32], j,
                    "Mismatch at index {} after appending {}",
                    j, i
                );
            }
        }
        assert_eq!(arr.len(), num_elements as usize);
        assert_eq!(arr.used_segments, 6); // 1+2+4+8+16+32 > 35, so 6 segments
    }

    #[test]
    fn test_pop_to_empty() {
        let mut arr: SegArray<i32> = SegArray::new();
        for i in 0..50 {
            arr.append(i);
        }

        for i in (0..50).rev() {
            assert_eq!(arr.len(), (i + 1) as usize);
            assert_eq!(arr.pop(), Some(i));
        }

        assert_eq!(arr.len(), 0);
        assert!(arr.is_empty());
        assert_eq!(arr.pop(), None);
    }

    #[test]
    fn test_index_mut() {
        let mut arr: SegArray<i32> = SegArray::new();
        for i in 0..20 {
            arr.append(i);
        }

        // Modify value at the start
        arr[0] = 100;
        assert_eq!(arr[0], 100);

        // Modify value in a later segment (index 10 is in segment 3)
        arr[10] = 200;
        assert_eq!(arr[10], 200);

        // Modify value at the end
        arr[19] = 300;
        assert_eq!(arr[19], 300);
    }

    #[test]
    fn test_iterator_on_multiple_segments() {
        let mut arr: SegArray<i32> = SegArray::new();
        let count = 25;
        for i in 0..count {
            arr.append(i);
        }

        let collected: Vec<i32> = arr.into_iter().collect();
        let expected: Vec<i32> = (0..count).collect();

        assert_eq!(collected, expected);
    }

    #[test]
    fn test_with_string_type() {
        let mut arr: SegArray<String> = SegArray::new();
        arr.append("hello".to_string());
        arr.append("world".to_string());

        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0], "hello".to_string());
        assert_eq!(arr[1], "world".to_string());

        assert_eq!(arr.pop(), Some("world".to_string()));
        assert_eq!(arr.pop(), Some("hello".to_string()));
        assert_eq!(arr.pop(), None);
    }

    #[test]
    #[should_panic(expected = "Index out of bounds (out of range)")]
    fn test_out_of_bounds_panic() {
        let mut arr: SegArray<i32> = SegArray::new();
        arr.append(10);
        arr.append(20);

        // This should panic
        let _ = arr[2];
    }

    #[test]
    fn test_internal_indexing_helpers() {
        // segment_index(index) -> (index + 1).ilog2()
        assert_eq!(SegArray::<i32>::segment_index(0), 0); // 1.ilog2() -> 0
        assert_eq!(SegArray::<i32>::segment_index(1), 1); // 2.ilog2() -> 1
        assert_eq!(SegArray::<i32>::segment_index(2), 1); // 3.ilog2() -> 1
        assert_eq!(SegArray::<i32>::segment_index(3), 2); // 4.ilog2() -> 2
        assert_eq!(SegArray::<i32>::segment_index(6), 2); // 7.ilog2() -> 2
        assert_eq!(SegArray::<i32>::segment_index(7), 3); // 8.ilog2() -> 3

        // segment_slot(index, seg_idx) -> index + 1 - (1 << seg_idx)
        assert_eq!(SegArray::<i32>::segment_slot(0, 0), 0); // 0+1 - 2^0 = 0
        assert_eq!(SegArray::<i32>::segment_slot(1, 1), 0); // 1+1 - 2^1 = 0
        assert_eq!(SegArray::<i32>::segment_slot(2, 1), 1); // 2+1 - 2^1 = 1
        assert_eq!(SegArray::<i32>::segment_slot(3, 2), 0); // 3+1 - 2^2 = 0
        assert_eq!(SegArray::<i32>::segment_slot(6, 2), 3); // 6+1 - 2^2 = 3
        assert_eq!(SegArray::<i32>::segment_slot(7, 3), 0); // 7+1 - 2^3 = 0
    }
}
