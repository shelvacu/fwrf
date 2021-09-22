use crate::config::*;
use crate::echar::EncodedChar;

#[derive(Debug,Clone,Copy,Eq,PartialEq,Default)]
pub struct CharSet(CharSetInner);

impl CharSet {
    pub fn set(&mut self, e: EncodedChar) {
        let inner = e.inner();
        if inner >= CHAR_SET_SIZE { panic!("invalid echar to set on charset {:?}", e) }
        self.0 |= 1 << inner;
    }

    pub fn has(&self, e: EncodedChar) -> bool {
        let inner = e.inner();
        if inner >= CHAR_SET_SIZE { panic!("invalid echar to get on charset {:?}", e) }
        (self.0 & (1 << inner)) > 0
    }

    // pub fn iter(self) -> impl Iterator<Item = EncodedChar> {
    //     CharSetRanged::all_values().filter_map(move |c| {
    //         let e:EncodedChar = c.into();
    //         if self.has(e) {
    //             Some(e)
    //         } else { None }
    //     })
    // }

    #[must_use]
    pub fn and(self, other: CharSet) -> CharSet {
        CharSet(self.0 & other.0)
    }
}