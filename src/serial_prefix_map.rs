use std::marker::PhantomData;
use std::ptr::NonNull;

use crate::wordstuffs::*;
use crate::charset::*;
use crate::echar::*;
use crate::config::*;

type Offset = u16;

type Line = [Offset; CHAR_SET_SIZE];

fn line_to_charset(line: Line) -> CharSet {
    let mut res = CharSet::default();
    for i in CharSetRanged::all_values() {
        if line[i] > 0 {
            res.set(i.into());
        }
    }
    res
}

pub struct SerialPrefixMaps {
    inner_rows: SingleDimSerialPrefixMap,
    #[cfg(not(feature = "square"))]
    inner_cols: SingleDimSerialPrefixMap,
}

impl SerialPrefixMaps {
    pub fn new(map: &WordPrefixMap) -> Self {
        Self {
            inner_rows: SingleDimSerialPrefixMap::build(map.rows()),
            #[cfg(not(feature = "square"))]
            inner_cols: SingleDimSerialPrefixMap::build(map.cols()),
        }
    }

    pub fn rows(&self) -> &SingleDimSerialPrefixMap {
        &self.inner_rows
    }

    #[cfg(feature = "square")]
    pub fn cols(&self) -> &SingleDimSerialPrefixMap {
        &self.inner_rows
    }

    #[cfg(not(feature = "square"))]
    pub fn cols(&self) -> &SingleDimSerialPrefixMap {
        &self.inner_cols
    }
}

pub struct SingleDimSerialPrefixMap {
    arena: Vec<Line>,
}

impl SingleDimSerialPrefixMap {
    pub fn build<const N: usize>(
        words: &TheMap<Word<N>, CharSet>,
    ) -> Self {
        let mut arena = vec![];

        Self::inner_build(
            words,
            &mut arena,
            0,
            Word([NULL_CHAR; N])
        );

        Self { arena }
    }

    fn inner_build<const N: usize>(
        words: &TheMap<Word<N>, CharSet>,
        arena: &mut Vec<Line>, //we need a reference to a vec (and not a mut slice) so that we can grow it if need be
        index: usize,
        word: Word<N>,
    ) -> Option<usize> {
        let mut i = 0;
        loop {
            if i >= N {
                return None;
            }
            if word.0[i] == NULL_CHAR {
                break;
            }
            i += 1;
        }
        let first_null = i;

        while index >= arena.len() {
            arena.push([0; CHAR_SET_SIZE])
        }
        //let my_line = &mut arena[index];
        let charset = words[&word];
        let mut end = index + 1;
        for i in CharSetRanged::all_values() {
            if charset.has(i.into()) {
                let mut new_word = word;
                new_word[first_null] = i.into();
                let offset = end - index;
                if offset >= Offset::MAX.into():usize {
                    panic!("Offset is not big enough for tree");
                }
                arena[index][i] = offset.try_into().unwrap();
                if let Some(new_end) =  Self::inner_build(
                    words,
                    arena,
                    end,
                    new_word,
                ) {
                    end = new_end
                } else { arena[index][i] = Offset::MAX }
            }
        }

        Some(end)
    }

    pub fn top(&self) -> Evil {
        Evil {
            ptr: NonNull::new(self.arena.as_ptr() as *mut Line).unwrap(),
            life: PhantomData,
        }
    }
}

#[derive(Copy,Clone)]
pub struct Evil<'a> {
    ptr: NonNull<Line>,
    life: PhantomData<&'a Line>
}

impl<'a> Evil<'a> {
    pub unsafe fn get_unchecked(self, i: CharSetRanged) -> Evil<'a> {
        let ptr = self.ptr.as_ptr();
        let line:Line = *ptr;
        let offset = line[i];
        #[cfg(not(feature = "unchecked"))]
        if offset == 0 || offset == Offset::MAX { panic!(); }

        Self {
            ptr: NonNull::new_unchecked(ptr.offset(offset as isize)),
            life: PhantomData,
        }
    }

    // pub fn get(self, i: CharSetRanged) -> Option<Evil<'a>> {
    //     let ptr = self.ptr.as_ptr();
    //     let line:Line = unsafe { *ptr };
    //     let offset = line[i];
    //     if offset == 0 || offset == Offset::MAX {
    //         None
    //     } else {
    //         Some(Self{
    //             ptr: unsafe { NonNull::new_unchecked(ptr.offset(offset as isize)) },
    //             life: PhantomData,
    //         })
    //     }
    // }

    pub fn line(self) -> Line {
        unsafe { *self.ptr.as_ptr() }
    }

    pub fn charset(self) -> CharSet {
        line_to_charset(self.line())
    }
}