use std::ops::Index;

struct SegArray<T> {
    count: u32,
    used_segments: u32,
    segments: [Option<Box<[T]>>; 32],
}

impl<T> SegArray<T> {
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
        todo!()
    }

    pub fn pop(&mut self) -> Option<T> {
        todo!()
    }

    fn segment_index(index: u32) -> u32 {
        index.ilog2() - 1
    }

    fn segment_slot(index: u32, segment_index: u32) -> u32 {
        index - (1 << (segment_index + 1))
    }

    fn segments_for_count(count: u32) -> u32 {}
}

impl<T> Index<u32> for SegArray<T> {
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

impl<T> IntoIterator for SegArray<T> {
    type Item = T;
    type IntoIter = SegArrayIterator<T>;

    fn into_iter(self) -> Self::IntoIter {
        todo!()
    }
}

struct SegArrayIterator<T> {
    array: SegArray<T>,
    idx: u32,
}

impl<T> Iterator for SegArrayIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
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

    fn basic_usage() {
        let mut arr: SegArray<i32> = SegArray::new();

        for i in 0..100 {
            arr.append(i);
            assert_eq!(arr.len(), (i + 1).try_into().unwrap());
        }

        for i in 0..100 {
            assert_eq!(arr[i], i.try_into().unwrap());
        }

        let mut x = 0;
        for item in arr {
            assert_eq!(item, x);
            x += 1;
        }

        assert_eq!(arr.pop(), 99);
        assert_eq!(arr.len(), 99);
    }
}
