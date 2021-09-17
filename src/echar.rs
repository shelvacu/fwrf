use core::convert::TryFrom;
use core::fmt;

use crate::config::*;

#[derive(PartialEq,Eq,PartialOrd,Ord,Copy,Clone,Hash)]
pub struct EncodedChar(u8);

impl fmt::Debug for EncodedChar {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        write!(f, "E{}", (*self).into():char)
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

macro_rules! encoded_char_impls {
    ($($codepoint:literal => $char:literal,)*) => {
        impl TryFrom<char> for EncodedChar {
            type Error = &'static str;

            fn try_from(value: char) -> Result<Self, Self::Error> {
                match value {
                    $(
                        $char => Ok(Self($codepoint)),
                    )*
                    '*' => Ok(NULL_CHAR),
                    _ => Err("Invalid char for EncodedChar")
                }
            }
        }

        impl From<CharSetRanged> for EncodedChar {
            fn from(value: CharSetRanged) -> EncodedChar {
                match value.into():usize {
                    $(
                        $codepoint => Self($codepoint),
                    )*
                    _ => {
                        unsafe { ::core::hint::unreachable_unchecked() }
                    }
                }
            }
        }

        impl From<EncodedChar> for char {
            fn from(value: EncodedChar) -> char {
                match value {
                    $(
                        EncodedChar($codepoint) => $char,
                    )*
                    EncodedChar(c) if (c as usize) == CHAR_SET_SIZE + 1 => '$',
                    NULL_CHAR => '*',
                    _ => '*',
                }
            }
        }

        #[allow(dead_code)]
        const CONST_TO_CHECK_ALL_CHARS_SET:() = match 0u16 {
            $(
                $codepoint => (),
            )*
            CHAR_SET_SIZE_U16..=u16::MAX => (),
        };
    }
}

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
    26 => '0',
    27 => '1',
    28 => '2',
    29 => '3',
    30 => '4',
    31 => '5',
}