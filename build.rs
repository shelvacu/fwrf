use std::env;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::Path;

fn main() {
    #[cfg(feature = "perfect")]
    {
        let path = Path::new(&env::var("OUT_DIR").unwrap()).join("codegen.rs");
        let mut file = BufWriter::new(File::create(&path).unwrap());

        let wordlist:String = std::fs::read_to_string(std::env::var_os("WORDLIST_FN").unwrap()).unwrap();

        let prefix_map:HashMap<[char; config::WORD_SQUARE_WIDTH], 

        let mut map = phf_codegen::Map::new();
        let mut n:u64 = 27941;
        for line in wordlist.lines() {
            map.entry(line, format!("{n}").as_str());
            n += 1;
        }

        write!(
            &mut file,
            "static KEYWORDS: phf::Map<&'static str, u64> = {}",
            map.build()
        )
        .unwrap();
        write!(&mut file, ";\n").unwrap();
    }
}