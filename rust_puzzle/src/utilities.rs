use std::collections::HashSet;
use std::hash::Hash;
use std::mem;
use std::ops::{
    BitAnd, BitAndAssign, BitOr, BitOrAssign, BitXor, BitXorAssign, Not, Sub, SubAssign,
};
use std::ptr::with_exposed_provenance;
use std::slice::Iter;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct USizeSet {
    lower: usize,
    upper: usize,
    len: usize,
    content: Vec<u64>,
}

#[derive(Debug, Eq, PartialEq)]
pub enum USizeSetError {
    InvalidBounds,
    DifferentBounds,
    OutOfBounds,
}

pub type USizeSetResult<V> = Result<V, USizeSetError>;

struct BitIterator {
    bit_index: usize,
    value: u64,
}

impl BitIterator {
    fn new(value: u64) -> BitIterator {
        BitIterator {
            bit_index: 0,
            value,
        }
    }

    fn progress(&mut self) {
        let diff = self.value.trailing_zeros() as usize;
        self.value >>= diff;
        self.bit_index += diff;
    }
}

impl Iterator for BitIterator {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        if self.value != 0 && (self.value & 1) == 0 {
            self.progress();
        }

        let result = if self.value == 0 {
            None
        } else {
            Some(self.bit_index)
        };
        self.value &= 0xfffffffffffffffe;
        result
    }
}

pub struct USizeSetIter<'a> {
    offset: usize,
    current: BitIterator,
    content: Iter<'a, u64>,
}

impl<'a> USizeSetIter<'a> {
    fn new(set: &'a USizeSet) -> USizeSetIter<'a> {
        let mut iter = set.content.iter();
        let first_bit_iterator = if let Some(&first) = iter.next() {
            BitIterator::new(first)
        } else {
            BitIterator::new(0)
        };

        USizeSetIter {
            offset: set.lower,
            current: first_bit_iterator,
            content: iter,
        }
    }
}

const U64_BIT_SIZE: usize = mem::size_of::<u64>() * 8;

impl<'a> Iterator for USizeSetIter<'a> {
    type Item = usize;

    fn next(&mut self) -> Option<usize> {
        loop {
            if let Some(bit_index) = self.current.next() {
                return Some(self.offset + bit_index);
            }

            if let Some(&next_content) = self.content.next() {
                self.current = BitIterator::new(next_content);
                self.offset += U64_BIT_SIZE;
            } else {
                return None;
            }
        }
    }
}

impl USizeSet {
    pub fn new(lower: usize, upper: usize) -> USizeSetResult<USizeSet> {
        if lower > upper {
            Err(USizeSetError::InvalidBounds)
        } else {
            let required_words = (upper - lower + 64) >> 6;
            Ok(USizeSet {
                lower,
                upper,
                len: 0,
                content: vec![0u64; required_words],
            })
        }
    }

    pub fn singleton(lower: usize, upper: usize, content: usize) -> USizeSetResult<USizeSet> {
        let mut result = USizeSet::new(lower, upper)?;
        result.insert(content)?;
        Ok(result)
    }

    pub fn range(lower: usize, upper: usize) -> USizeSetResult<USizeSet> {
        if lower > upper {
            Err(USizeSetError::InvalidBounds)
        } else {
            let mut content = Vec::new();
            let ones = upper - lower + 1;
            let ones_words = ones / U64_BIT_SIZE;

            for _ in 0..ones_words {
                content.push(!0);
            }

            let remaining_ones = ones - (ones_words << 6);

            if remaining_ones > 0 {
                content.push((1 << remaining_ones) - 1);
            }

            Ok(USizeSet {
                lower,
                upper,
                len: ones,
                content,
            })
        }
    }

    fn compute_index(&self, number: usize) -> USizeSetResult<(usize, u64)> {
        if number < self.lower || number > self.upper {
            Err(USizeSetError::OutOfBounds)
        } else {
            let index = number - self.lower;
            let word_index = index >> 6;
            let sub_word_index = index & 63;
            let mask = 1u64 << sub_word_index;
            Ok((word_index, mask))
        }
    }

    pub fn lower(&self) -> usize {
        self.lower
    }

    pub fn upper(&self) -> usize {
        self.upper
    }

    pub fn min(&self) -> Option<usize> {
        for (index, &content) in self.content.iter().enumerate() {
            let trailing_zeros = content.trailing_zeros() as usize;

            if trailing_zeros < U64_BIT_SIZE {
                let offset = index * U64_BIT_SIZE + trailing_zeros;
                return Some(self.lower + offset);
            }
        }

        None
    }

    pub fn max(&self) -> Option<usize> {
        for (index, &content) in self.content.iter().enumerate().rev() {
            let leading_zeros = content.leading_zeros() as usize;

            if leading_zeros < U64_BIT_SIZE {
                let offset = (index + 1) * U64_BIT_SIZE - leading_zeros - 1;
                return Some(self.lower + offset);
            }
        }

        None
    }

    pub fn contains(&self, number: usize) -> bool {
        if let Ok((word_index, mask)) = self.compute_index(number) {
            (self.content[word_index] & mask) > 0
        } else {
            false
        }
    }

    pub fn insert(&mut self, number: usize) -> USizeSetResult<bool> {
        let (word_index, mask) = self.compute_index(number)?;
        let word = &mut self.content[word_index];

        if *word & mask == 0 {
            self.len += 1;
            *word |= mask;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn remove(&mut self, number: usize) -> USizeSetResult<bool> {
        let (word_index, mask) = self.compute_index(number)?;
        let word = &mut self.content[word_index];

        if *word & mask > 0 {
            *word &= !mask;
            self.len -= 1;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn clear(&mut self) {
        for i in 0..self.content.len() {
            self.content[i] = 0;
        }

        self.len = 0;
    }

    pub fn iter(&self) -> USizeSetIter<'_> {
        USizeSetIter::new(self)
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    fn count(&self) -> usize {
        self.content.iter().map(|c| c.count_ones() as usize).sum()
    }

    fn op_assign(
        &mut self,
        other: &USizeSet,
        op: impl Fn(u64, u64) -> u64,
    ) -> USizeSetResult<bool> {
        if self.lower() != other.lower() || self.upper() != other.upper() {
            Err(USizeSetError::DifferentBounds)
        } else {
            let contents = self.content.iter_mut().zip(other.content.iter());
            let mut changed = false;

            for (self_u64, &other_u64) in contents {
                let self_before = *self_u64;
                *self_u64 = op(self_before, other_u64);
                changed |= self_before != *self_u64;
            }

            self.len = self.count();
            Ok(changed)
        }
    }

    fn op<F>(&self, other: &USizeSet, op_assign: F) -> USizeSetResult<USizeSet>
    where
        F: Fn(&mut USizeSet, &USizeSet) -> USizeSetResult<bool>,
    {
        let mut clone = self.clone();
        op_assign(&mut clone, other)?;
        Ok(clone)
    } // LINE 415
}
