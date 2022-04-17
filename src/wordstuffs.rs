use core::convert::{TryFrom, TryInto};
use core::cmp::Ordering;
use core::ops::{Index, IndexMut};
use core::fmt;

use fnv::FnvHashMap;

use crate::config::*;
use crate::echar::*;
use crate::serial_prefix_map::*;
use crate::charset::CharSet;

#[derive(PartialEq,Eq,PartialOrd,Ord,Copy,Clone,Hash)]
pub struct Word<const N:usize>(pub [EncodedChar; N]);

impl<const N:usize> fmt::Debug for Word<N> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "W{}({})", N, self.0.iter().map(|e| (*e).into():char).collect():String)
    }
}

impl<const N:usize> Word<N> {
    pub fn is_match(self, other: Self) -> bool {
        IntoIterator::into_iter(self.0.zip(other.0)).all(|(a,b)| a.is_match(b))
    }

    pub fn prefixes(self, pattern: Self) -> Vec<(Self, EncodedChar)> {
        let mut mod_self = self;
        let mut res = Vec::new();
        for i in (0..N).rev() {
            if pattern[i] == NULL_CHAR {
                let c = mod_self[i];
                mod_self[i] = NULL_CHAR;
                res.push((mod_self, c));
            } else {
                #[cfg(test)]
                assert_eq!(pattern[i], mod_self[i]);
                #[cfg(test)]
                assert_eq!(mod_self[i], self[i]);
            }
        }
        res
    }

    pub fn as_slice(&self) -> &[EncodedChar] {
        self.0.as_slice()
    }

    #[allow(dead_code)]
    pub fn from_str_with_nulls(s: &str) -> Result<Self, WordConversionError> {
        Self::from_str(s, true)
    }

    #[allow(dead_code)]
    pub fn from_str_no_nulls(s: &str) -> Result<Self, WordConversionError> {
        Self::from_str(s, false)
    }

    pub fn from_str(s: &str, nulls_allowed: bool) -> Result<Self, WordConversionError> {
        let mut res:Self = Default::default();
        let chars:Vec<_> = s.chars().collect();
        if chars.len() != N { return Err(WordConversionError::WrongLength) }
        //Gonna have to straight disagree with clippy here, this is the clearer way to do this
        #[allow(clippy::needless_range_loop)]
        for i in 0..N {
            res.0[i] = match chars[i].try_into() {
                Err(e) => return Err(WordConversionError::UnencodeableChar(i, e)),
                Ok(v) if !nulls_allowed && v == NULL_CHAR => return Err(WordConversionError::NullChar),
                Ok(v) => v,
            }
        }
        Ok(res)

    }
}

#[cfg(feature = "default-tests")]
#[test]
fn prefixes_work() {
    let pattern:Word<4> = Word::from_str_with_nulls("&&a&").unwrap();
    let word:Word<4> = Word::from_str_with_nulls("star").unwrap();
    let mut test_prefixes = word.prefixes(pattern);
    let mut expected_prefixes:Vec<(Word<4>,EncodedChar)> = vec![
        ("sta&",'r'),
        ("s&a&",'t'),
        ("&&a&",'s')
    ].into_iter().map(|(w,c)| (Word::from_str_with_nulls(w).unwrap(), c.try_into().unwrap())).collect();
    test_prefixes.sort();
    expected_prefixes.sort();
    assert_eq!(test_prefixes, expected_prefixes);
}

#[cfg(feature = "default-tests")]
#[test]
fn prefixes_work_degenerate() {
    let pattern:Word<4> = Word::from_str_with_nulls("&&&&").unwrap();
    let word:Word<4> = Word::from_str_with_nulls("star").unwrap();
    let mut test_prefixes = word.prefixes(pattern);
    let mut expected_prefixes:Vec<(Word<4>,EncodedChar)> = vec![
        ("sta&",'r'),
        ("st&&",'a'),
        ("s&&&",'t'),
        ("&&&&",'s'),
    ].into_iter().map(|(w,c)| (Word::from_str_with_nulls(w).unwrap(), c.try_into().unwrap())).collect();
    test_prefixes.sort();
    expected_prefixes.sort();
    assert_eq!(test_prefixes, expected_prefixes);
}

#[cfg(feature = "default-tests")]
#[test]
fn not_match() {
    let a:Word<5> = Word::from_str_with_nulls("&cb&&").unwrap();
    let b:Word<5> = Word::from_str_with_nulls("items").unwrap();
    assert!(!a.is_match(b));
}

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

#[derive(Debug,PartialEq,Eq)]
pub enum WordConversionError {
    WrongLength,
    UnencodeableChar(usize, <EncodedChar as TryFrom<char>>::Error),
    NullChar,
}

// impl<const N:usize> TryFrom<&str> for Word<N> {
//     type Error = WordConversionError;

//     fn try_from(input: &str) -> Result<Self, Self::Error> {
//         let mut res:Self = Default::default();
//         let chars:Vec<_> = input.chars().collect();
//         if chars.len() != N { return Err(WordConversionError::WrongLength) }
//         //Gonna have to straight disagree with clippy here, this is the clearer way to do this
//         #[allow(clippy::needless_range_loop)]
//         for i in 0..N {
//             res.0[i] = match chars[i].try_into() {
//                 Ok(v) => v,
//                 Err(e) => return Err(WordConversionError::UnencodeableChar(i, e)),
//             }
//         }
//         Ok(res)
//     }
// }

pub type TallWord = Word<WORD_SQUARE_HEIGHT>;
pub type WideWord = Word<WORD_SQUARE_WIDTH>;

#[derive(Debug,Clone,Copy,PartialEq,Eq,Hash)]
pub enum EitherWord {
    Tall(TallWord),
    #[cfg(not(feature = "square"))]
    Wide(WideWord),
}

// #[cfg(feature = "square")]
// impl TryFrom<&str> for EitherWord {
//     type Error = WordConversionError;

//     fn try_from(input: &str) -> Result<Self, Self::Error> {
//         Ok(Self::Tall(TallWord::try_from(input)?))
//     }
// }

// #[cfg(not(feature = "square"))]
// impl TryFrom<&str> for EitherWord {
//     type Error = WordConversionError;

//     fn try_from(input: &str) -> Result<Self, Self::Error> {
//         match TallWord::try_from(input) {
//             Ok(v) => return Ok(Self::from(v)),
//             Err(e @ WordConversionError::UnencodeableChar(_,_)) => return Err(e),
//             Err(WordConversionError::WrongLength) => (),
//         }
//         WideWord::try_from(input).map(Self::from)
//     }
// }

impl From<TallWord> for EitherWord {
    fn from(w: TallWord) -> Self {
        Self::Tall(w)
    }
}

#[cfg(not(feature = "square"))]
impl From<WideWord> for EitherWord {
    fn from(w: WideWord) -> Self {
        Self::Wide(w)
    }
}

impl EitherWord {
    pub fn from_str_with_nulls(s: &str) -> Result<Self, WordConversionError> {
        Self::from_str(s, true)
    }

    pub fn from_str_no_nulls(s: &str) -> Result<Self, WordConversionError> {
        Self::from_str(s, false)
    }
}

#[cfg(feature = "square")]
impl EitherWord {
    pub fn from_str(s: &str, nulls_allowed: bool) -> Result<Self, WordConversionError> {
        Ok(Self::Tall(Word::from_str(s, nulls_allowed)?))
    }

    pub fn as_slice(&self) -> &[EncodedChar] {
        match self {Self::Tall(v) => v.as_slice()}
    }

    pub fn tall(self) -> Option<TallWord> {
        Some(match self {Self::Tall(v) => v})
    }

    pub fn wide(self) -> Option<WideWord> {
        Some(match self {Self::Tall(v) => v})
    }

    #[allow(dead_code)]
    pub fn is_tall(self) -> bool { true }

    #[allow(dead_code)]
    pub fn is_wide(self) -> bool { true }
}

#[cfg(not(feature = "square"))]
impl EitherWord {
    pub fn from_str(s: &str, nulls_allowed: bool) -> Result<Self, WordConversionError> {
        match TallWord::from_str(s, nulls_allowed) {
            Ok(v) => return Ok(Self::from(v)),
            Err(WordConversionError::WrongLength) => (),
            Err(e) => return Err(e),
        }
        WideWord::from_str(s, nulls_allowed).map(Self::from)
    }

    pub fn as_slice(&self) -> &[EncodedChar] {
        match self {
            Self::Tall(v) => v.as_slice(),
            Self::Wide(v) => v.as_slice(),
        }
    }

    pub fn tall(self) -> Option<TallWord> {
        match self {
            Self::Tall(v) => Some(v),
            Self::Wide(_) => None,
        }
    }

    pub fn wide(self) -> Option<WideWord> {
        match self {
            Self::Tall(_) => None,
            Self::Wide(v) => Some(v),
        }
    }

    #[allow(dead_code)]
    pub fn is_tall(self) -> bool {
        matches!(self, Self::Tall(_))
    }

    #[allow(dead_code)]
    pub fn is_wide(self) -> bool {
        matches!(self, Self::Wide(_))
    }
}

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

#[derive(Copy,Clone,Eq,PartialEq,Ord,PartialOrd)]
pub struct GenericMatrix<T>(pub [T; WORD_SQUARE_SIZE]);

impl<T: fmt::Debug> fmt::Debug for GenericMatrix<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        writeln!(f, "Matrix(")?;
        for row in RowIndex::all_values() {
            writeln!(f, "    {}",ColIndex::all_values().map(|col| format!("{:?} ", self[MatrixIndex{row,col}])).collect():String)?
        }
        write!(f, ")")
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

    pub fn set_matrix(matrix: &mut WordMatrix, i: Index, val: Word) {
        for mi in MatrixIndex::each_cell_in_row(i) {
            matrix[mi] = val[mi.col]
        }
    }

    pub fn get_my_index(mi: MatrixIndex) -> Index {
        mi.row
    }

    pub fn get_word_intersecting_point(matrix: WordMatrix, point: MatrixIndex) -> Word {
        index_matrix(matrix, get_my_index(point))
    }

    #[cfg(not(feature = "serial"))]
    pub fn prefix_map(map: &WordPrefixMap) -> &TheMap<Word,CharSet> {
        map.rows()
    }

    #[cfg(feature = "serial")]
    pub fn prefix_map(map: &SerialPrefixMaps) -> &SingleDimSerialPrefixMap {
        map.rows()
    }

    pub fn prefix_map_mut(map: &mut WordPrefixMap) -> &mut TheMap<Word,CharSet> {
        map.rows_mut()
    }

    pub fn index_tuple<T,U>(t: &(T, U)) -> &T {
        &t.0
    }

    pub fn index_tuple_mut<T,U>(t: &mut (T, U)) -> &mut T {
        &mut t.0
    }

    pub fn get_from_either(e: EitherWord) -> Option<Word> {
        e.wide()
    }

    pub fn back(mi: MatrixIndex) -> Option<MatrixIndex> {
        if let Some(r) = mi.row.checked_sub(1) {
            Some(MatrixIndex{
                row: r,
                col: mi.col,
            })
        } else { None }
    }
}

#[cfg_attr(feature = "square", allow(dead_code))]
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

    pub fn set_matrix(matrix: &mut WordMatrix, i: Index, val: Word) {
        for mi in MatrixIndex::each_cell_in_col(i) {
            matrix[mi] = val[mi.row]
        }
    }

    pub fn get_my_index(mi: MatrixIndex) -> Index {
        mi.col
    }

    pub fn get_word_intersecting_point(matrix: WordMatrix, point: MatrixIndex) -> Word {
        index_matrix(matrix, get_my_index(point))
    }

    #[cfg(not(feature = "serial"))]
    pub fn prefix_map(map: &WordPrefixMap) -> &TheMap<Word,CharSet> {
        map.cols()
    }

    #[cfg(feature = "serial")]
    pub fn prefix_map(map: &SerialPrefixMaps) -> &SingleDimSerialPrefixMap {
        map.cols()
    }

    #[cfg(not(feature = "square"))]
    pub fn prefix_map_mut(map: &mut WordPrefixMap) -> &mut TheMap<Word,CharSet> {
        map.cols_mut()
    }

    #[cfg(feature = "square")]
    pub fn prefix_map_mut(_map: &mut WordPrefixMap) -> &mut TheMap<Word,CharSet> {
        unreachable!()
    }

    pub fn index_tuple<T,U>(t: &(U, T)) -> &T {
        &t.1
    }

    pub fn index_tuple_mut<T,U>(t: &mut (U, T)) -> &mut T {
        &mut t.1
    }

    pub fn get_from_either(e: EitherWord) -> Option<Word> {
        e.tall()
    }

    pub fn back(mi: MatrixIndex) -> Option<MatrixIndex> {
        if let Some(c) = mi.col.checked_sub(1) {
            Some(MatrixIndex{
                row: mi.row,
                col: c,
            })
        } else { None }
    }
}

// This little hack is extremely useful for IDE completions when using the each_dimension macros
#[cfg(debug)]
#[allow(dead_code)]
pub use dim_row as dim;

#[cfg(not(feature = "btreemap"))]
pub type TheMap<K, V> = FnvHashMap<K, V>;
#[cfg(feature = "btreemap")]
pub type TheMap<K, V> = std::collections::BTreeMap<K, V>;

pub type TheSet<V> = fnv::FnvHashSet<V>;

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