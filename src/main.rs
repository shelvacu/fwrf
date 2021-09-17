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
                    Err(e) => Err(String::from(format!("Must provide a valid integer. {:?}", e))),
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
            .help("Don't complain if there are no words of the necessary length in the given wordlist")
        )
        .arg(Arg::with_name("ignore-unencodeable")
            .long("ignore-unencodeable")
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
            .default_value("")
            .help("Only search for word rectangles that include all of the given comma-separated words.")
        )
        .get_matches()
    ;
    
    let loud = !args.is_present("quiet");
    let ignore_empty_wordlist = args.is_present("ignore-empty-wordlist");
    let ignore_unencodeable = args.is_present("ignore-unencodeable");
    let num_threads:u32 = args.value_of("threads").unwrap().parse().unwrap();
    
    let must_include_strings:Vec<String> = args.value_of("must-include").unwrap().split(",").map(str::to_string).collect();

    let mut templates:Vec<WordMatrix> = vec![Default::default()];

    for include_str in &must_include_strings {
        let mut success = false;
        let mut old_templates = vec![];
        std::mem::swap(&mut templates, &mut old_templates);
        each_dimension!(dim, {
            if let Ok(word) = include_str.as_str().try_into():Result<dim::Word,_> {
                success = true;
                for template in &old_templates {
                    for i in dim::Index::all_values() {
                        if dim::index_matrix(*template, i).is_match(word) {
                            let mut new_template = *template;
                            dim::set_matrix(&mut new_template, i, word);
                            templates.push(new_template);
                        }
                    }
                }
            }
        });

        if !success {
            if ignore_empty_wordlist {
                std::process::exit(0);
            } else {
                panic!("Must-use-word lengths do not match dimensions.");
            }
        }
    }

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

    let words = f.lines().filter(|r| {
        if let Ok(s) = r {
            s.len() == config::WORD_SQUARE_WIDTH || s.len() == config::WORD_SQUARE_HEIGHT
        } else { true }
    }).collect::<Result<Vec<_>,_>>()?;

    if !ignore_empty_wordlist && words.is_empty() {
        panic!("No words in wordlist!");
    }

    for template in &templates {
        let (row_counts, col_counts, prefix_map) = make_prefix_map(*template, words.iter(), ignore_unencodeable);
        if loud {
            eprintln!("Finished creating index, {} row words X {} col words", row_counts, col_counts);
        }

        // "m2w" => main thread to worker threads
        // "w2m" => worker threads to main thread
        let (m2w_tx, m2w_rx) = crossbeam_channel::bounded::<WordMatrix>(128);
        let (w2m_tx, w2m_rx) = std::sync::mpsc::sync_channel(128);
        let mut worker_handles = Vec::new();

        if loud {
            eprintln!("Creating {} worker threads.", num_threads);
        }

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

        drop(w2m_tx);

        let printing_thread = std::thread::spawn(move || {
            while let Ok(msg) = w2m_rx.recv() {
                print_word_matrix(msg);
            }
        });

        if loud {
            eprintln!("Starting.");
        }

        let mut time = devtimer::DevTime::new_simple();
        time.start();
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
        printing_thread.join().unwrap();
        time.stop();
        if loud {
            eprintln!("Took {} secs", (time.time_in_micros().unwrap() as u64 as f64) / 1_000_000.0)
        }
    }

    Ok(())
}

fn print_word_matrix(wm: WordMatrix) {
    for row in RowIndex::all_values() {
        for col in ColIndex::all_values() {
            print!("{}", wm[MatrixIndex{row,col}].into():char);
        }
        if row < RowIndex::MAX { print!("-"); }
    }
    print!("\n");
}

// It is assumed that this function does *not* need to be fast, and should be written in whatever way is reasonably fast and most correct and elegant.
fn make_prefix_map<I>
(
    template: WordMatrix,
    wordlist: I,
    ignore_unencodeable: bool,
) -> (usize, usize, WordPrefixMap)
where
    I: IntoIterator,
    I::Item: AsRef<str>, 
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
    'each_word: for w in wordlist {
        let s:&str = w.as_ref();
        let chars = s.chars().map(|c| c.to_ascii_lowercase()).collect():Vec<_>;
        each_unique_dimension!(dim, {
            if chars.len() == dim::size() {
                let mut w:dim::Word = Default::default();
                for i in dim::cross::Index::all_values() {
                    match chars[i.into():usize].try_into() {
                        Err(e) => {
                            if !ignore_unencodeable {
                                eprintln!("Could not encode {:?}: {:?}", &chars, e);
                            }
                            continue 'each_word;
                        }
                        Ok(NULL_CHAR) => {
                            if !ignore_unencodeable {
                                eprintln!("Found null indicator");
                            }
                            continue 'each_word;
                        }
                        Ok(v) => w[i] = v,
                    }
                }
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
        if is_nullish[at_idx] {
            if orig_matrix[at_idx] == NULL_CHAR {
                let (row_set, col_set) = each_dimension!(dim, {
                    dim::prefix_map(prefix_map).get(&dim::get_word_intersecting_point(matrix, at_idx)).map(|c| *c).unwrap_or_default()
                });
                charset_array[at_idx] = row_set.and(col_set);
            }
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