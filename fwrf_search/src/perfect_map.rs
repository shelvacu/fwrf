use fwrf_lib::wordstuffs::WordPrefixMap;

include!(concat!(env!("OUT_DIR"), "/perfect_map_autogen.rs"));

#[cfg(feature = "square")]
pub static THE_MAP:WordPrefixMap = WordPrefixMap::new(&PREFIX_MAP_ROW);

#[cfg(not(feature = "square"))]
pub static THE_MAP:WordPrefixMap = WordPrefixMap::new(&PREFIX_MAP_ROW, &PREFIX_MAP_COL);