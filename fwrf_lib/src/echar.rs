use core::convert::TryFrom;
use core::fmt;

use crate::config::*;

#[derive(PartialEq,Eq,PartialOrd,Ord,Copy,Clone,Hash)]
pub struct EncodedChar(pub u8);


#[cfg(feature = "perfectmap")]
impl phf::PhfHash for EncodedChar {
    #[inline]
    fn phf_hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.0.phf_hash(state)
    }
}

#[cfg(feature = "perfectmap")]
impl phf_shared::FmtConst for EncodedChar {
    fn fmt_const(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "::fwrf_lib::echar::EncodedChar({})", self.0)
    }
}

impl fmt::Debug for EncodedChar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        if *self == NULL_CHAR || (self.0 as usize) < CHAR_SET_SIZE {
            let as_char:char = (*self).into();
            write!(f, "E{}", as_char)
        } else {
            write!(f, "E{}", self.0)
        }
    }
}

impl Default for EncodedChar {
    fn default() -> Self {
        NULL_CHAR
    }
}

impl EncodedChar {
    pub fn inner(self) -> usize {
        self.0.into()
    }

    // Two EncodedChars "match" if either is NULL_CHAR or they are equal. 
    pub fn is_match(self, other: Self) -> bool {
        self == NULL_CHAR || other == NULL_CHAR || self == other
    }

    #[must_use]
    pub fn inc(self) -> Option<Self> {
        if self == NULL_CHAR {
            Some(Self(0))
        } else if self.0 < (CHAR_SET_SIZE - 1) as u8 {
            Some(Self(self.0 + 1))
        } else {
            None
        }
    }
}

pub const NULL_CHAR:EncodedChar = EncodedChar(u8::MAX);

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub struct UnencodeableChar(pub char);

impl fmt::Display for UnencodeableChar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UnencodeableChar({:?})", self.0)
    }
}

impl std::error::Error for UnencodeableChar {}

macro_rules! encoded_char_impls {
    ($($codepoint:literal => $char:literal,)*) => {
        impl TryFrom<char> for EncodedChar {
            type Error = UnencodeableChar;

            #[forbid(unreachable_patterns)]
            fn try_from(mut value: char) -> Result<Self, Self::Error> {
                value.make_ascii_lowercase();
                match value {
                    $(
                        $char => Ok(Self($codepoint)),
                    )*
                    '&' => Ok(NULL_CHAR),
                    _ => Err(UnencodeableChar(value))
                }
            }
        }

        impl From<CharSetRanged> for EncodedChar {
            #[forbid(unreachable_patterns)]
            fn from(value: CharSetRanged) -> EncodedChar {
                let as_usize:usize = value.into();
                match as_usize {
                    $(
                        $codepoint => Self($codepoint),
                    )*
                    _ => {
                        #[cfg(feature = "unchecked")]
                        unsafe { ::core::hint::unreachable_unchecked() }
                        #[cfg(not(feature = "unchecked"))]
                        unreachable!()
                    }
                }
            }
        }

        impl From<EncodedChar> for char {
            #[forbid(unreachable_patterns)]
            fn from(value: EncodedChar) -> char {
                match value {
                    $(
                        EncodedChar($codepoint) => $char,
                    )*
                    EncodedChar(c) if (c as usize) == CHAR_SET_SIZE + 1 => '$',
                    NULL_CHAR => '&',
                    _ => '?',
                }
            }
        }

        #[allow(dead_code)]
        #[forbid(unreachable_patterns)]
        const CONST_TO_CHECK_ALL_CHARS_SET:() = match 0u16 {
            $(
                $codepoint => (),
            )*
            CHAR_SET_SIZE_U16..=u16::MAX => (),
        };
    }
}

// The top 32 characters (after downcasing), excluding numerals.
#[cfg(feature = "charset-english-small")]
encoded_char_impls! {
    0  => 'a',
    1  => 'b',
    2  => 'c',
    3  => 'd',
    4  => 'e',
    5  => 'f',
    6  => 'g',
    7  => 'h',
    8  => 'i',
    9  => 'j',
    10 => 'k',
    11 => 'l',
    12 => 'm',
    13 => 'n',
    14 => 'o',
    15 => 'p',
    16 => 'q',
    17 => 'r',
    18 => 's',
    19 => 't',
    20 => 'u',
    21 => 'v',
    22 => 'w',
    23 => 'x',
    24 => 'y',
    25 => 'z',
    26 => '.',
    27 => '-',
    28 => ',',
    29 => 'é',
    30 => '\'',
    31 => '/',
}

// The top 64 most used characters (after downcasing) in the google ngrams corpus
#[cfg(feature = "charset-english-extended")]
encoded_char_impls! {
    0  => 'e',
    1  => 'i',
    2  => 'a',
    3  => 'n',
    4  => 's',
    5  => 'r',
    6  => 't',
    7  => 'o',
    8  => 'l',
    9  => 'c',
    10 => 'd',
    11 => 'u',
    12 => 'g',
    13 => 'm',
    14 => 'p',
    15 => 'h',
    16 => '.',
    17 => 'b',
    18 => 'y',
    19 => 'f',
    20 => 'v',
    21 => 'w',
    22 => 'k',
    23 => '1',
    24 => 'z',
    25 => 'x',
    26 => '0',
    27 => '-',
    28 => '2',
    29 => 'q',
    30 => 'j',
    31 => '3',
    32 => '4',
    33 => '9',
    34 => '5',
    35 => '7',
    36 => '8',
    37 => '6',
    38 => ',',
    39 => 'é',
    40 => '\'',
    41 => '/',
    42 => ':',
    43 => 'è',
    44 => 'á',
    45 => 'ü',
    46 => 'ó',
    47 => 'ö',
    48 => 'í',
    49 => 'ç',
    50 => 'ä',
    51 => 'ñ',
    52 => '*',
    53 => '@',
    54 => 'ú',
    55 => 'ø',
    56 => 'à',
    57 => 'æ',
    58 => 'â',
    59 => 'î',
    60 => 'œ',
    61 => 'ï',
    62 => 'ã',
    63 => 'ô',
}