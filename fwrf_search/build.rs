use std::env;
use std::fs::File;
use std::io::{BufWriter, BufReader, Write};
use std::io::BufRead;
use std::path::Path;

use fwrf_lib::*;
use wordstuffs::*;

fn main() {
    #[cfg(feature = "perfectmap")]
    {
        // TODO: make these better configurable
        let filter_aa = true;
        let ignore_unencodeable = true;

        let path = Path::new(&env::var("OUT_DIR").unwrap()).join("perfect_map_autogen.rs");
        let mut file = BufWriter::new(File::create(&path).unwrap());

        println!("cargo:rerun-if-env-changed=WORDLIST_FN");
        println!("cargo:warning=OUT_DIR is {:?}", path);
        let filename = std::env::var_os("WORDLIST_FN").unwrap();
        let f = BufReader::new(File::open(filename).unwrap());

        let mut words:TheSet<EitherWord> = Default::default();

        let mut lineno = 1;
        for maybe_line in f.lines() {
            if maybe_line.is_err() { eprintln!("Error on line {}", lineno); }
            let line = maybe_line.unwrap();
            lineno += 1;
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

        let (_, _, prefix_maps) = fwrf_lib::make_prefix_map(Default::default(), words);

        each_unique_dimension!(dim, {
            let mut dummy_storage:Vec<String> = Vec::new();
            let mut perfect_map_builder = phf_codegen::Map::new();

            let dynamic_map = dim::prefix_map(&prefix_maps);
            for (k,v) in dynamic_map.iter() {
                dummy_storage.push(format!("::fwrf_lib::charset::CharSet({})", v.0));
                perfect_map_builder.entry(k, dummy_storage.last().unwrap());
            }

            write!(
                &mut file,
                "pub static PREFIX_MAP_{}: ::phf::Map<::fwrf_lib::wordstuffs::dim_{}::Word, ::fwrf_lib::charset::CharSet> = {};\n\n",
                dim::MAIN_AXIS_NAME_CAPS,
                dim::MAIN_AXIS_NAME_LOWER,
                perfect_map_builder.build(),
            ).unwrap();
        });
    }
}