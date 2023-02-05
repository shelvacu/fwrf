
unsafe fn assume(cond: bool) {
    if !cond {
        std::hint::unreachable_unchecked()
    }
}

#[repr(transparent)]
struct NoTouchy<T>(T);

impl<T> std::ops::Deref for NoTouchy<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct BinarySearchedArrayMap<P: KeyValuePair>
{
    // Safety: data must not be modified
    data: Vec<P>,
}

pub trait KeyValuePair: Sized {
    type K: Ord;
    type V;

    fn key(&self) -> &Self::K;

    fn val(&self) -> &Self::V;
}

// This *is* slightly different from (K,V) because rust is free to reorder/pack as it likes
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Pair<K,V> {
    pub k: K,
    pub v: V,
}

impl<K: Ord,V> KeyValuePair for Pair<K,V> {
    type K = K;
    type V = V;

    fn key(&self) -> &Self::K {
        &self.k
    }

    fn val(&self) -> &Self::V {
        &self.v
    }
}

impl<K,V> From<(K,V)> for Pair<K,V> {
    fn from(tuple: (K,V)) -> Pair<K,V> {
        Pair{
            k: tuple.0,
            v: tuple.1,
        }
    }
}

impl<K,V> From<Pair<K,V>> for (K,V) {
    fn from(pair: Pair<K,V>) -> (K,V) {
        (pair.k, pair.v)
    }
}

pub struct Entry<'a, P: KeyValuePair> {
    // SAFETY: When present is true, index must be a valid index into map
    map: &'a BinarySearchedArrayMap<P>,
    index: usize,
    present: bool,
}

// #[derive(Clone)] doesn't work because it adds constraints K: Clone and V: Clone which aren't necesary
impl<'a, P: KeyValuePair> Clone for Entry<'a, P> {
    fn clone(&self) -> Self {
        Self {
            map: self.map,
            index: self.index,
            present: self.present,
        }
    }
}

impl<'a, P: KeyValuePair> Copy for Entry<'a, P> {}

impl<'a, P: KeyValuePair> Entry<'a, P> {
    pub fn get(&self) -> Option<&P> {
        if self.present {
            Some(unsafe { self.map.data.get_unchecked(self.index) })
        } else { None }
    }

    pub fn present(&self) -> bool {
        self.present
    }

    pub fn entry_near(&self, search: &P::K) -> Self {
        use std::cmp::Ordering::*;
        // Copied straight from rust std slice::binary_search_by and then modified as needed

        if self.data().len() == 0 {
            // If empty, the only allowable Entry is {index: 0, present: false}
            return *self;
        }

        if self.index == self.data().len() {
            // This is a valid index because we ensure the vec isn't empty above
            let last_idx = self.data().len() - 1;
            match unsafe{ self.data().get_unchecked(last_idx) }.key().cmp(search) {
                Less => {
                    return *self;
                },
                Equal => {
                    return Self {
                        map: self.map,
                        index: last_idx,
                        present: true,
                    }
                },
                Greater => {
                    return Self {
                        map: self.map,
                        index: last_idx,
                        present: true,
                    }.entry_near(search)
                },
            }
        }

        // We now know:
        // self.data.len() >= 1
        // self.index < self.data.len() (so self.index points at a valid element)

        //todo: find the right direction, then check +1, +2, +4, +8... away from current until direction switches
        let cmp = (unsafe { self.data().get_unchecked(self.index) }).key().cmp(search);

        let mut jump:usize = 1;
        let mut left:usize = 0;
        let mut right:usize = self.data().len();
        if cmp == Greater {
            right = self.index;
            while right > jump {
                let point = right - jump;

                match (unsafe { self.data().get_unchecked(point) }).key().cmp(search) {
                    Equal => {
                        return Entry{present: true, index: point, ..*self};
                    },
                    Greater => {
                        right = point;
                        jump += jump;
                    },
                    Less => {
                        left = point;
                        break;
                    }
                }
            }
        } else if cmp == Less {
            left = self.index;
            while left + jump < right {
                let point = left + jump;

                match (unsafe { self.data().get_unchecked(point) }).key().cmp(search) {
                    Equal => {
                        return Entry{present: true, index: point, ..*self};
                    },
                    Greater => {
                        right = point;
                        break;
                    },
                    Less => {
                        left = point;
                        jump += jump;
                    }
                }
            }
        } else {
            return Entry{present: true, ..*self};
        }

        let mut size = right - left;

        // INVARIANTS:
        // - 0 <= left <= left + size = right <= self.len()
        // - f returns Less for everything in self[..left]
        // - f returns Greater for everything in self[right..]
        // let mut size = self.data().len();
        // let mut left = 0;
        // let mut right = size;

        while left < right {
            let mid = left + size / 2;

            // SAFETY: the while condition means `size` is strictly positive, so
            // `size/2 < size`.  Thus `left + size/2 < left + size`, which
            // coupled with the `left + size <= self.len()` invariant means
            // we have `left + size/2 < self.len()`, and this is in-bounds.
            let cmp = (unsafe { self.data().get_unchecked(mid) }).key().cmp(search);

            // The reason why we use if/else control flow rather than match
            // is because match reorders comparison operations, which is perf sensitive.
            // This is x86 asm for u8: https://rust.godbolt.org/z/8Y8Pra.
            if cmp == Less {
                left = mid + 1;
            } else if cmp == Greater {
                right = mid;
            } else {
                // SAFETY: same as the `get_unchecked` above
                unsafe { assume(mid < self.data().len()) };
                return Entry{map: self.map, index: mid, present: true};
            }

            size = right - left;
        }

        // SAFETY: directly true from the overall invariant.
        // Note that this is `<=`, unlike the assume in the present:true path.
        unsafe { assume(left <= self.data().len()) };
        Entry{map: self.map, index: left, present: false}
    }

    fn data(&self) -> &Vec<P> {
        &self.map.data
    }
}

impl<K: Ord, V> BinarySearchedArrayMap<Pair<K, V>> {
    pub fn from_sorted(i: impl IntoIterator<Item = (K, V)>) -> Self {
        Self::from_sorted_pairs(i.into_iter().map(Into::into))
    }
}

impl<P: KeyValuePair> BinarySearchedArrayMap<P> {
    pub fn from_sorted_pairs(i: impl IntoIterator<Item = P>) -> Self {
        let iter = i.into_iter();
        let mut me = Self{data: Vec::with_capacity(iter.size_hint().0)};
        for pair in iter {
            if let Some(prev_pair) = me.data.last() {
                if prev_pair.key() > pair.key() {
                    panic!("Iterator returned out-of-order keys in from_sorted");
                }
                if prev_pair.key() == pair.key() {
                    panic!("Iterator returned duplicate keys in from_sorted")
                }
            }
            me.data.push(pair);
        }

        return me;
    }

    pub fn entry(&self, key: &P::K) -> Entry<P> {
        let index;
        let present;
        match self.data.binary_search_by_key(&key, KeyValuePair::key) {
            Ok(i) => {
                index = i;
                present = true;
            },
            Err(i) => {
                index = i;
                present = false;
            }
        }

        return Entry{
            map: self,
            index,
            present,
        }
    }

    pub fn get_first_blank_entry(&self) -> Entry<P> {
        return Entry{
            map: self,
            index: 0,
            present: false,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn it_do_stuff() {
        let bsam = BinarySearchedArrayMap::from_sorted([
            (0, 'a'),
            (1, 'b'),
            (2, 'c'),
        ]);

        let entry = bsam.entry(&1);
        assert_eq!(entry.get(), Some(&Pair::from((1, 'b'))));
        assert_eq!(entry.entry_near(&0).get(), Some(&Pair::from((0, 'a'))));
        assert_eq!(entry.entry_near(&2).get(), Some(&Pair::from((2, 'c'))));
        let three = entry.entry_near(&2);
        assert_eq!(three.entry_near(&0).get(), Some(&Pair::from((0, 'a'))));
        assert_eq!(three.entry_near(&2).entry_near(&1).get(), Some(&Pair::from((1, 'b'))));
        assert_eq!(three.entry_near(&2).entry_near(&2).get(), Some(&Pair::from((2, 'c'))));
    }

    #[test]
    fn it_do_beeg_stuff() {
        let keys:Vec<_> = (0..1000).into_iter().map(|n| n*10).collect();
        let bsam = BinarySearchedArrayMap::from_sorted(keys.iter().copied().map(|n| (n, ())));

        let expect = |n: i32| Some(Pair::from((n, ())));

        assert_eq!(bsam.entry(&0).get(), expect(0).as_ref());
        assert_eq!(bsam.entry(&1).get(), None);
        assert_eq!(bsam.entry(&(9990)).get(), expect(9990).as_ref());
        for init in [-100, -1, 0, 1, 10, 4999, 5000, 9985, 9990, 9999] {
            let entry = bsam.entry(&init);
            for (a, b) in [
                (-1, false),
                (0, true),
                (1, false),
                (10, true),
                (11, false),
                (4990, true),
                (4999, false),
                (5000, true),
                (5001, false),
                (5010, true),
                (5015, false),
                (9980, true),
                (9985, false),
                (9990, true),
                (9999, false),
            ] {
                let thing = expect(a);
                let near_entry = entry.entry_near(&a);
                assert_eq!(near_entry.get(), if b {thing.as_ref()} else {None});
                assert_eq!(near_entry.present(), b);
            }
        }
    }
}