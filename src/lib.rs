use std::ops::{Index, IndexMut};

pub trait SegArrayObject: Clone + std::fmt::Debug + Default {}

impl<T: Clone + std::fmt::Debug + Default> SegArrayObject for T {}

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
                self[new_count - 1] = value;
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
}
