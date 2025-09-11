use std::{
    alloc::Layout,
    marker::PhantomData,
    mem::ManuallyDrop,
    ops::{Index, IndexMut},
};

#[derive(Debug, Clone)]
pub struct SegArray<T> {
    count: usize,
    allocated_segments: usize,
    segments: [*mut T; 32],
    segment_usage: [usize; 32],
    _marker: PhantomData<T>,
}

impl<T> Default for SegArray<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> SegArray<T> {
    pub fn new() -> Self {
        Self {
            count: 0,
            allocated_segments: 0,
            segments: [std::ptr::null_mut(); 32],
            segment_usage: [0; 32],
            _marker: PhantomData,
        }
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn append(&mut self, value: T) {
        let new_count = self.count + 1;
        match self.grow(new_count) {
            Ok(()) => {
                let seg_idx = Self::segment_index(self.count);
                let seg_slot = Self::segment_slot(self.count, seg_idx);
                unsafe {
                    let write_slot = self.segments[seg_idx].add(seg_slot);
                    std::ptr::write(write_slot, value);
                }
                self.segment_usage[seg_idx] += 1;
                self.count = new_count;
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
        let seg_idx = Self::segment_index(idx);
        let seg_slot = Self::segment_slot(idx, seg_idx);
        let res = unsafe { self.segments[seg_idx].add(seg_slot).read() };
        self.segment_usage[seg_idx] -= 1;
        self.count = idx;

        Some(res)
    }

    // TODO: actual error types
    fn grow(&mut self, new_count: usize) -> Result<(), ()> {
        let new_segment_count = Self::segment_count_for_capacity(new_count);
        let old_segment_count = self.allocated_segments;
        if new_segment_count <= old_segment_count {
            return Ok(());
        }

        for i in old_segment_count..new_segment_count {
            debug_assert!(i < 32);
            self.segments[i] = Self::alloc_seg(1 << i);
            self.segment_usage[i] = 0;
        }
        self.allocated_segments = new_segment_count;

        Ok(())
    }

    fn alloc_seg(len: usize) -> *mut T {
        let layout = Layout::array::<T>(len).expect("Layout error");
        let ptr = unsafe { std::alloc::alloc(layout) as *mut T };
        if ptr.is_null() {
            std::alloc::handle_alloc_error(layout);
        }
        ptr
    }

    fn segment_index(index: usize) -> usize {
        (index + 1).ilog2().try_into().unwrap()
    }

    fn segment_slot(index: usize, segment_index: usize) -> usize {
        index + 1 - (1 << (segment_index))
    }

    fn segment_count_for_capacity(capacity: usize) -> usize {
        ilog2_ceil(capacity + 1)
    }
}

impl<T> Drop for SegArray<T> {
    fn drop(&mut self) {
        if self.allocated_segments == 0 {
            return;
        }

        // Before deallocating the buffers, we have to first drop each of the `T`s in the SegArray
        let currently_filled_segments = Self::segment_count_for_capacity(self.count);
        for i in 0..currently_filled_segments {
            let seg = self.segments[i];
            unsafe {
                let filled_seg_as_slice =
                    std::ptr::slice_from_raw_parts_mut(seg, self.segment_usage[i] - 1);
                std::ptr::drop_in_place(filled_seg_as_slice);
            }
        }

        for i in 0..self.allocated_segments {
            let seg = self.segments[i];
            let layout = Layout::array::<T>(1 << i).unwrap();
            unsafe {
                std::alloc::dealloc(seg as *mut u8, layout);
            }
        }
    }
}

impl<T> Index<usize> for SegArray<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        if index >= self.count {
            panic!(
                "Index out of bounds: index {index} is not less than length {}",
                self.count
            );
        }
        let seg_idx = Self::segment_index(index);
        let seg_slot = Self::segment_slot(index, seg_idx);
        unsafe { &*self.segments[seg_idx].add(seg_slot) }
    }
}

impl<T> IndexMut<usize> for SegArray<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        if index >= self.count {
            panic!(
                "Index out of bounds: index {index} is not less than length {}",
                self.count
            );
        }
        let seg_idx = Self::segment_index(index);
        let seg_slot = Self::segment_slot(index, seg_idx);
        unsafe { &mut *self.segments[seg_idx].add(seg_slot) }
    }
}

impl<T> IntoIterator for SegArray<T> {
    type Item = T;
    type IntoIter = SegArrayIntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        let array = ManuallyDrop::new(self);
        SegArrayIntoIter {
            idx: 0,
            count: array.count,
            allocated_segments: array.allocated_segments,
            segments: array.segments,
            segment_usage: array.segment_usage,
            _marker: PhantomData
        }
    }
}

pub struct SegArrayIntoIter<T> {
    idx: usize,
    count: usize,
    allocated_segments: usize,
    segments: [*mut T; 32],
    segment_usage: [usize; 32],
    _marker: PhantomData<T>,
}

impl<T> Iterator for SegArrayIntoIter<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        if self.idx == self.count {
            None
        } else {
            let seg_idx = SegArray::<T>::segment_index(self.idx);
            let seg_slot = SegArray::<T>::segment_slot(self.idx, seg_idx);
            let item = unsafe { self.segments[seg_idx].add(seg_slot).read() };
            self.idx += 1;
            Some(item)
        }
    }
}

impl<T> Drop for SegArrayIntoIter<T> {
    fn drop(&mut self) {
        // Need to drop all elements in indices [idx, count). The ones before idx have already been
        // moved out, so dropping them is wrong.
        let first_seg_including_drop = SegArray::<T>::segment_count_for_capacity(self.idx + 1) - 1;
        let currently_filled_segments = SegArray::<T>::segment_count_for_capacity(self.count);
        let mut already_dropped = self.idx;
        for i in first_seg_including_drop..currently_filled_segments {
            let drop_slice_start_slot = SegArray::<T>::segment_slot(already_dropped, i);
            let drop_slice = unsafe { self.segments[i].add(drop_slice_start_slot) };
            let drop_slice_len = ((1 << i) - drop_slice_start_slot).min(self.segment_usage[i]);
            unsafe {
                let filled_seg_as_slice =
                    std::ptr::slice_from_raw_parts_mut(drop_slice, drop_slice_len);
                std::ptr::drop_in_place(filled_seg_as_slice);
            }
            already_dropped += drop_slice_len;
        }

        for i in 0..self.allocated_segments {
            let layout = Layout::array::<T>(1 << i).unwrap();
            unsafe {
                std::alloc::dealloc(self.segments[i] as *mut u8, layout);
            }
        }
    }
}

fn ilog2_ceil(x: usize) -> usize {
    assert!(x != 0);
    let l2 = x.ilog2();
    if 1 << l2 == x {
        l2.try_into().unwrap()
    } else {
        (l2 + 1).try_into().unwrap()
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

        for (x, item) in arr.into_iter().take(21).enumerate() {
            println!("{x}");
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
                    arr[j.try_into().unwrap()],
                    j,
                    "Mismatch at index {} after appending {}",
                    j,
                    i
                );
            }
        }
        assert_eq!(arr.len(), num_elements as usize);
        assert_eq!(arr.allocated_segments, 6); // 1+2+4+8+16+32 > 35, so 6 segments
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
    #[should_panic(expected = "Index out of bounds: index 2 is not less than length 2")]
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
