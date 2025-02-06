use std::collections::HashSet;
use std::hash::Hash;
use std::mem;
use std::ops::{
    BitAnd,
    BitAndAssign,
    BitOr,
    BitOrAssign,
    BitXor,
    BitXorAssign,
    Not,
    Sub,
    SubAssign
};
use std::slice::Iter;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct USizeSet{
    lower: usize,
    upper: usize,
    len: usize,
    content: Vec<u64>
}

#[derive(Debug, Eq, PartialEq)]
pub enum USizeSetError{
    InvalidBounds,
    DifferentBounds,
    OutOfBounds
}

pub type USizeSetResult<V> = Result<V, USizeSetError>;

struct BitIterator{
    bit_index: usize,
    value: u64
}

impl BitIterator{
    fn new(value: u64) -> BitIterator{
        BitIterator{
            bit_index: 0,
            value
        }
    }

    fn progress(&mut self){
        let diff = self.value.trailing_zeros() as usize;
        self.value >>= diff;
        self.bit_index += diff;
    }
}

impl Iterator for BitIterator{
    type Item = usize;
    
    fn next(&mut self) -> Option<usize>{
        if self.value != 0 && (self.value & 1) == 0 {
            self.progress();
        }

        let result = if self.value == 0 { None } else { Some(self.bit_index) };
        self.value &= 0xfffffffffffffffe;
        result
    }
}

pub struct USizeSetIter<'a>{
    offset: usize,
    current: BitIterator,
    content: Iter<'a, u64>
}

impl<'a> USizeSetIter<'a> {
    fn new(set: &'a USizeSet) -> USizeSetIter<'a> {
        let mut iter = set.content.iter();
        let first_bit_iterator = if let Some(&first) = iter.next() {
            BitIterator::new(first)
        }
        else{
            BitIterator::new(0)
        };

        USizeSetIter{
            offset: set.lower,
            current: first_bit_iterator,
            content: iter
        }
    }
}

const U64_BIT_SIZE: usize = mem::size_of::<u64>() * 8;

// line 113