#![feature(type_ascription, array_zip, min_specialization)]

mod config;
mod echar;
mod charset;
mod wordstuffs;

use std::io::{self, BufReader};
use std::io::prelude::*;
use std::fs::File;

use progressing::{
    Baring,
    bernoulli::Bar as BernoulliBar,
};

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
        .arg(Arg::with_name("show-progress")
            .long("show-progress")
            .short("p")
            .help("Show a progress bar on STDERR")
        )
        .arg(Arg::with_name("must-include")
            .long("must-include")
            .short("m")
            .takes_value(true)
            .help("Only search for word rectangles that include all of the given comma-separated words. These words are automatically added to the wordlist.")
        )
        .arg(Arg::with_name("fancy-output")
            .long("fancy-output")
            .short("f")
            .help("Shows output word rectangles across multiple lines (easier to see column words that way) and unbuffered. May be a significant slowdown if many results are produced.")
        )
        .arg(Arg::with_name("filter-aa")
            .long("filter-aa")
            .short("a")
            .help("Filters words of all the same letter (like 'aaaaaa')")
        )
        .get_matches()
    ;
    
    let loud = !args.is_present("quiet");
    let ignore_empty_wordlist = args.is_present("ignore-empty-wordlist");
    let ignore_unencodeable = args.is_present("ignore-unencodeable");
    let fancy = args.is_present("fancy-output");
    let show_progress = args.is_present("show-progress");
    let num_threads:u32 = args.value_of("threads").unwrap().parse().unwrap();
    let filter_aa = args.is_present("filter-aa");

    let f = BufReader::new(File::open(args.value_of("wordlist").unwrap())?);

    let mut words:TheSet<EitherWord> = Default::default();

    for maybe_line in f.lines() {
        let line = maybe_line.unwrap();
        match EitherWord::from_str_no_nulls(line.as_str()) {
            Ok(w) => {
                let s = w.as_slice();
                let mut all_same = true;
                for i in 1..s.len() {
                    all_same = all_same && s[0] == s[i];
                }
                if !filter_aa || !all_same {
                    words.insert(w);
                }
            },
            Err(WordConversionError::WrongLength) => (),
            Err(e) => {
                if !ignore_unencodeable {
                    panic!("Could not encode {:?} due to {:?}", &line, e);
                }
            }
        }
    }
    
    let must_include_strings:Vec<String> = args
        .value_of("must-include")
        .map(|s| s
            .split(',')
            .map(str::to_string)
            .collect()
        )
        .unwrap_or_default();

    // This is purposefully *not* a hashset, a word that appears twice in the must_include list must appear twice in any result word rectangles.
    let mut must_include:Vec<EitherWord> = Vec::new();

    for include_str in &must_include_strings {
        match EitherWord::from_str_with_nulls(include_str.as_str()) {
            Ok(word) => {
                must_include.push(word);
                words.insert(word);
            },
            Err(WordConversionError::WrongLength) => {
                if ignore_empty_wordlist {
                    std::process::exit(0);
                } else {
                    panic!("Must-use word {:?} length do not match dimensions.", include_str);
                }
            },
            Err(e @ WordConversionError::UnencodeableChar(_,_)) => {
                panic!("Error encoding must-use word {:?} due to {:?}", include_str, e);
            },
            Err(WordConversionError::NullChar) => unreachable!(),
        }
    }

    assert_eq!(must_include.len(), must_include_strings.len());

    if show_progress && !must_include.is_empty() {
        eprintln!("ERR: Cannot use both show-progress and must-use together.");
        std::process::exit(1);
    }

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

    if !ignore_empty_wordlist && words.is_empty() {
        panic!("No words in wordlist!");
    }

    if loud {
        eprintln!("Starting.");
    }

    let compute_func = move |w2m_rx:std::sync::mpsc::Receiver<WordMatrix>| {
        if fancy {
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
        } else {
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
        }
    };

    let mut time = devtimer::DevTime::new_simple();
    time.start();

    outer_compute(
        words,
        templates.as_slice(),
        num_threads as usize,
        compute_func,
        show_progress,
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
    wordlist: TheSet<EitherWord>,
    templates: &[WordMatrix],
    num_threads: usize,
    output_func: impl 'static + Send + FnOnce(std::sync::mpsc::Receiver<WordMatrix>) -> Result<(), std::io::Error>,
    show_progress: bool,
) {
    use std::sync::Arc;
    let wordlist_arc = Arc::new(wordlist);
    // "w2m" => worker threads to output thread
    let (w2m_tx, w2m_rx) = std::sync::mpsc::sync_channel(4);
    let output_thread = std::thread::spawn(move || output_func(w2m_rx));
    for template in templates {
        let (_row_counts, _col_counts, prefix_map) = make_prefix_map(*template, wordlist_arc.iter().copied());

        // "m2w" => main thread to worker threads
        let (m2w_tx, m2w_rx) = crossbeam_channel::bounded::<WordMatrix>(2);
        let (prog_tx, prog_rx) = crossbeam_channel::bounded::<()>(2);
        let mut worker_handles = Vec::new();

        let prefix_map_arc = Arc::new(prefix_map);

        
        for _ in 0..num_threads {
            let rxc = m2w_rx.clone();
            let txc = w2m_tx.clone();
            let progc = prog_tx.clone();
            let my_prefix_map = Arc::clone(&prefix_map_arc);
            let my_wordlist = Arc::clone(&wordlist_arc);
            worker_handles.push(
                std::thread::spawn( move || {
                    while let Ok(msg) = rxc.recv() {
                        compute(
                            &my_prefix_map,
                            msg,
                            MatrixIndex{row: RowIndex::MAX, col: ColIndex::MAX},
                            |a| {
                                each_dimension!(dim, {
                                    for i in dim::Index::all_values() {
                                        let word = dim::index_matrix(a, i);
                                        if !my_wordlist.contains(&word.into()) {
                                            return
                                        }
                                    }
                                });
                                txc.send(a).unwrap();
                            }
                        );
                        if show_progress {
                            progc.send(()).unwrap();
                        }
                    }
                })
            );
        }

        let a = &*prefix_map_arc;
        let mut mi = MatrixIndex::ZERO;
        {
            let mut nulls_so_far = 0;
            while nulls_so_far < config::WORD_SQUARE_WIDTH-1 {
                if template[mi] == NULL_CHAR { nulls_so_far += 1 }
                mi = match mi.inc() {
                    Some(v) => v,
                    None => break,
                }
            }
        }
        if DEBUG { dbg!(mi); }

        let mut count = 0;
        let progress_bar_thread = if show_progress {
            compute(a, *template, mi, |_| count += 1);
            let mut progress_bar = BernoulliBar::with_goal(count).timed();
            eprintln!("{}", progress_bar);
            let mut last_progress_display = std::time::Instant::now();
            Some(std::thread::spawn(move || {
                while let Ok(_) = prog_rx.recv() {
                    progress_bar.add(true);
                    if last_progress_display.elapsed().as_secs() >= 1 {
                        last_progress_display = std::time::Instant::now();
                        eprintln!("{}", progress_bar);
                    }
                }
            }))
        } else { None };

        let f = |ca| {
            if DEBUG { dbg!(ca); }
            m2w_tx.send(ca).unwrap();
            if show_progress {
            }
        };
        if DEBUG { dbg!(); }
        compute(
            a,
            *template,
            mi,
            f,
        );
        if DEBUG { dbg!(); }

        drop(m2w_tx);
        drop(prog_tx);
        for h in worker_handles {
            h.join().unwrap();
        }
        if let Some(t) = progress_bar_thread { t.join().unwrap() }
        if DEBUG { dbg!(); }
    }
    if DEBUG { dbg!(); }
    drop(w2m_tx);
    output_thread.join().unwrap().unwrap();
    if DEBUG { dbg!(); }
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
        // if DEBUG {
        //     dbg!(at_idx,matrix);
        // }
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
                if DEBUG { dbg!(); }
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
        if DEBUG { dbg!(); }

        expected_results.sort();

        let wordlist:TheSet<EitherWord> = wordlist_str.iter().map(|&s| EitherWord::from_str_no_nulls(s).unwrap()).collect();
        let must_use:Vec<EitherWord> = must_use_str.iter().map(|&s| EitherWord::from_str_with_nulls(s).unwrap()).collect();
        let templates:Vec<WordMatrix> = make_templates(must_use.as_slice(),vec![Default::default()]);
        let results_mutex = Arc::new(Mutex::new(Vec::new()));
        if DEBUG { dbg!(); }

        let their_results_mutex = Arc::clone(&results_mutex);
        outer_compute(
            wordlist,
            templates.as_slice(),
            1,
            move |rx| {
                let mut results_lock = their_results_mutex.lock().unwrap();
                if DEBUG { dbg!(); }
                while let Ok(ws) = rx.recv() { results_lock.push(ws); }
                drop(results_lock);
                drop(their_results_mutex);
                if DEBUG { dbg!(); }
                Ok(())
            },
            false,
        );
        if DEBUG { dbg!(); }

        let mut lock = results_mutex.lock().unwrap();
        let mut results = Vec::new();
        if DEBUG { dbg!(); }
        std::mem::swap(&mut results, &mut lock);
        drop(lock);
        drop(results_mutex);

        results.sort();
        if DEBUG { dbg!(); }

        assert_eq!(results, expected_results);
    }

    #[cfg(all(feature = "width-5", feature = "height-5"))]
    #[test]
    fn sator_square() {
        if DEBUG { dbg!(); }
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
                ],
                &[
                    "rotas",
                    "opera",
                    "tenet",
                    "arepo",
                    "sator",
                ],
            ],
        );
    }

    #[cfg(all(feature = "width-5", feature = "height-5"))]
    #[test]
    fn aaaaa() {
        assert_results(
            &["aaaaa"], 
            &[],
            &[
                &[
                    "aaaaa",
                    "aaaaa",
                    "aaaaa",
                    "aaaaa",
                    "aaaaa",
                ]
            ]
        );
    }

    #[cfg(all(feature = "width-6", feature = "height-4"))]
    #[test]
    fn fwrf() {
        assert_results(
            &[
                "fresco",
                "worker",
                "raging",
                "frosty",
                "fwrf",
                "roar",
                "ergo",
                "skis",
                "cent",
                "orgy",
            ],
            &[],
            &[
                &[
                    "fresco",
                    "worker",
                    "raging",
                    "frosty",
                ]
            ]
        );
    }

    #[cfg(all(feature = "width-4", feature = "height-2"))]
    #[test]
    /// With no length 2 words available, this should never produce a result. This is a potential edge case because the templates will be completely filled.
    fn must_use_fills_1() {
        assert_results(
            &["test", "word"],
            &["test", "word"],
            &[],
        )
    }

    #[cfg(all(feature = "width-4", feature = "height-2"))]
    #[test]
    /// With some but not all length 2 words available, this should never produce a result. This is a potential edge case because the templates will be completely filled.
    fn must_use_fills_2() {
        assert_results(
            &["test", "word", "tw", "sr", "td"],
            &["test", "word"],
            &[],
        )
    }

    #[cfg(all(feature = "width-4", feature = "height-2"))]
    #[test]
    /// With length 2 words available, this should produce 1 result (test|word but not word|test). This is a potential edge case because the templates will be completely filled.
    /// 
    /// ```
    /// TEST
    /// WORD
    /// ```
    fn must_use_fills_3() {
        assert_results(
            &["test", "word", "tw", "eo", "sr", "td"],
            &["test", "word"],
            &[
                &[
                    "test",
                    "word",
                ],
            ],
        )
    }
}