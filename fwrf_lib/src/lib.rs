#![feature(
    type_ascription,
    min_specialization,
    portable_simd,
    iter_array_chunks
)]

pub mod config;
pub mod echar;
pub mod charset;
pub mod wordstuffs;
#[cfg(feature = "serial")]
pub mod serial_prefix_map;
pub mod binary_searched_array_map;
pub mod bsam2;

//use config::*;
use wordstuffs::*;
//use charset::*;
use echar::*;
#[cfg(feature = "serial")]
use serial_prefix_map::*;

// It is assumed that this function does *not* need to be fast, and should be written in whatever way is reasonably fast and most correct and elegant.
pub fn make_prefix_map<I>
(
    template: WordMatrix,
    wordlist: I,
) -> (usize, usize, DynamicWordPrefixMap)
where
    I: IntoIterator<Item = EitherWord>,
{
    let mut word_counts = [0usize; 2];
    let mut res:DynamicWordPrefixMap = Default::default();
    let mut word_templates = (vec![], vec![]);
    each_dimension!(dim, {
        let my_templates = dim::index_tuple_mut(&mut word_templates);
        for i in dim::Index::all_values() {
            let word = dim::index_matrix(template, i);
            my_templates.push(word);
        }
        my_templates.sort();
        my_templates.dedup();
    });
    #[cfg(feature = "square")]
    {
        for el in &word_templates.1 {
            word_templates.0.push(*el);
        }
        word_templates.0.sort();
        word_templates.0.dedup();
    }
    for w in wordlist {
        each_unique_dimension!(dim, {
            if let Some(w) = dim::get_from_either(w) {
                word_counts[dim::DIMENSION_ID] += 1;
                for c in &*w { assert_ne!(*c, NULL_CHAR); }
                for &template in dim::index_tuple(&word_templates) {
                    if template.is_match(w) {
                        let p = w.prefixes(template);
                        for (prefix,c) in p {
                            dim::prefix_map_mut(&mut res).entry(prefix).or_default().set(c);
                        }
                    }
                }
            }
        })
    }
    let row_counts = word_counts[wordstuffs::dim_row::DIMENSION_ID];
    #[cfg(feature = "square")]
    let col_counts = row_counts;
    #[cfg(not(feature = "square"))]
    let col_counts = word_counts[wordstuffs::dim_col::DIMENSION_ID];
    (row_counts, col_counts, res)
}