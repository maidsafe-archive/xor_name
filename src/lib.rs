// Copyright 2015 MaidSafe.net limited.
//
// This SAFE Network Software is licensed to you under (1) the MaidSafe.net Commercial License,
// version 1.0 or later, or (2) The General Public License (GPL), version 3, depending on which
// licence you accepted on initial access to the Software (the "Licences").
//
// By contributing code to the SAFE Network Software, or to this project generally, you agree to be
// bound by the terms of the MaidSafe Contributor Agreement, version 1.0.  This, along with the
// Licenses can be found in the root directory of this project at LICENSE, COPYING and CONTRIBUTOR.
//
// Unless required by applicable law or agreed to in writing, the SAFE Network Software distributed
// under the GPL Licence is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.
//
// Please review the Licences for the specific language governing permissions and limitations
// relating to use of the SAFE Network Software.

extern crate rustc_serialize;
extern crate rand;

#[macro_use]
extern crate maidsafe_utilities;

use rustc_serialize::{Decodable, Decoder, Encodable, Encoder};
use rustc_serialize::hex::{ToHex, FromHex, FromHexError};
use std::cmp::Ordering;
use std::fmt;
use std::hash;
use std::ops;

pub fn slice_as_u8_64_array(slice: &[u8]) -> [u8; 64] {
    assert!(slice.len() == 64);
    let mut arr = [0u8; 64];
    // TODO (canndrew): This should use copy_memory when it's stable
    for i in 0..64 {
        arr[i] = slice[i];
    }
    arr
}

/// Constant byte length of `XorName`.
pub const XOR_NAME_LEN: usize = 64;

/// Constant bit length of `XorName`.
pub const XOR_NAME_BITS: usize = XOR_NAME_LEN * 8;

/// Returns true if both slices are equal in length and have equal contents.
pub fn slice_equal<T: PartialEq>(lhs: &[T], rhs: &[T]) -> bool {
    lhs.len() == rhs.len() && lhs.iter().zip(rhs.iter()).all(|(a, b)| a == b)
}

/// Errors that can occur when decoding a `XorName` from a string.
pub enum XorNameFromHexError {
    /// The given invalid hex character occurred at the given position.
    InvalidCharacter(char, usize),
    /// The hex string did not encode `XOR_NAME_LEN` bytes.
    InvalidLength,
}

/// A [`XOR_NAME_BITS`](constant.XOR_NAME_BITS.html)-bit number, viewed as a point in XOR space.
///
/// This has as its only field an array of [`XOR_NAME_LEN`](constant.XOR_NAME_LEN.html) bytes,
/// i. e. a number between 0 and 2<sup>XOR_NAME_BITS</sup> - 1, the `XorName`'s "ID".
///
/// XOR space is the space of these numbers, with the [XOR metric][1] as a notion of distance,
/// i. e. the points with IDs `x` and `y` are considered to have distance `x xor y`.
///
/// [1]: https://en.wikipedia.org/wiki/Kademlia#System_details
#[derive(Eq, Copy)]
pub struct XorName(pub [u8; XOR_NAME_LEN]);

#[allow(unused)]
impl XorName {
    /// Construct a XorName from a `XOR_NAME_LEN` byte array.
    pub fn new(id: [u8; XOR_NAME_LEN]) -> XorName {
        XorName(id)
    }

    /// Return the internal array.
    pub fn get_id(&self) -> [u8; XOR_NAME_LEN] {
        self.0
    }

    /// Hex-encode the `XorName` as a `String`.
    pub fn as_hex(&self) -> String {
        self.0.to_hex()
    }

    /// **Deprecated**
    ///
    /// Currently identical to `bucket_index`. This method will be replaced with
    /// `XOR_NAME_BITS - bucket_index` or removed entirely.
    pub fn bucket_distance(&self, name: &XorName) -> usize {
        self.bucket_index(name)
    }

    /// **Deprecated**
    ///
    /// Use the equivalent `cmp_distance` instead.
    pub fn cmp_closeness(&self, lhs: &XorName, rhs: &XorName) -> Ordering {
        self.cmp_distance(lhs, rhs)
    }

    /// Returns the number of leading bits in which `self` and `name` agree.
    ///
    /// Here, "leading bits" means the most significant bits. E. g. for `10101...` and `10011...`,
    /// that value will be 2, as their common prefix `10` has length 2 and the third bit is the
    /// first one in which they disagree.
    ///
    /// Equivalently, this is `XOR_NAME_BITS - bucket_distance`, where `bucket_distance` is the
    /// length of the remainders after the common prefix is removed from the IDs of `self` and
    /// `name`.
    ///
    /// The bucket distance is the magnitude of the XOR distance. More precisely, if `d > 0` is the
    /// XOR distance between `self` and `name`, the bucket distance equals `floor(log2(d))`, i. e.
    /// a bucket distance of `n` means that 2<sup>`n - 1`</sup> `<= d <` 2<sup>`n`</sup>.
    pub fn bucket_index(&self, name: &XorName) -> usize {
        for byte_index in 0..XOR_NAME_LEN {
            if self.0[byte_index] != name.0[byte_index] {
                return (byte_index * 8) +
                       (self.0[byte_index] ^ name.0[byte_index]).leading_zeros() as usize;
            }
        }
        XOR_NAME_BITS
    }

    /// Compares `lhs` and `rhs` with respect to their distance from `self`.
    pub fn cmp_distance(&self, lhs: &XorName, rhs: &XorName) -> Ordering {
        for i in 0..XOR_NAME_LEN {
            if lhs.0[i] != rhs.0[i] {
                return Ord::cmp(&(lhs.0[i] ^ self.0[i]), &(rhs.0[i] ^ self.0[i]));
            }
        }
        Ordering::Equal
    }

    /// Hex-decode a `XorName` from a `&str`.
    pub fn from_hex(s: &str) -> Result<XorName, XorNameFromHexError> {
        let data = match s.from_hex() {
            Ok(v) => v,
            Err(FromHexError::InvalidHexCharacter(c, p)) => {
                return Err(XorNameFromHexError::InvalidCharacter(c, p))
            }
            Err(FromHexError::InvalidHexLength) => return Err(XorNameFromHexError::InvalidLength),
        };
        if data.len() != XOR_NAME_LEN {
            return Err(XorNameFromHexError::InvalidLength);
        }
        Ok(XorName(slice_as_u8_64_array(&data[..])))
    }

    // Private function exposed in fmt Debug {:?} and Display {} traits.
    fn get_debug_id(&self) -> String {
        format!("{:02x}{:02x}{:02x}..{:02x}{:02x}{:02x}",
                self.0[0],
                self.0[1],
                self.0[2],
                self.0[XOR_NAME_LEN - 3],
                self.0[XOR_NAME_LEN - 2],
                self.0[XOR_NAME_LEN - 1])
    }
}


pub impl fmt::Debug for XorName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.get_debug_id())
    }
}

pub impl fmt::Display for XorName {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.get_debug_id())
    }
}

pub impl PartialEq for XorName {
    fn eq(&self, other: &XorName) -> bool {
        slice_equal(&self.0, &other.0)
    }
}

// TODO - document this if required to be public, else move to test mod
impl rand::Rand for XorName {
    fn rand<R: rand::Rng>(rng: &mut R) -> XorName {
        let mut ret = [0u8; XOR_NAME_LEN];
        for r in ret[..].iter_mut() {
            *r = <u8 as rand::Rand>::rand(rng);
        }
        XorName(ret)
    }
}

/// Returns true if `lhs` is closer to `target` than `rhs`.
///
/// "Closer" here is as per the Kademlia notion of XOR distance, i.e. the distance between two
/// `XorName`s is the bitwise XOR of their values.
///
/// Equivalently, this returns `true` if in the most significant bit where `lhs` and `rhs`
/// disagree, `lhs` agrees with `target`.
pub fn closer_to_target(lhs: &XorName, rhs: &XorName, target: &XorName) -> bool {
    target.cmp_closeness(lhs, rhs) == Ordering::Less
}

/// Returns true if `lhs` is closer to `target` than `rhs`, or when `lhs == rhs`.
///
/// "Closer" here is as per the Kademlia notion of XOR distance, i.e. the distance between two
/// `XorName`s is the bitwise XOR of their values.
pub fn closer_to_target_or_equal(lhs: &XorName, rhs: &XorName, target: &XorName) -> bool {
    target.cmp_closeness(lhs, rhs) != Ordering::Greater
}

/// The `XorName`s can be ordered from zero as an integer. This is equivalent to ordering them by
/// their distance from the name `0`.
impl Ord for XorName {
    #[inline]
    fn cmp(&self, other: &XorName) -> Ordering {
        Ord::cmp(&&self.0[..], &&other.0[..])
    }
}

impl PartialOrd for XorName {
    #[inline]
    fn partial_cmp(&self, other: &XorName) -> Option<Ordering> {
        PartialOrd::partial_cmp(&&self.0[..], &&other.0[..])
    }
    #[inline]
    fn lt(&self, other: &XorName) -> bool {
        PartialOrd::lt(&&self.0[..], &&other.0[..])
    }
    #[inline]
    fn le(&self, other: &XorName) -> bool {
        PartialOrd::le(&&self.0[..], &&other.0[..])
    }
    #[inline]
    fn gt(&self, other: &XorName) -> bool {
        PartialOrd::gt(&&self.0[..], &&other.0[..])
    }
    #[inline]
    fn ge(&self, other: &XorName) -> bool {
        PartialOrd::ge(&&self.0[..], &&other.0[..])
    }
}

impl hash::Hash for XorName {
    fn hash<H: hash::Hasher>(&self, state: &mut H) {
        state.write(&self.0[..])
    }
}

impl Clone for XorName {
    fn clone(&self) -> Self {
        let mut arr_cloned = [0u8; XOR_NAME_LEN];
        let &XorName(arr_self) = self;

        for i in 0..arr_self.len() {
            arr_cloned[i] = arr_self[i];
        }

        XorName(arr_cloned)
    }
}

impl ops::Index<ops::Range<usize>> for XorName {
    type Output = [u8];
    fn index(&self, index: ops::Range<usize>) -> &[u8] {
        let &XorName(ref b) = self;
        b.index(index)
    }
}
impl ops::Index<ops::RangeTo<usize>> for XorName {
    type Output = [u8];
    fn index(&self, index: ops::RangeTo<usize>) -> &[u8] {
        let &XorName(ref b) = self;
        b.index(index)
    }
}
impl ops::Index<ops::RangeFrom<usize>> for XorName {
    type Output = [u8];
    fn index(&self, index: ops::RangeFrom<usize>) -> &[u8] {
        let &XorName(ref b) = self;
        b.index(index)
    }
}
impl ops::Index<ops::RangeFull> for XorName {
    type Output = [u8];
    fn index(&self, index: ops::RangeFull) -> &[u8] {
        let &XorName(ref b) = self;
        b.index(index)
    }
}


impl Encodable for XorName {
    fn encode<E: Encoder>(&self, encoder: &mut E) -> Result<(), E::Error> {
        encoder.emit_seq(XOR_NAME_LEN, |encoder| {
            for (i, e) in self[..].iter().enumerate() {
                try!(encoder.emit_seq_elt(i, |encoder| e.encode(encoder)))
            }
            Ok(())
        })
    }
}

impl Decodable for XorName {
    fn decode<D: Decoder>(decoder: &mut D) -> Result<XorName, D::Error> {
        decoder.read_seq(|decoder, len| {
            if len != XOR_NAME_LEN {
                return Err(decoder.error(&format!("Expecting array of length: {}, but found {}",
                                                  XOR_NAME_LEN,
                                                  len)));
            }
            let mut res = XorName([0; XOR_NAME_LEN]);
            {
                let XorName(ref mut arr) = res;
                for (i, val) in arr.iter_mut().enumerate() {
                    *val = try!(decoder.read_seq_elt(i, |decoder| Decodable::decode(decoder)));
                }
            }
            Ok(res)
        })
    }
}

#[cfg(test)]
mod test {
    extern crate cbor;
    use super::*;
    use rand;

    #[test]
    fn serialisation_xor_name() {
        let obj_before: XorName = rand::random();
        let mut e = cbor::Encoder::from_memory();
        unwrap_result!(e.encode(&[&obj_before]));

        let mut d = cbor::Decoder::from_bytes(e.as_bytes());
        let obj_after: XorName = unwrap_result!(unwrap_option!(d.decode().next(), ""));
        assert_eq!(obj_before, obj_after);
    }

    #[test]
    fn xor_name_equal_assertion() {
        let type1: XorName = rand::random();
        let type1_clone = type1.clone();
        let type2: XorName = rand::random();
        assert_eq!(type1, type1_clone);
        assert!(type1 == type1_clone);
        assert!(!(type1 != type1_clone));
        assert!(type1 != type2);
    }

    #[test]
    fn closeness() {
        let obj0: XorName = rand::random();
        let obj0_clone = obj0.clone();
        let obj1: XorName = rand::random();
        assert!(closer_to_target(&obj0_clone, &obj1, &obj0));
        assert!(!closer_to_target(&obj1, &obj0_clone, &obj0));
    }

    #[test]
    fn format_random_nametype() {
        // test for Random XorName
        for _ in 0..5 {
            let my_name: XorName = rand::random();
            let debug_id = my_name.get_debug_id();
            let full_id = my_name.as_hex();
            assert_eq!(debug_id.len(), 14);
            assert_eq!(full_id.len(), 2 * XOR_NAME_LEN);
            assert_eq!(&debug_id[0..6], &full_id[0..6]);
            assert_eq!(&debug_id[8..14],
                       &full_id[2 * XOR_NAME_LEN - 6..2 * XOR_NAME_LEN]);
            assert_eq!(&debug_id[6..8], "..");
        }
    }

    #[test]
    fn format_fixed_low_char_nametype() {
        // test for fixed low char values in XorName
        let low_char_id = [1u8; XOR_NAME_LEN];
        let my_low_char_name = XorName::new(low_char_id);
        let debug_id = my_low_char_name.get_debug_id();
        let full_id = my_low_char_name.as_hex();
        assert_eq!(debug_id.len(), 14);
        assert_eq!(full_id.len(), 2 * XOR_NAME_LEN);
        assert_eq!(&debug_id[0..6], &full_id[0..6]);
        assert_eq!(&debug_id[8..14],
                   &full_id[2 * XOR_NAME_LEN - 6..2 * XOR_NAME_LEN]);
        assert_eq!(&debug_id[6..8], "..");
    }
}
