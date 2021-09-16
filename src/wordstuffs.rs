use std::cmp::Ordering;
use std::ops::{Index, IndexMut};
use std::convert::TryInto;

use fnv::FnvHashMap;

use crate::config::*;
use crate::echar::*;
use crate::charset::CharSet;

#[derive(PartialEq,Eq,PartialOrd,Ord,Debug,Copy,Clone,Hash)]
pub struct Word<const N:usize>([EncodedChar; N]);

impl<const N:usize> Default for Word<N> {
    fn default() -> Self {
        Self([NULL_CHAR; N])
    }
}

impl<const N:usize> core::ops::Deref for Word<N> {
    type Target = [EncodedChar; N];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<const N:usize> core::ops::DerefMut for Word<N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub type TallWord = Word<WORD_SQUARE_HEIGHT>;
pub type WideWord = Word<WORD_SQUARE_WIDTH>;

const HEIGHT_MINUS_ONE:usize = WORD_SQUARE_HEIGHT - 1;
const WIDTH_MINUS_ONE:usize  = WORD_SQUARE_WIDTH  - 1;
const SIZE_MINUS_ONE:usize   = WORD_SQUARE_SIZE   - 1;

pub type RowIndex = deranged::Usize<0, HEIGHT_MINUS_ONE>;
pub type ColIndex = deranged::Usize<0, WIDTH_MINUS_ONE>;
pub type MatrixFlatIndex = deranged::Usize<0, SIZE_MINUS_ONE>;

impl Index<RowIndex> for TallWord {
    type Output = EncodedChar;

    fn index(&self, idx: RowIndex) -> &Self::Output {
        let i:usize = idx.into();
        #[cfg(feature = "unchecked")]
        unsafe {
            self.0.get_unchecked(i)
        }
        #[cfg(not(feature = "unchecked"))]
        self.0.get(i).unwrap()
    }
}

#[cfg(not(feature = "square"))]
impl Index<ColIndex> for WideWord {
    type Output = EncodedChar;

    fn index(&self, idx: ColIndex) -> &Self::Output {
        let i:usize = idx.into();
        #[cfg(feature = "unchecked")]
        unsafe {
            self.0.get_unchecked(i)
        }
        #[cfg(not(feature = "unchecked"))]
        self.0.get(i).unwrap()
    }
}

impl IndexMut<RowIndex> for TallWord {
    fn index_mut(&mut self, idx: RowIndex) -> &mut Self::Output {
        let i:usize = idx.into();
        #[cfg(feature = "unchecked")]
        unsafe {
            self.0.get_unchecked_mut(i)
        }
        #[cfg(not(feature = "unchecked"))]
        self.0.get_mut(i).unwrap()
    }
}

#[cfg(not(feature = "square"))]
impl IndexMut<ColIndex> for WideWord {
    fn index_mut(&mut self, idx: ColIndex) -> &mut Self::Output {
        let i:usize = idx.into();
        #[cfg(feature = "unchecked")]
        unsafe {
            self.0.get_unchecked_mut(i)
        }
        #[cfg(not(feature = "unchecked"))]
        self.0.get_mut(i).unwrap()
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct MatrixIndex{
    pub row: RowIndex,
    pub col: ColIndex,
}

impl MatrixIndex {
    pub const ZERO:Self = MatrixIndex{row: RowIndex::MIN, col: ColIndex::MIN};

    pub fn into_flat_index(self) -> MatrixFlatIndex {
        let r:usize = self.row.into();
        let c:usize = self.col.into();
        let f:usize = (r*WORD_SQUARE_WIDTH) + c;
        #[cfg(feature = "unchecked")]
        unsafe { MatrixFlatIndex::new_unchecked(f) }
        #[cfg(not(feature = "unchecked"))]
        MatrixFlatIndex::new(f).unwrap()
    }

    pub fn each_cell_in_row(row: RowIndex) -> impl Iterator<Item = MatrixIndex> {
        ColIndex::all_values().map(move |col| MatrixIndex{row, col})
    }

    pub fn each_cell_in_col(col: ColIndex) -> impl Iterator<Item = MatrixIndex> {
        RowIndex::all_values().map(move |row| MatrixIndex{row, col})
    }

    pub fn inc(self) -> Option<Self> {
        if let Some(new_col) = self.col.checked_add(1) {
            return Some(Self{
                row: self.row,
                col: new_col,
            })
        }
        if let Some(new_row) = self.row.checked_add(1) {
            return Some(Self{
                row: new_row,
                col: ColIndex::MIN,
            })
        }
        None
    }
    
    pub fn dec(self) -> Option<Self> {
        if let Some(new_col) = self.col.checked_sub(1) {
            return Some(Self{
                row: self.row,
                col: new_col,
            })
        }
        if let Some(new_row) = self.row.checked_sub(1) {
            return Some(Self{
                row: new_row,
                col: ColIndex::MAX,
            })
        }
        None
    }
}

impl Ord for MatrixIndex {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.row.cmp(&other.row) {
            Ordering::Equal => {
                self.col.cmp(&other.col)
            },
            ord => ord
        }
    }
}

impl PartialOrd for MatrixIndex {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug,Copy,Clone,Eq,PartialEq,Ord,PartialOrd)]
pub struct GenericMatrix<T>([T; WORD_SQUARE_SIZE]);

impl<T> GenericMatrix<T> {
    #[must_use]
    pub fn map<F, U>(self, mut f: F) -> GenericMatrix<U> 
    where 
        F: FnMut(T) -> U, 
        U: Default + Copy,
        T: Copy 
    {
        let mut other:GenericMatrix<U> = Default::default();
        let mut idx = MatrixIndex::ZERO;
        loop {
            other[idx] = f(self[idx]);
            idx = if let Some(new) = idx.inc() { new } else { break; };
        }
        other
    }
}

impl<T: Default + Copy> Default for GenericMatrix<T> {
    fn default() -> Self {
        Self([Default::default(); WORD_SQUARE_SIZE])
    }
}

pub type WordMatrix = GenericMatrix<EncodedChar>;

impl<T> Index<MatrixIndex> for GenericMatrix<T> {
    type Output = T;

    fn index(&self, idx: MatrixIndex) -> &Self::Output {
        &self.0[idx.into_flat_index()]
    }
}

impl<T> IndexMut<MatrixIndex> for GenericMatrix<T> {
    fn index_mut(&mut self, idx: MatrixIndex) -> &mut Self::Output {
        &mut self.0[idx.into_flat_index()]
    }
}

// pub trait Dimension {
//     type Word;
//     type Index;
//     type Cross: Dimension;

//     fn index_matrix(matrix: WordMatrix, i: Self::Index) -> Self::Word;

//     fn cross() -> Self::Cross;

//     fn prefix_map(map: &WordPrefixMap) -> &TheMap<Self::Word,CharSet>;

//     fn prefix_map_mut(map: &mut WordPrefixMap) -> &mut TheMap<Self::Word,CharSet>;

//     fn get_my_index(mi: MatrixIndex) -> Self::Index;

//     fn get_word_intersecting_point(matrix: WordMatrix, point: MatrixIndex) -> Self::Word {
//         Self::index_matrix(matrix, Self::get_my_index(point))
//     }

//     fn size() -> usize;
// }

// #[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
// pub struct DimRow;
// #[derive(Debug,Clone,Copy,PartialEq,Eq,PartialOrd,Ord)]
// pub struct DimCol;

pub mod dim_row {
    use super::*;
    pub type Word = WideWord;
    pub type Index = RowIndex;
    pub use super::dim_col as cross;

    pub const DIMENSION_ID:usize = 0;

    pub fn index_matrix(matrix: WordMatrix, i: Index) -> Word {
        let mut res:Word = Default::default();
        for mi in MatrixIndex::each_cell_in_row(i) {
            res[mi.col] = matrix[mi];
        }
        res
    }

    pub fn get_my_index(mi: MatrixIndex) -> Index {
        mi.row
    }

    pub fn get_word_intersecting_point(matrix: WordMatrix, point: MatrixIndex) -> Word {
        index_matrix(matrix, get_my_index(point))
    }

    pub fn prefix_map(map: &WordPrefixMap) -> &TheMap<Word,CharSet> {
        map.rows()
    }

    pub fn prefix_map_mut(map: &mut WordPrefixMap) -> &mut TheMap<Word,CharSet> {
        map.rows_mut()
    }

    pub fn size() -> usize { WORD_SQUARE_WIDTH }
}

pub mod dim_col {
    use super::*;
    pub type Word = TallWord;
    pub type Index = ColIndex;
    pub use super::dim_row as cross;

    pub const DIMENSION_ID:usize = 1;

    pub fn index_matrix(matrix: WordMatrix, i: Index) -> Word {
        let mut res:Word = Default::default();
        for mi in MatrixIndex::each_cell_in_col(i) {
            res[mi.row] = matrix[mi];
        }
        res
    }

    pub fn get_my_index(mi: MatrixIndex) -> Index {
        mi.col
    }

    pub fn get_word_intersecting_point(matrix: WordMatrix, point: MatrixIndex) -> Word {
        index_matrix(matrix, get_my_index(point))
    }

    pub fn prefix_map(map: &WordPrefixMap) -> &TheMap<Word,CharSet> {
        map.cols()
    }

    #[cfg(not(feature = "square"))]
    pub fn prefix_map_mut(map: &mut WordPrefixMap) -> &mut TheMap<Word,CharSet> {
        map.cols_mut()
    }

    #[cfg(feature = "square")]
    pub fn prefix_map_mut(map: &mut WordPrefixMap) -> &mut TheMap<Word,CharSet> {
        unreachable!()
    }

    pub fn size() -> usize { WORD_SQUARE_HEIGHT }
}

type TheMap<K, V> = FnvHashMap<K, V>;

#[derive(Debug,Default)]
pub struct WordPrefixMap {
    inner_rows: TheMap<WideWord,CharSet>,
    #[cfg(not(feature = "square"))]
    inner_cols: TheMap<TallWord,CharSet>,
}

impl WordPrefixMap {
    pub fn rows(&self) -> &TheMap<WideWord,CharSet> {
        &self.inner_rows
    }

    pub fn cols(&self) -> &TheMap<TallWord,CharSet> {
        #[cfg(not(feature = "square"))]
        return &self.inner_cols;
        #[cfg(feature = "square")]
        return self.rows();
    }

    pub fn rows_mut(&mut self) -> &mut TheMap<WideWord,CharSet> {
        &mut self.inner_rows
    }

    #[cfg(not(feature = "square"))]
    pub fn cols_mut(&mut self) -> &mut TheMap<TallWord,CharSet> {
        &mut self.inner_cols
    }
}

#[macro_export]
macro_rules! each_dimension {
    ($dim_name:ident, $block:expr) => {
        {
            let res1 = {
                use $crate::wordstuffs::dim_row as $dim_name;
                $block
            };
            let res2 = {
                use $crate::wordstuffs::dim_col as $dim_name;
                $block
            };
            (res1, res2)
        }
    };
}

#[macro_export]
macro_rules! each_unique_dimension {
    ($dim_name:ident, $block:expr) => {
        {
            {
                use $crate::wordstuffs::dim_row as $dim_name;
                $block
            };
            #[cfg(not(feature = "square"))]
            {
                use $crate::wordstuffs::dim_col as $dim_name;
                $block
            };
        }
    };
}