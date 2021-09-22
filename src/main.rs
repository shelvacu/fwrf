#![feature(type_ascription, array_zip, min_specialization)]

mod config;
mod echar;
mod charset;
mod wordstuffs;

use std::convert::TryInto;
use std::io::{self, BufReader};
use std::io::prelude::*;
use std::fs::File;

use clap::{
    App,
    Arg
};

use wordstuffs::*;
use charset::*;
use echar::*;

#[cfg(feature = "do-debug")]
const DEBUG:bool = true;
#[cfg(not(feature = "do-debug"))]
const DEBUG:bool = false;

fn main() -> io::Result<()> {
    let args = App::new(format!("Fast Word Rectangle Finder o{}x{}", config::WORD_SQUARE_WIDTH, config::WORD_SQUARE_HEIGHT))
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .about(clap::crate_description!())
        .arg(Arg::with_name("threads")
            .default_value(if DEBUG { "1" } else {"4"})
            .takes_value(true)
            .validator(|arg| {
                match arg.parse::<u32>() {
                    Ok(_) => Ok(()),
                    Err(e) => Err(format!("Must provide a valid integer. {:?}", e)),
                }
            })
            .help("Number of threads to use.")
            .long("threads")
            .short("t")
        )
        .arg(Arg::with_name("wordlist")
            .required(true)
            .help("the wordlist file path, a plain-text UTF-8 file with each word separated by a newline")
        )
        .arg(Arg::with_name("ignore-empty-wordlist")
            .long("ignore-empty-wordlist")
            .short("e")
            .help("Don't complain if there are no words of the necessary length in the given wordlist")
        )
        .arg(Arg::with_name("ignore-unencodeable")
            .long("ignore-unencodeable")
            .short("u")
            .help("Don't show a warning when a word is dropped because it contains unencodeable characters.")
        )
        .arg(Arg::with_name("quiet")
            .long("quiet")
            .short("q")
            .help("Don't show any status messages; STDERR will be empty if no errors/warnings occured. (See also --ignore-*)")
        )
        .arg(Arg::with_name("must-include")
            .long("must-include")
            .short("m")
            .takes_value(true)
            .help("Only search for word rectangles that include all of the given comma-separated words.")
        )
        .arg(Arg::with_name("fancy-output")
            .long("fancy-output")
            .short("f")
            .help("Shows output word rectangles across multiple lines (easier to see column words that way) and unbuffered. May be a significant slowdown if many results are produced.")
        )
        .get_matches()
    ;
    
    let loud = !args.is_present("quiet");
    let ignore_empty_wordlist = args.is_present("ignore-empty-wordlist");
    let ignore_unencodeable = args.is_present("ignore-unencodeable");
    let fancy = args.is_present("fancy-output");
    let num_threads:u32 = args.value_of("threads").unwrap().parse().unwrap();
    
    let must_include_strings:Vec<String> = args.value_of("must-include").map(|s| s.split(',').map(str::to_string).collect()).unwrap_or_default();

    let mut must_include:Vec<EitherWord> = Vec::new();

    for include_str in &must_include_strings {
        match include_str.as_str().try_into():Result<EitherWord, _> {
            Ok(word) => must_include.push(word),
            Err(WordConversionError::WrongLength) => {
                if ignore_empty_wordlist {
                    std::process::exit(0);
                } else {
                    panic!("Must-use word {:?} length do not match dimensions.", include_str);
                }
            },
            Err(e @ WordConversionError::UnencodeableChar(_,_)) => {
                panic!("Error encoding must-use word {:?} due to {:?}", include_str, e);
            }
        }
    }

    assert_eq!(must_include.len(), must_include_strings.len());

    let templates:Vec<WordMatrix> = make_templates(must_include.as_slice(),vec![Default::default()]);

    if DEBUG {
        dbg!(&templates);
    }

    if templates.is_empty() {
        if ignore_empty_wordlist {
            std::process::exit(0);
        } else {
            panic!("must-use words can not be fit together.");
        }
    }

    if loud {
        eprintln!("Word rectangle order {}x{}", config::WORD_SQUARE_WIDTH, config::WORD_SQUARE_HEIGHT);
        eprintln!("Start: creating index");
    }

    let f = BufReader::new(File::open(args.value_of("wordlist").unwrap())?);

    let mut words = Vec::new();

    for maybe_line in f.lines() {
        let line = maybe_line.unwrap();
        match line.as_str().try_into():Result<EitherWord, _> {
            Ok(w) => words.push(w),
            Err(WordConversionError::WrongLength) => (),
            Err(e) => {
                if !ignore_unencodeable {
                    panic!("Could not encode {:?} due to {:?}", &line, e);
                }
            }
        }
    }

    if !ignore_empty_wordlist && words.is_empty() {
        panic!("No words in wordlist!");
    }

    if loud {
        eprintln!("Starting.");
    }

    let compute_func:Box<dyn 'static + Send + FnOnce(std::sync::mpsc::Receiver<WordMatrix>) -> Result<(), std::io::Error>>;
    if fancy {
        compute_func = Box::new(move |w2m_rx:std::sync::mpsc::Receiver<WordMatrix>| {
            let mut minibuffer = String::new();
            while let Ok(wm) = w2m_rx.recv() {
                for row in RowIndex::all_values() {
                    for col in ColIndex::all_values() {
                        minibuffer.push(wm[MatrixIndex{row,col}].into():char);
                    }
                    minibuffer.push('\n');
                }
                println!("{}", minibuffer);
                minibuffer.truncate(0);
            }
            Ok(())
        })
    } else {
        compute_func = Box::new(move |w2m_rx:std::sync::mpsc::Receiver<WordMatrix>| {
            let mut minibuffer = String::new();
            let mut writer = std::io::BufWriter::with_capacity(1024*1024, std::io::stdout());
            while let Ok(wm) = w2m_rx.recv() {
                for row in RowIndex::all_values() {
                    for col in ColIndex::all_values() {
                        minibuffer.push(wm[MatrixIndex{row,col}].into():char);
                    }
                    if row < RowIndex::MAX {
                        minibuffer.push('|');
                    }
                }
                minibuffer.push('\n');
                writer.write_all(minibuffer.as_bytes())?;
                minibuffer.truncate(0);
            }
            writer.flush()?;
            Ok(())
        })
    }

    let mut time = devtimer::DevTime::new_simple();
    time.start();

    outer_compute(
        words.as_slice(),
        templates.as_slice(),
        num_threads as usize,
        compute_func,
    );

    time.stop();
    if loud {
        eprintln!("Took {} secs", (time.time_in_micros().unwrap() as u64 as f64) / 1_000_000.0)
    }

    Ok(())
}

fn make_templates(
    must_use: &[EitherWord],
    from_templates: Vec<WordMatrix>,
) -> Vec<WordMatrix> {
    let (current_word, rest) = if let Some(v) = must_use.split_last() { v } else { return from_templates };
    let mut to_templates = Vec::new();
    each_dimension!(dim, {
        if let Some(word) = dim::get_from_either(*current_word) {
            for template in &from_templates {
                for i in dim::Index::all_values() {
                    if word.is_match(dim::index_matrix(*template, i)) {
                        let mut new_matrix = *template;
                        dim::set_matrix(&mut new_matrix, i, word);
                        to_templates.push(new_matrix);
                    }
                }
            }
        }
    });
    make_templates(rest, to_templates)
}

fn outer_compute(
    wordlist: &[EitherWord],
    templates: &[WordMatrix],
    num_threads: usize,
    output_func: impl 'static + Send + FnOnce(std::sync::mpsc::Receiver<WordMatrix>) -> Result<(), std::io::Error>,
) {
    // "w2m" => worker threads to output thread
    let (w2m_tx, w2m_rx) = std::sync::mpsc::sync_channel(128);
    let output_thread = std::thread::spawn(move || output_func(w2m_rx));
    for template in templates {
        let (_row_counts, _col_counts, prefix_map) = make_prefix_map(*template, wordlist.iter().copied());

        each_unique_dimension!(dim, {
            if dim::prefix_map(&prefix_map).is_empty() {
                continue;
            }
        });

        // "m2w" => main thread to worker threads
        let (m2w_tx, m2w_rx) = crossbeam_channel::bounded::<WordMatrix>(128);
        let mut worker_handles = Vec::new();

        let prefix_map_arc = std::sync::Arc::new(prefix_map);

        
        for _ in 0..num_threads {
            let rxc = m2w_rx.clone();
            let txc = w2m_tx.clone();
            let my_prefix_map = std::sync::Arc::clone(&prefix_map_arc);
            worker_handles.push(
                std::thread::spawn( move || {
                    while let Ok(msg) = rxc.recv() {
                        compute(
                            &my_prefix_map,
                            msg,
                            MatrixIndex{row: RowIndex::MAX, col: ColIndex::MAX},
                            |a| txc.send(a).unwrap()
                        );
                    }
                })
            );
        }

        let a = &*prefix_map_arc;
        let m = MatrixIndex{row: 1usize.try_into().unwrap(), col: 0usize.try_into().unwrap()};
        let f = |ca| m2w_tx.send(ca).unwrap();
        compute(
            a,
            *template,
            m,
            f,
        );

        drop(m2w_tx);
        for h in worker_handles {
            h.join().unwrap();
        }
    }
    output_thread.join().unwrap().unwrap();
}

// It is assumed that this function does *not* need to be fast, and should be written in whatever way is reasonably fast and most correct and elegant.
fn make_prefix_map<I>
(
    template: WordMatrix,
    wordlist: I,
) -> (usize, usize, WordPrefixMap)
where
    I: IntoIterator<Item = EitherWord>,
{
    let mut word_counts = [0usize; 2];
    let mut res:WordPrefixMap = Default::default();
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

fn compute<F: FnMut(WordMatrix)>(
    prefix_map: &WordPrefixMap,
    orig_matrix: WordMatrix,
    target_idx: MatrixIndex,
    mut on_result: F,
) {
    let mut at_idx = MatrixIndex::ZERO;
    let mut charset_array:GenericMatrix<CharSet> = Default::default();
    let mut is_nullish:GenericMatrix<bool> = GenericMatrix([true; config::WORD_SQUARE_SIZE]);
    let mut matrix = orig_matrix;

    for row in RowIndex::all_values() {
        for col in ColIndex::all_values() {
            let mi = MatrixIndex{row,col};
            if orig_matrix[mi] != NULL_CHAR {
                charset_array[mi].set(orig_matrix[mi]);
            }
        }
    }

    loop {
        if DEBUG {
            dbg!(at_idx,matrix);
        }
        if is_nullish[at_idx] && orig_matrix[at_idx] == NULL_CHAR {
            let (row_set, col_set) = each_dimension!(dim, {
                dim::prefix_map(prefix_map).get(&dim::get_word_intersecting_point(matrix, at_idx)).copied().unwrap_or_default()
            });
            charset_array[at_idx] = row_set.and(col_set);
        }

        if orig_matrix[at_idx] == NULL_CHAR || !is_nullish[at_idx] {
            match matrix[at_idx].inc() {
                Some(e) => matrix[at_idx] = e,
                None => {
                    matrix[at_idx] = orig_matrix[at_idx];
                    is_nullish[at_idx] = true;
                    match at_idx.dec() {
                        Some(i) => {
                            at_idx = i;
                        },
                        None => return,
                    }
                    continue;
                }
            }
        }

        is_nullish[at_idx] = false;
        if charset_array[at_idx].has(matrix[at_idx]) {
            let next = at_idx.inc();
            if next == target_idx.inc() {
                (&mut on_result)(matrix);
            } else if let Some(i) = next {
                at_idx = i;
            } else {
                unreachable!();
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::sync::{Arc,Mutex};
    use super::*;
    use config::{WORD_SQUARE_WIDTH,WORD_SQUARE_HEIGHT};

    #[allow(dead_code)]
    fn assert_results(
        wordlist_str: &[&str],
        must_use_str: &[&str],
        expected_results_str: &[&[&str]],
    ) {
        let mut expected_results:Vec<_> = expected_results_str.iter().map(|str_a| {
            let mut m = WordMatrix::default();
            assert_eq!(str_a.len(), WORD_SQUARE_HEIGHT);
            for rowi in RowIndex::all_values() {
                let row_chars:Vec<_> = str_a[rowi.into():usize].chars().collect();
                assert_eq!(row_chars.len(), WORD_SQUARE_WIDTH);
                for coli in ColIndex::all_values() {
                    let mi = MatrixIndex{row: rowi, col: coli};
                    m[mi] = row_chars[coli.into():usize].try_into().unwrap();
                }
            }
            m
        }).collect();

        expected_results.sort();

        let wordlist:Vec<EitherWord> = wordlist_str.iter().map(|&s| s.try_into().unwrap()).collect();
        let must_use:Vec<EitherWord> = must_use_str.iter().map(|&s| s.try_into().unwrap()).collect();
        let templates:Vec<WordMatrix> = make_templates(must_use.as_slice(),vec![Default::default()]);
        let results_mutex = Arc::new(Mutex::new(Vec::new()));

        let their_results_mutex = Arc::clone(&results_mutex);
        outer_compute(
            wordlist.as_slice(),
            templates.as_slice(),
            1,
            move |rx| {
                let mut results_lock = their_results_mutex.lock().unwrap();
                while let Ok(ws) = rx.recv() { results_lock.push(ws); }
                drop(results_lock);
                drop(their_results_mutex);
                Ok(())
            }
        );

        let mut lock = results_mutex.lock().unwrap();
        let mut results = Vec::new();
        std::mem::swap(&mut results, &mut lock);
        drop(lock);
        drop(results_mutex);

        results.sort();

        assert_eq!(results, expected_results);
    }

    #[cfg(all(feature = "width-5", feature = "height-5"))]
    #[test]
    fn sator_square() {
        assert_results(
            &["sator","arepo","opera","rotas","tenet"],
            &[],
            &[
                &[
                    "sator",
                    "arepo",
                    "tenet",
                    "opera",
                    "rotas",
                ]
            ],
        );
    }
}