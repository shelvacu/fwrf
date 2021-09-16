#[cfg(feature = "width-2")]
pub const WORD_SQUARE_WIDTH:usize = 2;
#[cfg(feature = "width-3")]
pub const WORD_SQUARE_WIDTH:usize = 3;
#[cfg(feature = "width-4")]
pub const WORD_SQUARE_WIDTH:usize = 4;
#[cfg(feature = "width-5")]
pub const WORD_SQUARE_WIDTH:usize = 5;
#[cfg(feature = "width-6")]
pub const WORD_SQUARE_WIDTH:usize = 6;
#[cfg(feature = "width-7")]
pub const WORD_SQUARE_WIDTH:usize = 7;
#[cfg(feature = "width-8")]
pub const WORD_SQUARE_WIDTH:usize = 8;
#[cfg(feature = "width-9")]
pub const WORD_SQUARE_WIDTH:usize = 9;
#[cfg(feature = "width-10")]
pub const WORD_SQUARE_WIDTH:usize = 10;
#[cfg(feature = "width-11")]
pub const WORD_SQUARE_WIDTH:usize = 11;
#[cfg(feature = "width-12")]
pub const WORD_SQUARE_WIDTH:usize = 12;
#[cfg(feature = "width-13")]
pub const WORD_SQUARE_WIDTH:usize = 13;
#[cfg(feature = "width-14")]
pub const WORD_SQUARE_WIDTH:usize = 14;
#[cfg(feature = "width-15")]
pub const WORD_SQUARE_WIDTH:usize = 15;

#[cfg(feature = "height-2")]
pub const WORD_SQUARE_HEIGHT:usize = 2;
#[cfg(feature = "height-3")]
pub const WORD_SQUARE_HEIGHT:usize = 3;
#[cfg(feature = "height-4")]
pub const WORD_SQUARE_HEIGHT:usize = 4;
#[cfg(feature = "height-5")]
pub const WORD_SQUARE_HEIGHT:usize = 5;
#[cfg(feature = "height-6")]
pub const WORD_SQUARE_HEIGHT:usize = 6;
#[cfg(feature = "height-7")]
pub const WORD_SQUARE_HEIGHT:usize = 7;
#[cfg(feature = "height-8")]
pub const WORD_SQUARE_HEIGHT:usize = 8;
#[cfg(feature = "height-9")]
pub const WORD_SQUARE_HEIGHT:usize = 9;
#[cfg(feature = "height-10")]
pub const WORD_SQUARE_HEIGHT:usize = 10;
#[cfg(feature = "height-11")]
pub const WORD_SQUARE_HEIGHT:usize = 11;
#[cfg(feature = "height-12")]
pub const WORD_SQUARE_HEIGHT:usize = 12;
#[cfg(feature = "height-13")]
pub const WORD_SQUARE_HEIGHT:usize = 13;
#[cfg(feature = "height-14")]
pub const WORD_SQUARE_HEIGHT:usize = 14;
#[cfg(feature = "height-15")]
pub const WORD_SQUARE_HEIGHT:usize = 15;

pub const WORD_SQUARE_SIZE:usize = WORD_SQUARE_WIDTH * WORD_SQUARE_HEIGHT;

pub type CharSetInner = u32;
pub const CHAR_SET_SIZE:usize = 32;
pub const CHAR_SET_SIZE_U16:u16 = CHAR_SET_SIZE as u16;
const CHAR_SET_SIZE_MINUS_1:usize = CHAR_SET_SIZE - 1;
pub type CharSetRanged = deranged::Usize<0,CHAR_SET_SIZE_MINUS_1>;

// TODO: is this really needed?
static_assertions::const_assert!(WORD_SQUARE_SIZE < (u8::MAX as usize));

#[cfg(feature = "square")]
static_assertions::const_assert_eq!(WORD_SQUARE_HEIGHT, WORD_SQUARE_WIDTH);

#[cfg(not(feature = "square"))]
static_assertions::const_assert_ne!(WORD_SQUARE_HEIGHT, WORD_SQUARE_WIDTH);

static_assertions::const_assert_eq!(std::mem::size_of::<CharSetInner>() * 8, CHAR_SET_SIZE);