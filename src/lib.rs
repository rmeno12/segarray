use std::ops::{Index, IndexMut};

pub struct SegArray<T>
where
    T: std::fmt::Debug + Default,
{
    count: u32,
    used_segments: u32,
    segments: [Option<Box<[T]>>; 32],
}

impl<T> SegArray<T>
where
    T: std::fmt::Debug + Default + Default,
{
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
        todo!()
    }

    // TODO: actual error types
    fn grow(&mut self, new_count: u32) -> Result<(), ()> {
        let new_segment_count = Self::segment_count_for_capacity(new_count);
        let old_segment_count = self.used_segments;
        if new_segment_count <= old_segment_count {
            return Ok(());
        }

        for i in old_segment_count..new_segment_count {
            let idx = old_segment_count + i;
            debug_assert!(idx < 32);
            self.segments[idx as usize] = Some({
                let mut v = Vec::new();
                v.resize_with(1 << idx, T::default);
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

impl<T> Index<u32> for SegArray<T>
where
    T: std::fmt::Debug + Default,
{
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

impl<T> IndexMut<u32> for SegArray<T>
where
    T: std::fmt::Debug + Default,
{
    fn index_mut(&mut self, index: u32) -> &mut Self::Output {
        let seg_idx = Self::segment_index(index);
        let seg_slot = Self::segment_slot(index, seg_idx);
        let seg = self.segments[seg_idx as usize]
            .as_mut()
            .expect("Index out of bounds (segment doesn't exist)");
        &mut seg[seg_slot as usize]
    }
}

impl<T> IntoIterator for SegArray<T>
where
    T: std::fmt::Debug + Default,
{
    type Item = T;
    type IntoIter = SegArrayIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        todo!()
    }
}

pub struct SegArrayIterator<T>
where
    T: std::fmt::Debug + Default,
{
    array: SegArray<T>,
    idx: u32,
}

impl<T> Iterator for SegArrayIterator<T>
where
    T: std::fmt::Debug + Default,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
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

        let mut x = 0;
        for item in arr {
            assert_eq!(item, x);
            x += 1;
        }
    }
}
