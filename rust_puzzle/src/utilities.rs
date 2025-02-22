use std::collections::HashSet;
use std::hash::Hash;
use std::{iter, mem};
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
    }

    pub fn union_assign(&mut self, other: &USizeSet) -> USizeSetResult<bool> {
        self.op_assign(other, u64::bitor)
    }

    pub fn union(&self, other: &USizeSet) -> USizeSetResult<USizeSet> {
        self.op(other, USizeSet::union_assign)
    }

    pub fn intersect_assign(&mut self, other: &USizeSet) -> USizeSetResult<bool> {
        self.op_assign(other, u64::bitand)
    }

    pub fn intersect(&self, other: &USizeSet) -> USizeSetResult<USizeSet> {
        self.op(other, USizeSet::intersect_assign)
    }

    pub fn difference_assign(&mut self, other: &USizeSet) -> USizeSetResult<bool> {
        self.op_assign(other, |a, b| a & !b)
    }

    pub fn difference(&self, other: &USizeSet) -> USizeSetResult<USizeSet> {
        self.op(other, USizeSet::difference_assign)
    }

    pub fn symmetric_difference_assign(&mut self, other: &USizeSet) -> USizeSetResult<bool> {
        self.op_assign(other, u64::bitxor)
    }

    pub fn symmetric_difference(&self, other: &USizeSet) -> USizeSetResult<USizeSet> {
        self.op(other, USizeSet::symmetric_difference_assign)
    }

    pub fn complement_assign(&mut self) {
        let len = self.content.len();

        for i in 0..(len - 1) {
            self.content[i] = !self.content[i];
        }

        let rem_bits = (self.upper() - self.lower() + 1) % U64_BIT_SIZE;

        if rem_bits > 0 {
            let mask = u64::MAX >> (U64_BIT_SIZE - rem_bits);
            self.content[len - 1] ^= mask;
        }

        self.len = self.count();
    }

    pub fn complement(&self) -> USizeSet {
        let mut result = self.clone();
        result.complement_assign();
        result
    }

    fn rel<F>(&self, other: &USizeSet, u64_rel: F) -> USizeSetResult<bool>
    where
        F: Fn(u64, u64) -> bool,
    {
        if self.lower != other.lower || self.upper != other.upper {
            Err(USizeSetError::DifferentBounds)
        } else {
            let contents = self.content.iter().zip(other.content.iter());

            for (&self_u64, &other_u64) in contents {
                if !u64_rel(self_u64, other_u64) {
                    return Ok(false);
                }
            }

            Ok(true)
        }
    }

    pub fn is_disjoint(&self, other: &USizeSet) -> USizeSetResult<bool> {
        self.rel(other, |s, o| s & o == 0)
    }

    pub fn is_subset(&self, other: &USizeSet) -> USizeSetResult<bool> {
        self.rel(other, |s, o| s & o == s)
    }

    pub fn is_proper_subset(&self, other: &USizeSet) -> USizeSetResult<bool> {
        Ok(self.is_subset(other)? && self.len < other.len)
    }

    pub fn is_superset(&self, other: &USizeSet) -> USizeSetResult<bool> {
        other.is_subset(self)
    }

    pub fn is_proper_superset(&self, other: &USizeSet) -> USizeSetResult<bool> {
        other.is_proper_subset(self)
    }
}

#[macro_export]
macro_rules! set {
    ($set:expr; $e:expr) => {
        ($set).insert($e).unwrap()
    };

    ($set:expr; $e:expr, $($es:expr),+) => {
        set!($set; $e);
        set!($set; $($es),+)
    };

    ($lower:expr, $upper:expr; $($es:expr),+) => {
        {
            let mut set = USizeSet::new($lower, $upper).unwrap();
            set!(set; $($es),+);
            set
        }
    };
}

impl BitAnd<&USizeSet> for USizeSet {
    type Output = USizeSet;

    fn bitand(mut self, rhs: &USizeSet) -> USizeSet {
        self.intersect_assign(rhs).unwrap();
        self
    }
}

impl BitOr<&USizeSet> for USizeSet {
    type Output = USizeSet;

    fn bitor(mut self, rhs: &USizeSet) -> USizeSet {
        self.union_assign(rhs).unwrap();
        self
    }
}

impl Sub<&USizeSet> for USizeSet {
    type Output = USizeSet;

    fn sub(mut self, rhs: &USizeSet) -> USizeSet {
        self.difference_assign(rhs).unwrap();
        self
    }
}

impl BitXor<&USizeSet> for USizeSet {
    type Output = USizeSet;

    fn bitxor(mut self, rhs: &USizeSet) -> USizeSet {
        self.symmetric_difference_assign(rhs).unwrap();
        self
    }
}

impl BitAnd for &USizeSet {
    type Output = USizeSet;

    fn bitand(self, rhs: Self) -> USizeSet {
        self.intersect(rhs).unwrap()
    }
}

impl BitOr for &USizeSet {
    type Output = USizeSet;

    fn bitor(self, rhs: Self) -> USizeSet {
        self.union(rhs).unwrap()
    }
}

impl Sub for &USizeSet {
    type Output = USizeSet;

    fn sub(self, rhs: Self) -> USizeSet {
        self.difference(rhs).unwrap()
    }
}

impl BitXor for &USizeSet {
    type Output = USizeSet;

    fn bitxor(self, rhs: Self) -> USizeSet {
        self.symmetric_difference(rhs).unwrap()
    }
}

impl BitAndAssign<&USizeSet> for USizeSet {
    fn bitand_assign(&mut self, rhs: &USizeSet) {
        self.intersect_assign(rhs).unwrap();
    }
}

impl BitOrAssign<&USizeSet> for USizeSet {
    fn bitor_assign(&mut self, rhs: &USizeSet) {
        self.union_assign(rhs).unwrap();
    }
}

impl SubAssign<&USizeSet> for USizeSet {
    fn sub_assign(&mut self, rhs: &USizeSet) {
        self.difference_assign(rhs).unwrap();
    }
}

impl BitXorAssign<&USizeSet> for USizeSet {
    fn bitxor_assign(&mut self, rhs: &USizeSet) {
        self.symmetric_difference_assign(rhs).unwrap();
    }
}

impl BitAndAssign<&USizeSet> for &mut USizeSet {
    fn bitand_assign(&mut self, rhs: &USizeSet) {
        self.intersect_assign(rhs).unwrap();
    }
}

impl BitOrAssign<&USizeSet> for &mut USizeSet {
    fn bitor_assign(&mut self, rhs: &USizeSet) {
        self.union_assign(rhs).unwrap();
    }
}

impl SubAssign<&USizeSet> for &mut USizeSet {
    fn sub_assign(&mut self, rhs: &USizeSet) {
        self.difference_assign(rhs).unwrap();
    }
}

impl BitXorAssign<&USizeSet> for &mut USizeSet {
    fn bitxor_assign(&mut self, rhs: &USizeSet) {
        self.symmetric_difference_assign(rhs).unwrap();
    }
}

impl Not for &USizeSet {
    type Output = USizeSet;

    fn not(self) -> Self::Output {
        self.complement()
    }
}

impl Not for USizeSet {
    type Output = USizeSet;

    fn not(mut self) -> USizeSet {
        self.complement_assign();
        self
    }
}

pub(crate) fn contains_duplicate<I>(mut iter: I) -> bool 
where 
    I: Iterator, 
    I::Item: Hash + Eq 
    {
    let mut set = HashSet::new();
    iter.any(|e| !set.insert(e))
}

pub(crate) fn abs_diff(a: usize, b: usize) -> usize {
    if a < b {
        b - a
    }
    else {
        a - b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_set_is_empty() {
        let set = USizeSet::new(1, 9).unwrap();
        assert!(set.is_empty());
        assert!(!set.contains(1));
        assert!(!set.contains(3));
        assert!(!set.contains(9));
        assert_eq!(0, set.len());
    }

    #[test]
    fn range_set_is_full() {
        let set = USizeSet::range(1, 9).unwrap();
        assert!(!set.is_empty());
        assert!(set.contains(1));
        assert!(set.contains(3));
        assert!(set.contains(9));
        assert_eq!(9, set.len());
    }

    #[test]
    fn multi_word_range() {
        let set = USizeSet::range(100, 199).unwrap();
        assert!(set.contains(100));
        assert!(set.contains(199));
        assert!(!set.contains(99));
        assert!(!set.contains(200));
        assert_eq!(100, set.len());
    }

    #[test]
    fn singleton_set_contains_only_given_element() {
        let set = USizeSet::singleton(1, 9, 3).unwrap();
        assert!(!set.is_empty());
        assert!(!set.contains(1));
        assert!(set.contains(3));
        assert!(!set.contains(9));
        assert_eq!(1, set.len());
    }

    #[test]
    fn set_macro_has_specified_range() {
        let set = set!(2, 5; 3);
        assert_eq!(2, set.lower());
        assert_eq!(5, set.upper());
    }

    #[test]
    fn set_macro_contains_specified_elements() {
        let set = set!(2, 8; 3, 7, 8);
        assert_eq!(3, set.len());
        assert!(set.contains(3));
        assert!(set.contains(7));
        assert!(set.contains(8));
        assert!(!set.contains(5));
    }

    #[test]
    fn set_creation_error() {
        assert_eq!(Err(USizeSetError::InvalidBounds), USizeSet::new(1, 0));
        assert_eq!(Err(USizeSetError::InvalidBounds), USizeSet::new(5, 3));
    }

    #[test]
    fn set_insertion_error() {
        let mut set = USizeSet::new(1, 5).unwrap();
        assert_eq!(Err(USizeSetError::OutOfBounds), set.insert(0));
        assert_eq!(Err(USizeSetError::OutOfBounds), set.insert(6));
    }

    #[test]
    fn set_operation_error() {
        let set_1 = USizeSet::new(1, 9).unwrap();
        let set_2 = USizeSet::new(1, 6).unwrap();
        assert_eq!(Err(USizeSetError::DifferentBounds), set_1.union(&set_2));
        assert_eq!(Err(USizeSetError::DifferentBounds), set_2.intersect(&set_1));
    }

    #[test]
    fn manipulation() {
        let mut set = USizeSet::new(1, 9).unwrap();
        set.insert(2).unwrap();
        set.insert(4).unwrap();
        set.insert(6).unwrap();

        assert!(!set.is_empty());
        assert!(set.contains(2));
        assert!(set.contains(4));
        assert!(set.contains(6));
        assert_eq!(3, set.len());

        set.remove(4).unwrap();

        assert!(!set.is_empty());
        assert!(set.contains(2));
        assert!(!set.contains(4));
        assert!(set.contains(6));
        assert_eq!(2, set.len());

        set.clear();

        assert!(set.is_empty());
        assert!(!set.contains(2));
        assert!(!set.contains(4));
        assert!(!set.contains(6));
        assert_eq!(0, set.len());
    }

    #[test]
    fn iteration() {
        let mut set = USizeSet::new(1, 100).unwrap();
        set.insert(1).unwrap();
        set.insert(12).unwrap();
        set.insert(23).unwrap();
        set.insert(36).unwrap();
        set.insert(42).unwrap();
        set.insert(64).unwrap();
        set.insert(65).unwrap();
        set.insert(97).unwrap();
        set.insert(100).unwrap();

        let mut iter = set.iter();

        assert_eq!(Some(1), iter.next());
        assert_eq!(Some(12), iter.next());
        assert_eq!(Some(23), iter.next());
        assert_eq!(Some(36), iter.next());
        assert_eq!(Some(42), iter.next());
        assert_eq!(Some(64), iter.next());
        assert_eq!(Some(65), iter.next());
        assert_eq!(Some(97), iter.next());
        assert_eq!(Some(100), iter.next());
        assert_eq!(None, iter.next());
    }

    #[test]
    fn double_insert() {
        let mut set = USizeSet::new(1, 9).unwrap();
        assert!(set.insert(3).unwrap());
        assert!(set.insert(4).unwrap());
        assert!(!set.insert(3).unwrap());

        assert!(set.contains(3));
        assert_eq!(2, set.len());
    }

    #[test]
    fn double_remove() {
        let mut set = USizeSet::range(1, 9).unwrap();
        assert!(set.remove(3).unwrap());
        assert!(set.remove(5).unwrap());
        assert!(!set.remove(3).unwrap());

        assert!(!set.contains(3));
        assert_eq!(7, set.len());
    }

    fn op_test_lhs() -> USizeSet {
        set!(1, 4; 2, 4)
    }

    fn op_test_rhs() -> USizeSet {
        set!(1, 4; 3, 4)
    }

    fn triangle_nums_to_100() -> USizeSet {
        set!(1, 100; 1, 3, 6, 10, 15, 21, 28, 36, 45, 55, 66, 78, 91)
    }

    fn fibs_to_100() -> USizeSet {
        set!(1, 100; 1, 2, 3, 5, 8, 13, 21, 34, 55, 89)
    }

    #[test]
    fn union() {
        let result = op_test_lhs() | &op_test_rhs();
        let expected = set!(1, 4; 2, 3, 4);
        assert_eq!(expected, result);
        assert_eq!(3, result.len());
    }

    #[test]
    fn multi_word_union() {
        let result = triangle_nums_to_100() | &fibs_to_100();
        let expected = set!(1, 100;
            1, 2, 3, 5, 6, 8, 10, 13, 15, 21,
            28, 34, 36, 45, 55, 66, 78, 89, 91);
        assert_eq!(expected, result);
        assert_eq!(19, result.len());
    }

    #[test]
    fn intersection() {
        let result = op_test_lhs() & &op_test_rhs();
        let expected = set!(1, 4; 4);
        assert_eq!(expected, result);
        assert_eq!(1, result.len());
    }

    #[test]
    fn multi_word_intersection() {
        let result = triangle_nums_to_100() & &fibs_to_100();
        let expected = set!(1, 100; 1, 3, 21, 55);
        assert_eq!(expected, result);
        assert_eq!(4, result.len())
    }

    #[test]
    fn difference() {
        let result = op_test_lhs() - &op_test_rhs();
        let expected = set!(1, 4; 2);
        assert_eq!(expected, result);
        assert_eq!(1, result.len());
    }

    #[test]
    fn multi_word_difference() {
        let result = triangle_nums_to_100() - &fibs_to_100();
        let expected = set!(1, 100; 6, 10, 15, 28, 36, 45, 66, 78, 91);
        assert_eq!(expected, result);
        assert_eq!(9, result.len());
    }

    #[test]
    fn symmetric_difference() {
        let result = op_test_lhs() ^ &op_test_rhs();
        let expected = set!(1, 4; 2, 3);
        assert_eq!(expected, result);
        assert_eq!(2, result.len());
    }

    #[test]
    fn multi_word_symmetric_difference() {
        let result = triangle_nums_to_100() ^ &fibs_to_100();
        let expected =
            set!(1, 100;
                2, 5, 6, 8, 10, 13, 15, 28, 34, 36, 45, 66, 78, 89, 91);
        assert_eq!(expected, result);
        assert_eq!(15, result.len());
    }

    #[test]
    fn complement() {
        let result = !op_test_lhs();
        let expected = set!(1, 4; 1, 3);
        assert_eq!(expected, result);
        assert_eq!(2, result.len());
    }

    #[test]
    fn multi_word_complement() {
        let result = !triangle_nums_to_100();
        let mut expected = USizeSet::range(1, 100).unwrap();

        for i in 1..=13 {
            expected.remove(i * (i + 1) / 2).unwrap();
        }

        assert_eq!(expected, result);
        assert_eq!(87, result.len());
    }

    #[test]
    fn complement_full() {
        let result = !USizeSet::range(5, 105).unwrap();
        let expected = USizeSet::new(5, 105).unwrap();
        assert_eq!(expected, result);
        assert_eq!(0, result.len());
    }

    #[test]
    fn complement_empty() {
        let result = !USizeSet::new(5, 105).unwrap();
        let expected = USizeSet::range(5, 105).unwrap();
        assert_eq!(expected, result);
        assert_eq!(101, result.len());
    }

    #[test]
    fn contains_duplicate_false() {
        let vec = vec![1, 5, 2, 4, 3];
        assert!(!contains_duplicate(vec.iter()));
        assert!(!contains_duplicate(vec.iter().map(|i| i.to_string())));
    }

    #[test]
    fn contains_duplicate_true() {
        let vec = vec![1, 5, 2, 4, 5];
        assert!(contains_duplicate(vec.iter()));
        assert!(contains_duplicate(vec.iter().map(|i| i.to_string())));
    }

    #[test]
    fn min_empty() {
        assert_eq!(None, USizeSet::new(512, 1024).unwrap().min());
    }

    #[test]
    fn min_filled() {
        assert_eq!(Some(2), set!(1, 9; 2, 5).min());
        assert_eq!(Some(100), set!(1, 200; 100, 105, 195).min());
    }

    #[test]
    fn max_empty() {
        assert_eq!(None, USizeSet::new(512, 1024).unwrap().max());
    }

    #[test]
    fn max_filled() {
        assert_eq!(Some(5), set!(1, 9; 2, 5).max());
        assert_eq!(Some(100), set!(1, 200; 5, 95, 100).max());
    }

    #[test]
    fn disjoint_relations() {
        let primes = set!(1, 10; 2, 3, 5, 7);
        let squares = set!(1, 10; 1, 4, 9);
        let even = set!(1, 10; 2, 4, 6, 8, 10);

        assert!(primes.is_disjoint(&squares).unwrap());
        assert!(!primes.is_disjoint(&even).unwrap());
    }

    #[test]
    fn multi_word_disjoint_relations() {
        let fibs = fibs_to_100();
        let squares = set!(1, 100; 1, 4, 9, 16, 25, 36, 49, 64, 81, 100);
        let big_squares = set!(1, 100; 4, 9, 16, 25, 36, 49, 64, 81, 100);
        let singleton_89 = USizeSet::singleton(1, 100, 89).unwrap();

        assert!(!fibs.is_disjoint(&squares).unwrap());
        assert!(fibs.is_disjoint(&big_squares).unwrap());
        assert!(!fibs.is_disjoint(&singleton_89).unwrap());
    }

    fn assert_subset(a: &USizeSet, b: &USizeSet) {
        assert!(a.is_subset(b).unwrap());
        assert!(b.is_superset(a).unwrap());
    }

    fn assert_not_subset(a: &USizeSet, b: &USizeSet) {
        assert!(!a.is_subset(b).unwrap());
        assert!(!b.is_superset(a).unwrap());
    }

    fn subset_test_sets() -> (USizeSet, USizeSet, USizeSet, USizeSet) {
        (
            set!(1, 10; 2, 3, 5, 7),
            set!(1, 10; 3, 5, 7),
            set!(1, 10; 1, 4, 9),
            set!(1, 10; 1, 4, 9)
        )
    }

    fn multi_word_subset_test_sets() -> (USizeSet, USizeSet, USizeSet, USizeSet, USizeSet) {
        (
            set!(1, 100; 20, 30, 50, 70),
            set!(1, 100; 30, 50, 70),
            set!(1, 100; 20, 30, 50),
            set!(1, 100; 10, 40, 90),
            set!(1, 100; 10, 40, 90)
        )
    }

    #[test]
    fn subset_relations() {
        let (a, b, c, d) = subset_test_sets();

        assert_not_subset(&a, &b);
        assert_subset(&b, &a);
        assert_not_subset(&a, &c);
        assert_not_subset(&c, &a);
        assert_subset(&c, &d);
        assert_subset(&d, &c);
    }

    #[test]
    fn multi_word_subset_relations() {
        let (a, b, c, d, e) = multi_word_subset_test_sets();

        assert_not_subset(&a, &b);
        assert_subset(&b, &a);
        assert_not_subset(&a, &c);
        assert_subset(&c, &a);
        assert_not_subset(&a, &d);
        assert_not_subset(&d, &a);
        assert_subset(&d, &e);
        assert_subset(&e, &d);
    }

    fn assert_proper_subset(a: &USizeSet, b: &USizeSet) {
        assert!(a.is_proper_subset(b).unwrap());
        assert!(b.is_proper_superset(a).unwrap());
    }

    fn assert_not_proper_subset(a: &USizeSet, b: &USizeSet) {
        assert!(!a.is_proper_subset(b).unwrap());
        assert!(!b.is_proper_superset(a).unwrap());
    }

    #[test]
    fn proper_subset_relations() {
        let (a, b, c, d) = subset_test_sets();

        assert_not_proper_subset(&a, &b);
        assert_proper_subset(&b, &a);
        assert_not_proper_subset(&a, &c);
        assert_not_proper_subset(&c, &a);
        assert_not_proper_subset(&c, &d);
        assert_not_proper_subset(&d, &c);
    }

    #[test]
    fn multi_word_proper_subset_relations() {
        let (a, b, c, d, e) = multi_word_subset_test_sets();

        assert_not_proper_subset(&a, &b);
        assert_proper_subset(&b, &a);
        assert_not_proper_subset(&a, &c);
        assert_proper_subset(&c, &a);
        assert_not_proper_subset(&a, &d);
        assert_not_proper_subset(&d, &a);
        assert_not_proper_subset(&d, &e);
        assert_not_proper_subset(&e, &d);
    }
}