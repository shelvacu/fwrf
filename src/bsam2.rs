use std::simd::Simd;
pub struct Bsam2<V> {
    _keys: Vec<Simd<u64, 8>>,
    _vals: Vec<V>,
}

pub struct KeysIter<'a> {
    keys: &'a [Simd<u64, 8>],
    idx: usize,
}

impl<'a> std::iter::Iterator for KeysIter<'a> {
    type Item = u64;
    
    fn next(&mut self) -> Option<Self::Item> {
        let res = self.keys.get(self.idx/8).map(|simd| simd[self.idx%8]);
        self.idx += 1;
        res
    }
}

impl<V> Bsam2<V> {
    pub fn from_iter<K, F>(thing: impl IntoIterator<Item = (K, V)>, mut dummy: F) -> Self
    where
      K: Copy + From<u64>,
      u64: From<K>,
      F: FnMut() -> V,
    {
        let iter = thing.into_iter();
        let mut keys = Vec::with_capacity(iter.size_hint().0/8);
        let mut vals = Vec::with_capacity(iter.size_hint().0);

        let mut chunks = iter.into_iter().map(|(k,v)| (k.into(), v)).array_chunks::<8>();
        for pairs in &mut chunks {
            keys.push(Simd::from_array(pairs.map(|(k,v)| {
                vals.push(v);
                k
            })));
        }

        // deal with chunks.remainder()
        if let Some(rem) = chunks.into_remainder() {
            let mut rem_v:Vec<_> = rem.collect();
            while rem_v.len() < 8 {
                let prev_key:u64 = rem_v.last().unwrap().0;
                let new_key = prev_key.checked_add(1).expect("Not enough keyspace for dummy values");
                rem_v.push((new_key, dummy()));
            }
            let (new_keys, new_values):(Vec<_>, Vec<_>) = rem_v.into_iter().unzip();
            keys.push(Simd::from_slice(new_keys.as_slice()));
            vals.extend(new_values);
        }

        let res = Self {
            _keys: keys,
            _vals: vals,
        };

        if res.simd_keys().is_empty() {
            return res;
        }

        let mut keys_iter = res.keys();
        let mut prev = keys_iter.next().unwrap();

        for next in keys_iter {
            if prev >= next {
                panic!("ordering violated; iterator must return orderd keys");
            }
            prev = next;
        }
        
        res
    }

    pub fn simd_keys(&self) -> &[Simd<u64, 8>] {
        &self._keys
    }

    pub fn keys(&self) -> impl Iterator<Item = u64> + '_ {
        KeysIter{
            keys: self.simd_keys(),
            idx: 0,
        }
    }
}