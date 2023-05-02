// Copyright 2015-2020 Parity Technologies
// Copyright 2023-2023 Ethers-rs Team
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! ABI encoder.
//!
//! ### `encode/decode_single`
//!
//! [`crate::SolType::encode_single()`] and [`encode_single()`] operate on a
//! single token. They wrap this token in a tuple, and pass it to the encoder.
//! Use this interface when abi-encoding a single token. This is suitable for
//! encoding a type in isolation, or for encoding parameters for single-param
//! functions.
//!
//! ### `encode/decode_params`
//!
//! [`crate::SolType::encode_params()`] and [`encode_params()`] operate on a
//! sequence. If the sequence is a tuple, the tuple is inferred to be a set of
//! Solidity function parameters,
//!
//! The corresponding [`crate::SolType::decode_params()`] and
//! [`crate::decode_params()`] reverse this operation, decoding a tuple from a
//! blob.
//!
//! This is used to encode the parameters for a solidity function
//!
//! ### `encode/decode`
//!
//! [`crate::SolType::encode()`] and [`encode()`] operate on a sequence of
//! tokens. This sequence is inferred not to be function parameters.
//!
//! This is the least useful one. Most users will not need it.

#[cfg(not(feature = "std"))]
use crate::no_std_prelude::*;
use crate::{token::TokenSeq, util::pad_u32, TokenType, Word};

/// An ABI encoder. This is not intended for public consumption. It should be
/// used only by the token types. If you have found yourself here, you probably
/// want to use the high-level [`crate::SolType`] interface (or its dynamic
/// equivalent) instead.
#[derive(Default, Clone, Debug)]
pub struct Encoder {
    buf: Vec<Word>,
    suffix_offset: Vec<u32>,
}

impl Encoder {
    /// Instantiate a new encoder with a given capacity in words.
    pub fn with_capacity(size: usize) -> Self {
        Self {
            buf: Vec::with_capacity(size + 1),
            suffix_offset: vec![],
        }
    }

    /// Finish the encoding process, returning the encoded words
    // https://github.com/rust-lang/rust-clippy/issues/4979
    #[allow(clippy::missing_const_for_fn)]
    pub fn finish(self) -> Vec<Word> {
        self.buf
    }

    /// Finish the encoding process, returning the encoded bytes
    pub fn into_bytes(self) -> Vec<u8> {
        self.buf
            .into_iter()
            .flat_map(Word::to_fixed_bytes)
            .collect()
    }

    /// Determine the current suffix offset
    pub fn suffix_offset(&self) -> u32 {
        *self.suffix_offset.last().unwrap()
    }

    /// Push a new suffix offset
    pub fn push_offset(&mut self, words: u32) {
        self.suffix_offset.push(words * 32);
    }

    /// Pop the last suffix offset
    pub fn pop_offset(&mut self) -> u32 {
        self.suffix_offset.pop().unwrap()
    }

    /// Bump the suffix offset by a given number of words
    pub fn bump_offset(&mut self, words: u32) {
        (*self.suffix_offset.last_mut().unwrap()) += words * 32;
    }

    /// Append a word to the encoder
    pub fn append_word(&mut self, word: Word) {
        self.buf.push(word);
    }

    /// Append a pointer to the current suffix offset
    pub fn append_indirection(&mut self) {
        self.append_word(pad_u32(self.suffix_offset()));
    }

    /// Append a sequence length
    pub fn append_seq_len<T>(&mut self, seq: &[T]) {
        self.append_word(pad_u32(seq.len() as u32));
    }

    /// Append a seqeunce of bytes, padding to the next word
    fn append_bytes(&mut self, bytes: &[u8]) {
        let len = (bytes.len() + 31) / 32;
        for i in 0..len {
            let mut padded = Word::default();

            let to_copy = match i == len - 1 {
                false => 32,
                true => match bytes.len() % 32 {
                    0 => 32,
                    x => x,
                },
            };

            let offset = 32 * i;
            padded[..to_copy].copy_from_slice(&bytes[offset..offset + to_copy]);
            self.append_word(padded);
        }
    }

    /// Append a sequence of bytes as a packed sequence with a length prefix
    pub fn append_packed_seq(&mut self, bytes: &[u8]) {
        self.append_seq_len(bytes);
        self.append_bytes(bytes);
    }

    /// Shortcut for appending a token sequence
    pub fn append_head_tail<T>(&mut self, token: &T)
    where
        T: TokenSeq,
    {
        token.encode_sequence(self);
    }
}

/// Encodes vector of tokens into ABI-compliant vector of bytes.
pub(crate) fn encode_impl<T>(tokens: T) -> Vec<u8>
where
    T: TokenSeq,
{
    let mut enc = Encoder::with_capacity(tokens.total_words());

    enc.append_head_tail(&tokens);

    enc.finish()
        .into_iter()
        .flat_map(Word::to_fixed_bytes)
        .collect()
}

/// Encode an ABI token sequence
pub fn encode<T>(token: T) -> Vec<u8>
where
    T: TokenSeq,
{
    encode_impl(token)
}

/// Encode a single token
pub fn encode_single<T>(token: T) -> Vec<u8>
where
    T: TokenType,
{
    encode((token,))
}

/// Encode a tuple as ABI function params, suitable for passing to a function
pub fn encode_params<T>(token: T) -> Vec<u8>
where
    T: TokenSeq,
{
    if T::can_be_params() {
        encode(token)
    } else {
        encode((token,))
    }
}

#[cfg(test)]
mod tests {
    use ethers_primitives::{Address, U256};
    use hex_literal::hex;

    #[cfg(not(feature = "std"))]
    use crate::no_std_prelude::*;
    use crate::{sol_type, util::pad_u32, SolType};

    #[test]
    fn encode_address() {
        let address = Address::from([0x11u8; 20]);
        let expected = hex!("0000000000000000000000001111111111111111111111111111111111111111");
        let encoded = sol_type::Address::encode_single(address);
        assert_eq!(encoded, expected);
    }

    #[test]
    fn encode_dynamic_array_of_addresses() {
        type MyTy = sol_type::Array<sol_type::Address>;
        let rust = vec![Address::from([0x11u8; 20]), Address::from([0x22u8; 20])];
        let encoded = MyTy::encode_single(rust);
        let expected = hex!(
            "
			0000000000000000000000000000000000000000000000000000000000000020
			0000000000000000000000000000000000000000000000000000000000000002
			0000000000000000000000001111111111111111111111111111111111111111
			0000000000000000000000002222222222222222222222222222222222222222
		"
        )
        .to_vec();
        assert_eq!(encoded, expected);
    }

    #[test]
    fn encode_fixed_array_of_addresses() {
        type MyTy = sol_type::FixedArray<sol_type::Address, 2>;

        let addresses = [Address::from([0x11u8; 20]), Address::from([0x22u8; 20])];

        let encoded = MyTy::encode_single(addresses);
        let encoded_params = MyTy::encode_params(addresses);
        let expected = hex!(
            "
    		0000000000000000000000001111111111111111111111111111111111111111
    		0000000000000000000000002222222222222222222222222222222222222222
    	"
        )
        .to_vec();
        assert_eq!(encoded_params, expected);
        assert_eq!(encoded, expected);
    }

    #[test]
    fn encode_two_addresses() {
        type MyTy = (sol_type::Address, sol_type::Address);
        let addresses = (Address::from([0x11u8; 20]), Address::from([0x22u8; 20]));

        let encoded = MyTy::encode(addresses);
        let encoded_params = MyTy::encode_params(addresses);
        let expected = hex!(
            "
    		0000000000000000000000001111111111111111111111111111111111111111
    		0000000000000000000000002222222222222222222222222222222222222222
    	"
        )
        .to_vec();
        assert_eq!(encoded, expected);
        assert_eq!(encoded_params, expected);
    }

    #[test]
    fn encode_fixed_array_of_dynamic_array_of_addresses() {
        type MyTy = sol_type::FixedArray<sol_type::Array<sol_type::Address>, 2>;
        let fixed = [
            vec![Address::from([0x11u8; 20]), Address::from([0x22u8; 20])],
            vec![Address::from([0x33u8; 20]), Address::from([0x44u8; 20])],
        ];

        let expected = hex!(
            "
    		0000000000000000000000000000000000000000000000000000000000000020
    		0000000000000000000000000000000000000000000000000000000000000040
    		00000000000000000000000000000000000000000000000000000000000000a0
    		0000000000000000000000000000000000000000000000000000000000000002
    		0000000000000000000000001111111111111111111111111111111111111111
    		0000000000000000000000002222222222222222222222222222222222222222
    		0000000000000000000000000000000000000000000000000000000000000002
    		0000000000000000000000003333333333333333333333333333333333333333
    		0000000000000000000000004444444444444444444444444444444444444444
    	"
        )
        .to_vec();
        let encoded = MyTy::encode_single(fixed.clone());
        assert_eq!(encoded, expected);
        let encoded_params = MyTy::encode_params(fixed);
        assert_eq!(encoded_params, expected);
    }

    #[test]
    fn encode_dynamic_array_of_fixed_array_of_addresses() {
        type TwoAddrs = sol_type::FixedArray<sol_type::Address, 2>;
        type MyTy = sol_type::Array<TwoAddrs>;

        let dynamic = vec![
            [Address::from([0x11u8; 20]), Address::from([0x22u8; 20])],
            [Address::from([0x33u8; 20]), Address::from([0x44u8; 20])],
        ];

        let expected = hex!(
            "
    		0000000000000000000000000000000000000000000000000000000000000020
    		0000000000000000000000000000000000000000000000000000000000000002
    		0000000000000000000000001111111111111111111111111111111111111111
    		0000000000000000000000002222222222222222222222222222222222222222
    		0000000000000000000000003333333333333333333333333333333333333333
    		0000000000000000000000004444444444444444444444444444444444444444
    	"
        )
        .to_vec();
        // a DynSeq at top level ALWAYS has extra indirection
        let encoded = MyTy::encode_single(dynamic.clone());
        assert_eq!(encoded, expected);
        let encoded_params = MyTy::encode_params(dynamic);
        assert_eq!(encoded_params, expected);
    }

    #[test]
    fn encode_dynamic_array_of_dynamic_arrays() {
        type MyTy = sol_type::Array<sol_type::Array<sol_type::Address>>;

        let dynamic = vec![
            vec![Address::from([0x11u8; 20])],
            vec![Address::from([0x22u8; 20])],
        ];

        let expected = hex!(
            "
    		0000000000000000000000000000000000000000000000000000000000000020
    		0000000000000000000000000000000000000000000000000000000000000002
    		0000000000000000000000000000000000000000000000000000000000000040
    		0000000000000000000000000000000000000000000000000000000000000080
    		0000000000000000000000000000000000000000000000000000000000000001
    		0000000000000000000000001111111111111111111111111111111111111111
    		0000000000000000000000000000000000000000000000000000000000000001
    		0000000000000000000000002222222222222222222222222222222222222222
    	"
        )
        .to_vec();
        // a DynSeq at top level ALWAYS has extra indirection
        let encoded = MyTy::encode_single(dynamic.clone());
        assert_eq!(encoded, expected);
        let encoded_params = MyTy::encode_params(dynamic);
        assert_eq!(encoded_params, expected);
    }

    #[test]
    fn encode_dynamic_array_of_dynamic_arrays2() {
        type MyTy = sol_type::Array<sol_type::Array<sol_type::Address>>;

        let dynamic = vec![
            vec![Address::from([0x11u8; 20]), Address::from([0x22u8; 20])],
            vec![Address::from([0x33u8; 20]), Address::from([0x44u8; 20])],
        ];
        let expected = hex!(
            "
    		0000000000000000000000000000000000000000000000000000000000000020
    		0000000000000000000000000000000000000000000000000000000000000002
    		0000000000000000000000000000000000000000000000000000000000000040
    		00000000000000000000000000000000000000000000000000000000000000a0
    		0000000000000000000000000000000000000000000000000000000000000002
    		0000000000000000000000001111111111111111111111111111111111111111
    		0000000000000000000000002222222222222222222222222222222222222222
    		0000000000000000000000000000000000000000000000000000000000000002
    		0000000000000000000000003333333333333333333333333333333333333333
    		0000000000000000000000004444444444444444444444444444444444444444
    	"
        )
        .to_vec();
        // a DynSeq at top level ALWAYS has extra indirection
        let encoded = MyTy::encode_single(dynamic.clone());
        assert_eq!(encoded, expected);
        let encoded_params = MyTy::encode_params(dynamic);
        assert_eq!(encoded_params, expected);
    }

    #[test]
    fn encode_fixed_array_of_fixed_arrays() {
        type MyTy = sol_type::FixedArray<sol_type::FixedArray<sol_type::Address, 2>, 2>;

        let fixed = [
            [Address::from([0x11u8; 20]), Address::from([0x22u8; 20])],
            [Address::from([0x33u8; 20]), Address::from([0x44u8; 20])],
        ];

        let encoded = MyTy::encode(fixed);
        let encoded_params = MyTy::encode_params(fixed);
        let expected = hex!(
            "
    		0000000000000000000000001111111111111111111111111111111111111111
    		0000000000000000000000002222222222222222222222222222222222222222
    		0000000000000000000000003333333333333333333333333333333333333333
    		0000000000000000000000004444444444444444444444444444444444444444
    	"
        )
        .to_vec();
        // a non-dynamic FixedSeq at top level NEVER has extra indirection
        assert_eq!(encoded, expected);
        assert_eq!(encoded_params, expected);
    }

    #[test]
    fn encode_fixed_array_of_static_tuples_followed_by_dynamic_type() {
        type Tup = (sol_type::Uint<256>, sol_type::Uint<256>, sol_type::Address);
        type Fixed = sol_type::FixedArray<Tup, 2>;
        type MyTy = (Fixed, sol_type::String);

        let data = (
            [
                (
                    U256::from(93523141),
                    U256::from(352332135),
                    Address::from([0x44u8; 20]),
                ),
                (
                    U256::from(12411),
                    U256::from(451),
                    Address::from([0x22u8; 20]),
                ),
            ],
            "gavofyork".to_string(),
        );

        let expected = hex!(
            "
    		0000000000000000000000000000000000000000000000000000000005930cc5
    		0000000000000000000000000000000000000000000000000000000015002967
    		0000000000000000000000004444444444444444444444444444444444444444
    		000000000000000000000000000000000000000000000000000000000000307b
    		00000000000000000000000000000000000000000000000000000000000001c3
    		0000000000000000000000002222222222222222222222222222222222222222
    		00000000000000000000000000000000000000000000000000000000000000e0
    		0000000000000000000000000000000000000000000000000000000000000009
    		6761766f66796f726b0000000000000000000000000000000000000000000000
    	"
        )
        .to_vec();
        // a dynamic FixedSeq at top level should start with indirection
        // when not param encoded. For this particular test, there was an
        // implicit param incoding
        let encoded = MyTy::encode_single(data.clone());
        assert_ne!(encoded, expected);

        let encoded_params = MyTy::encode_params(data);
        assert_eq!(encoded_params, expected);
        assert_eq!(encoded_params.len() + 32, encoded.len());
    }

    #[test]
    fn encode_empty_array() {
        type MyTy0 = (sol_type::Array<sol_type::Address>,);

        let data = (vec![],);

        // Empty arrays
        let encoded = MyTy0::encode_single(data.clone());
        let encoded_params = MyTy0::encode_params(data);
        let expected = hex!(
            "
    		0000000000000000000000000000000000000000000000000000000000000020
    		0000000000000000000000000000000000000000000000000000000000000000
    	    "
        )
        .to_vec();
        // a dynamic FixedSeq at top level should start with indirection
        // when not param encoded. For this particular test, there was an
        // implicit param incoding
        assert_ne!(encoded, expected);
        assert_eq!(encoded_params, expected);
        assert_eq!(encoded_params.len() + 32, encoded.len());

        type MyTy = (
            sol_type::Array<sol_type::Address>,
            sol_type::Array<sol_type::Address>,
        );
        let data = (vec![], vec![]);

        let expected = hex!(
            "
    		0000000000000000000000000000000000000000000000000000000000000040
    		0000000000000000000000000000000000000000000000000000000000000060
    		0000000000000000000000000000000000000000000000000000000000000000
    		0000000000000000000000000000000000000000000000000000000000000000
    	    "
        )
        .to_vec();
        // a dynamic FixedSeq at top level should start with indirection
        // when not param encoded. For this particular test, there was an
        // implicit param incoding
        // Empty arrays
        let encoded = MyTy::encode_single(data.clone());
        assert_ne!(encoded, expected);
        let encoded_params = MyTy::encode_params(data);
        assert_eq!(encoded_params, expected);
        assert_eq!(encoded_params.len() + 32, encoded.len());

        type MyTy2 = (
            sol_type::Array<sol_type::Array<sol_type::Address>>,
            sol_type::Array<sol_type::Array<sol_type::Address>>,
        );
        let data = (vec![vec![]], vec![vec![]]);

        // Nested empty arrays
        let expected = hex!(
            "
    		0000000000000000000000000000000000000000000000000000000000000040
    		00000000000000000000000000000000000000000000000000000000000000a0
    		0000000000000000000000000000000000000000000000000000000000000001
    		0000000000000000000000000000000000000000000000000000000000000020
    		0000000000000000000000000000000000000000000000000000000000000000
    		0000000000000000000000000000000000000000000000000000000000000001
    		0000000000000000000000000000000000000000000000000000000000000020
    		0000000000000000000000000000000000000000000000000000000000000000
    	"
        )
        .to_vec();
        // a dynamic FixedSeq at top level should start with indirection
        // when not param encoded. For this particular test, there was an
        // implicit param incoding
        let encoded = MyTy2::encode_single(data.clone());
        assert_ne!(encoded, expected);
        let encoded_params = MyTy2::encode_params(data);
        assert_eq!(encoded_params, expected);
        assert_eq!(encoded_params.len() + 32, encoded.len());
    }

    #[test]
    fn encode_bytes() {
        type MyTy = sol_type::Bytes;
        let bytes = vec![0x12, 0x34];

        let encoded = MyTy::encode_single(bytes);
        let expected = hex!(
            "
    		0000000000000000000000000000000000000000000000000000000000000020
    		0000000000000000000000000000000000000000000000000000000000000002
    		1234000000000000000000000000000000000000000000000000000000000000
    	"
        )
        .to_vec();
        assert_eq!(encoded, expected);
    }

    #[test]
    fn encode_fixed_bytes() {
        let encoded = sol_type::FixedBytes::<2>::encode_single([0x12, 0x34]);
        let expected = hex!("1234000000000000000000000000000000000000000000000000000000000000");
        assert_eq!(encoded, expected);
    }

    #[test]
    fn encode_string() {
        let s = "gavofyork".to_string();
        let encoded = sol_type::String::encode_single(s);
        let expected = hex!(
            "
    		0000000000000000000000000000000000000000000000000000000000000020
    		0000000000000000000000000000000000000000000000000000000000000009
    		6761766f66796f726b0000000000000000000000000000000000000000000000
    	"
        )
        .to_vec();
        assert_eq!(encoded, expected);
    }

    #[test]
    fn encode_bytes2() {
        let bytes = hex!("10000000000000000000000000000000000000000000000000000000000002").to_vec();
        let encoded = sol_type::Bytes::encode_single(bytes);
        let expected = hex!(
            "
    		0000000000000000000000000000000000000000000000000000000000000020
    		000000000000000000000000000000000000000000000000000000000000001f
    		1000000000000000000000000000000000000000000000000000000000000200
    	"
        )
        .to_vec();
        assert_eq!(encoded, expected);
    }

    #[test]
    fn encode_bytes3() {
        let bytes = hex!(
            "
    		1000000000000000000000000000000000000000000000000000000000000000
    		1000000000000000000000000000000000000000000000000000000000000000
    	"
        )
        .to_vec();
        let encoded = sol_type::Bytes::encode_single(bytes);
        let expected = hex!(
            "
    		0000000000000000000000000000000000000000000000000000000000000020
    		0000000000000000000000000000000000000000000000000000000000000040
    		1000000000000000000000000000000000000000000000000000000000000000
    		1000000000000000000000000000000000000000000000000000000000000000
    	"
        )
        .to_vec();
        assert_eq!(encoded, expected);
    }

    #[test]
    fn encode_two_bytes() {
        type MyTy = (sol_type::Bytes, sol_type::Bytes);

        let bytes = (
            hex!("10000000000000000000000000000000000000000000000000000000000002").to_vec(),
            hex!("0010000000000000000000000000000000000000000000000000000000000002").to_vec(),
        );
        let encoded = MyTy::encode_single(bytes.clone());
        let encoded_params = MyTy::encode_params(bytes);
        let expected = hex!(
            "
    		0000000000000000000000000000000000000000000000000000000000000040
    		0000000000000000000000000000000000000000000000000000000000000080
    		000000000000000000000000000000000000000000000000000000000000001f
    		1000000000000000000000000000000000000000000000000000000000000200
    		0000000000000000000000000000000000000000000000000000000000000020
    		0010000000000000000000000000000000000000000000000000000000000002
    	"
        )
        .to_vec();
        // a dynamic FixedSeq at top level should start with indirection
        // when not param encoded. For this particular test, there was an
        // implicit param incoding
        assert_ne!(encoded, expected);
        assert_eq!(encoded_params, expected);
        assert_eq!(encoded_params.len() + 32, encoded.len());
    }

    #[test]
    fn encode_uint() {
        let uint = 4;
        let encoded = sol_type::Uint::<8>::encode_single(uint);
        let expected = hex!("0000000000000000000000000000000000000000000000000000000000000004");
        assert_eq!(encoded, expected);
    }

    #[test]
    fn encode_int() {
        let int = 4;
        let encoded = sol_type::Int::<8>::encode_single(int);
        let expected = hex!("0000000000000000000000000000000000000000000000000000000000000004");
        assert_eq!(encoded, expected);
    }

    #[test]
    fn encode_bool() {
        let encoded = sol_type::Bool::encode_single(true);
        let expected = hex!("0000000000000000000000000000000000000000000000000000000000000001");
        assert_eq!(encoded, expected);
    }

    #[test]
    fn encode_bool2() {
        let encoded = sol_type::Bool::encode_single(false);
        let expected = hex!("0000000000000000000000000000000000000000000000000000000000000000");
        assert_eq!(encoded, expected);
    }

    #[test]
    fn comprehensive_test() {
        type MyTy = (
            sol_type::Uint<8>,
            sol_type::Bytes,
            sol_type::Uint<8>,
            sol_type::Bytes,
        );

        let bytes = hex!(
            "
    		131a3afc00d1b1e3461b955e53fc866dcf303b3eb9f4c16f89e388930f48134b
    		131a3afc00d1b1e3461b955e53fc866dcf303b3eb9f4c16f89e388930f48134b
    	"
        )
        .to_vec();

        let data = (5, bytes.clone(), 3, bytes);

        let encoded = MyTy::encode_single(data.clone());
        let encoded_params = MyTy::encode_params(data);

        let expected = hex!(
            "
    		0000000000000000000000000000000000000000000000000000000000000005
    		0000000000000000000000000000000000000000000000000000000000000080
    		0000000000000000000000000000000000000000000000000000000000000003
    		00000000000000000000000000000000000000000000000000000000000000e0
    		0000000000000000000000000000000000000000000000000000000000000040
    		131a3afc00d1b1e3461b955e53fc866dcf303b3eb9f4c16f89e388930f48134b
    		131a3afc00d1b1e3461b955e53fc866dcf303b3eb9f4c16f89e388930f48134b
    		0000000000000000000000000000000000000000000000000000000000000040
    		131a3afc00d1b1e3461b955e53fc866dcf303b3eb9f4c16f89e388930f48134b
    		131a3afc00d1b1e3461b955e53fc866dcf303b3eb9f4c16f89e388930f48134b
    	"
        )
        .to_vec();
        // a dynamic FixedSeq at top level should start with indirection
        // when not param encoded. For this particular test, there was an
        // implicit param incoding
        assert_ne!(encoded, expected);
        assert_eq!(encoded_params, expected);
        assert_eq!(encoded_params.len() + 32, encoded.len());
    }

    #[test]
    fn test_pad_u32() {
        // this will fail if endianess is not supported
        assert_eq!(pad_u32(0x1)[31], 1);
        assert_eq!(pad_u32(0x100)[30], 1);
    }

    #[test]
    fn comprehensive_test2() {
        type MyTy = (
            sol_type::Bool,
            sol_type::String,
            sol_type::Uint<8>,
            sol_type::Uint<8>,
            sol_type::Uint<8>,
            sol_type::Array<sol_type::Uint<8>>,
        );

        let data = (true, "gavofyork".to_string(), 2, 3, 4, vec![5, 6, 7]);

        let expected = hex!(
            "
    		0000000000000000000000000000000000000000000000000000000000000001
    		00000000000000000000000000000000000000000000000000000000000000c0
    		0000000000000000000000000000000000000000000000000000000000000002
    		0000000000000000000000000000000000000000000000000000000000000003
    		0000000000000000000000000000000000000000000000000000000000000004
    		0000000000000000000000000000000000000000000000000000000000000100
    		0000000000000000000000000000000000000000000000000000000000000009
    		6761766f66796f726b0000000000000000000000000000000000000000000000
    		0000000000000000000000000000000000000000000000000000000000000003
    		0000000000000000000000000000000000000000000000000000000000000005
    		0000000000000000000000000000000000000000000000000000000000000006
    		0000000000000000000000000000000000000000000000000000000000000007
    	"
        )
        .to_vec();
        // a dynamic FixedSeq at top level should start with indirection
        // when not param encoded. For this particular test, there was an
        // implicit param incoding
        let encoded = MyTy::encode_single(data.clone());
        assert_ne!(encoded, expected);
        let encoded_params = MyTy::encode_params(data);
        assert_eq!(encoded_params, expected);
        assert_eq!(encoded_params.len() + 32, encoded.len());
    }

    #[test]
    fn encode_dynamic_array_of_bytes() {
        type MyTy = sol_type::Array<sol_type::Bytes>;
        let data = vec![hex!(
            "019c80031b20d5e69c8093a571162299032018d913930d93ab320ae5ea44a4218a274f00d607"
        )
        .to_vec()];

        let expected = hex!(
            "
    		0000000000000000000000000000000000000000000000000000000000000020
    		0000000000000000000000000000000000000000000000000000000000000001
    		0000000000000000000000000000000000000000000000000000000000000020
    		0000000000000000000000000000000000000000000000000000000000000026
    		019c80031b20d5e69c8093a571162299032018d913930d93ab320ae5ea44a421
    		8a274f00d6070000000000000000000000000000000000000000000000000000
    	"
        )
        .to_vec();
        // a DynSeq at top level ALWAYS has extra indirection
        let encoded = MyTy::encode_single(data.clone());
        assert_eq!(encoded, expected);
        let encoded_params = MyTy::encode_params(data);
        assert_eq!(encoded_params, expected);
    }

    #[test]
    fn encode_dynamic_array_of_bytes2() {
        type MyTy = sol_type::Array<sol_type::Bytes>;

        let data = vec![
            hex!("4444444444444444444444444444444444444444444444444444444444444444444444444444")
                .to_vec(),
            hex!("6666666666666666666666666666666666666666666666666666666666666666666666666666")
                .to_vec(),
        ];

        let expected = hex!(
            "
    		0000000000000000000000000000000000000000000000000000000000000020
    		0000000000000000000000000000000000000000000000000000000000000002
    		0000000000000000000000000000000000000000000000000000000000000040
    		00000000000000000000000000000000000000000000000000000000000000a0
    		0000000000000000000000000000000000000000000000000000000000000026
    		4444444444444444444444444444444444444444444444444444444444444444
    		4444444444440000000000000000000000000000000000000000000000000000
    		0000000000000000000000000000000000000000000000000000000000000026
    		6666666666666666666666666666666666666666666666666666666666666666
    		6666666666660000000000000000000000000000000000000000000000000000
    	"
        )
        .to_vec();
        // a DynSeq at top level ALWAYS has extra indirection
        let encoded = MyTy::encode_single(data.clone());
        assert_eq!(encoded, expected);
        let encoded_params = MyTy::encode_params(data);
        assert_eq!(encoded_params, expected);
    }

    #[test]
    fn encode_static_tuple_of_addresses() {
        type MyTy = (sol_type::Address, sol_type::Address);
        let data = (Address::from([0x11u8; 20]), Address::from([0x22u8; 20]));

        let encoded = MyTy::encode(data);
        let encoded_params = MyTy::encode_params(data);

        let expected = hex!(
            "
    		0000000000000000000000001111111111111111111111111111111111111111
    		0000000000000000000000002222222222222222222222222222222222222222
    	"
        )
        .to_vec();
        assert_eq!(encoded, expected);
        assert_eq!(encoded_params, expected);
    }

    #[test]
    fn encode_dynamic_tuple() {
        type MyTy = (sol_type::String, sol_type::String);
        let data = ("gavofyork".to_string(), "gavofyork".to_string());

        let expected = hex!(
            "
    		0000000000000000000000000000000000000000000000000000000000000020
    		0000000000000000000000000000000000000000000000000000000000000040
    		0000000000000000000000000000000000000000000000000000000000000080
    		0000000000000000000000000000000000000000000000000000000000000009
    		6761766f66796f726b0000000000000000000000000000000000000000000000
    		0000000000000000000000000000000000000000000000000000000000000009
    		6761766f66796f726b0000000000000000000000000000000000000000000000
    	"
        )
        .to_vec();
        // a dynamic FixedSeq at top level should start with indirection
        // when not param encoded.
        let encoded = MyTy::encode_single(data.clone());
        assert_eq!(encoded, expected);
        let encoded_params = MyTy::encode_params(data);
        assert_ne!(encoded_params, expected);
        assert_eq!(encoded_params.len() + 32, encoded.len());
    }

    #[test]
    fn encode_dynamic_tuple_of_bytes2() {
        type MyTy = (sol_type::Bytes, sol_type::Bytes);

        let data = (
            hex!("4444444444444444444444444444444444444444444444444444444444444444444444444444")
                .to_vec(),
            hex!("6666666666666666666666666666666666666666666666666666666666666666666666666666")
                .to_vec(),
        );

        let encoded = MyTy::encode_single(data.clone());
        let encoded_params = MyTy::encode_params(data);

        let expected = hex!(
            "
    		0000000000000000000000000000000000000000000000000000000000000020
    		0000000000000000000000000000000000000000000000000000000000000040
    		00000000000000000000000000000000000000000000000000000000000000a0
    		0000000000000000000000000000000000000000000000000000000000000026
    		4444444444444444444444444444444444444444444444444444444444444444
    		4444444444440000000000000000000000000000000000000000000000000000
    		0000000000000000000000000000000000000000000000000000000000000026
    		6666666666666666666666666666666666666666666666666666666666666666
    		6666666666660000000000000000000000000000000000000000000000000000
    	"
        )
        .to_vec();
        // a dynamic FixedSeq at top level should start with indirection
        // when not param encoded.
        assert_eq!(encoded, expected);
        assert_ne!(encoded_params, expected);
        assert_eq!(encoded_params.len() + 32, encoded.len());
    }

    #[test]
    fn encode_complex_tuple() {
        type MyTy = (
            sol_type::Uint<256>,
            sol_type::String,
            sol_type::Address,
            sol_type::Address,
        );

        let data = (
            U256::from_be_bytes::<32>([0x11u8; 32]),
            "gavofyork".to_owned(),
            Address::from([0x11u8; 20]),
            Address::from([0x22u8; 20]),
        );

        let expected = hex!(
            "
            0000000000000000000000000000000000000000000000000000000000000020
            1111111111111111111111111111111111111111111111111111111111111111
            0000000000000000000000000000000000000000000000000000000000000080
            0000000000000000000000001111111111111111111111111111111111111111
    		0000000000000000000000002222222222222222222222222222222222222222
    		0000000000000000000000000000000000000000000000000000000000000009
    		6761766f66796f726b0000000000000000000000000000000000000000000000
    	"
        )
        .to_vec();
        // a dynamic FixedSeq at top level should start with indirection
        // when not param encoded.
        let encoded = MyTy::encode_single(data.clone());
        assert_eq!(encoded, expected);
        let encoded_params = MyTy::encode_params(data);
        assert_ne!(encoded_params, expected);
        assert_eq!(encoded_params.len() + 32, encoded.len());
    }

    #[test]
    fn encode_nested_tuple() {
        type MyTy = (
            sol_type::String,
            sol_type::Bool,
            sol_type::String,
            (
                sol_type::String,
                sol_type::String,
                (sol_type::String, sol_type::String),
            ),
        );

        let data = (
            "test".to_string(),
            true,
            "cyborg".to_string(),
            (
                "night".to_string(),
                "day".to_string(),
                ("weee".to_string(), "funtests".to_string()),
            ),
        );

        let encoded = MyTy::encode_single(data.clone());
        let encoded_params = MyTy::encode_params(data);

        let expected = hex!(
            "
    		0000000000000000000000000000000000000000000000000000000000000020
    		0000000000000000000000000000000000000000000000000000000000000080
    		0000000000000000000000000000000000000000000000000000000000000001
    		00000000000000000000000000000000000000000000000000000000000000c0
    		0000000000000000000000000000000000000000000000000000000000000100
    		0000000000000000000000000000000000000000000000000000000000000004
    		7465737400000000000000000000000000000000000000000000000000000000
    		0000000000000000000000000000000000000000000000000000000000000006
    		6379626f72670000000000000000000000000000000000000000000000000000
    		0000000000000000000000000000000000000000000000000000000000000060
    		00000000000000000000000000000000000000000000000000000000000000a0
    		00000000000000000000000000000000000000000000000000000000000000e0
    		0000000000000000000000000000000000000000000000000000000000000005
    		6e69676874000000000000000000000000000000000000000000000000000000
    		0000000000000000000000000000000000000000000000000000000000000003
    		6461790000000000000000000000000000000000000000000000000000000000
    		0000000000000000000000000000000000000000000000000000000000000040
    		0000000000000000000000000000000000000000000000000000000000000080
    		0000000000000000000000000000000000000000000000000000000000000004
    		7765656500000000000000000000000000000000000000000000000000000000
    		0000000000000000000000000000000000000000000000000000000000000008
    		66756e7465737473000000000000000000000000000000000000000000000000
    	"
        )
        .to_vec();
        // a dynamic FixedSeq at top level should start with indirection
        // when not param encoded
        assert_eq!(encoded, expected);
        assert_ne!(encoded_params, expected);
        assert_eq!(encoded_params.len() + 32, encoded.len());
    }

    #[test]
    fn encode_params_containing_dynamic_tuple() {
        type MyTy = (
            sol_type::Address,
            (sol_type::Bool, sol_type::String, sol_type::String),
            sol_type::Address,
            sol_type::Address,
            sol_type::Bool,
        );
        let data = (
            Address::from([0x22u8; 20]),
            (true, "spaceship".to_owned(), "cyborg".to_owned()),
            Address::from([0x33u8; 20]),
            Address::from([0x44u8; 20]),
            false,
        );

        let encoded = MyTy::encode_single(data.clone());
        let encoded_params = MyTy::encode_params(data);

        let expected = hex!(
            "
    		0000000000000000000000002222222222222222222222222222222222222222
    		00000000000000000000000000000000000000000000000000000000000000a0
    		0000000000000000000000003333333333333333333333333333333333333333
    		0000000000000000000000004444444444444444444444444444444444444444
    		0000000000000000000000000000000000000000000000000000000000000000
    		0000000000000000000000000000000000000000000000000000000000000001
    		0000000000000000000000000000000000000000000000000000000000000060
    		00000000000000000000000000000000000000000000000000000000000000a0
    		0000000000000000000000000000000000000000000000000000000000000009
    		7370616365736869700000000000000000000000000000000000000000000000
    		0000000000000000000000000000000000000000000000000000000000000006
    		6379626f72670000000000000000000000000000000000000000000000000000
    	"
        )
        .to_vec();
        // a dynamic FixedSeq at top level should start with indirection
        // when not param encoded. For this particular test, there was an
        // implicit param incoding
        assert_ne!(encoded, expected);
        assert_eq!(encoded_params, expected);
        assert_eq!(encoded_params.len() + 32, encoded.len());
    }

    #[test]
    fn encode_params_containing_static_tuple() {
        type MyTy = (
            sol_type::Address,
            (sol_type::Address, sol_type::Bool, sol_type::Bool),
            sol_type::Address,
            sol_type::Address,
        );

        let data = (
            Address::from([0x11u8; 20]),
            (Address::from([0x22u8; 20]), true, false),
            Address::from([0x33u8; 20]),
            Address::from([0x44u8; 20]),
        );

        let encoded = MyTy::encode(data);
        let encoded_params = MyTy::encode_params(data);

        let expected = hex!(
            "
    		0000000000000000000000001111111111111111111111111111111111111111
    		0000000000000000000000002222222222222222222222222222222222222222
    		0000000000000000000000000000000000000000000000000000000000000001
    		0000000000000000000000000000000000000000000000000000000000000000
    		0000000000000000000000003333333333333333333333333333333333333333
    		0000000000000000000000004444444444444444444444444444444444444444
    	"
        )
        .to_vec();

        // a static FixedSeq should NEVER indirect
        assert_eq!(encoded, expected);
        assert_eq!(encoded_params, expected);
    }

    #[test]
    fn encode_dynamic_tuple_with_nested_static_tuples() {
        type MyTy = (
            ((sol_type::Bool, sol_type::Uint<16>),),
            sol_type::Array<sol_type::Uint<16>>,
        );

        let data = (((false, 0x777),), vec![0x42, 0x1337]);

        let encoded = MyTy::encode_single(data.clone());
        let encoded_params = MyTy::encode_params(data);

        let expected = hex!(
            "
    		0000000000000000000000000000000000000000000000000000000000000020
    		0000000000000000000000000000000000000000000000000000000000000000
    		0000000000000000000000000000000000000000000000000000000000000777
    		0000000000000000000000000000000000000000000000000000000000000060
    		0000000000000000000000000000000000000000000000000000000000000002
    		0000000000000000000000000000000000000000000000000000000000000042
    		0000000000000000000000000000000000000000000000000000000000001337
    	"
        )
        .to_vec();
        // a dynamic FixedSeq at top level should start with indirection
        // when not param encoded
        assert_eq!(encoded, expected);
        assert_ne!(encoded_params, expected);
        assert_eq!(encoded_params.len() + 32, encoded.len());
    }
}