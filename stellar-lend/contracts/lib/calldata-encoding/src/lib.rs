#![no_std]

use soroban_sdk::{contracttype, Bytes, Env, Vec};

#[contracttype]
pub struct EncodedOperation {
    pub op_type: u8,
    pub data: Bytes,
}

#[contracttype]
pub struct CompressedBatch {
    pub shared_params: Bytes,
    pub operations: Vec<EncodedOperation>,
    pub compression_ratio: u32,
}

pub struct CalldataEncoder;

impl CalldataEncoder {
    pub fn encode_varint(value: u64) -> Vec<u8> {
        let mut result = Vec::new();
        let mut v = value;

        while v >= 128 {
            let byte = ((v & 0x7F) as u8) | 0x80;
            result.push(byte);
            v >>= 7;
        }
        result.push((v & 0x7F) as u8);

        result
    }

    pub fn decode_varint(data: &[u8], offset: &mut usize) -> Result<u64, &'static str> {
        let mut result = 0u64;
        let mut shift = 0;

        loop {
            if *offset >= data.len() {
                return Err("Unexpected end of data");
            }

            let byte = data[*offset] as u64;
            *offset += 1;

            result |= (byte & 0x7F) << shift;

            if byte < 128 {
                break;
            }

            shift += 7;
            if shift >= 64 {
                return Err("Varint overflow");
            }
        }

        Ok(result)
    }

    pub fn encode_delta(current: i128, previous: i128) -> Vec<u8> {
        let delta = current - previous;
        Self::encode_varint(delta as u64)
    }

    pub fn encode_bitmask(values: &[bool]) -> u8 {
        let mut mask = 0u8;
        for (i, &val) in values.iter().enumerate().take(8) {
            if val {
                mask |= 1 << i;
            }
        }
        mask
    }

    pub fn decode_bitmask(mask: u8) -> Vec<bool> {
        let mut result = Vec::new();
        for i in 0..8 {
            result.push((mask & (1 << i)) != 0);
        }
        result
    }

    pub fn compress_batch(
        _env: &Env,
        shared_asset: &[u8],
        shared_user: &[u8],
        operations: Vec<EncodedOperation>,
    ) -> CompressedBatch {
        let mut shared_params = Vec::new();

        if !shared_asset.is_empty() {
            shared_params.extend_from_slice(shared_asset);
        }
        if !shared_user.is_empty() {
            shared_params.extend_from_slice(shared_user);
        }

        let original_size = operations.len() * 256;
        let compressed_size = shared_params.len() + (operations.len() * 64);

        let compression_ratio = if compressed_size > 0 {
            ((original_size - compressed_size) * 100) as u32 / original_size as u32
        } else {
            0
        };

        CompressedBatch {
            shared_params: Bytes::from_slice(_env, &shared_params),
            operations,
            compression_ratio,
        }
    }

    pub fn estimate_gas_savings(original_size: u32, compressed_size: u32) -> u32 {
        if original_size == 0 {
            return 0;
        }

        let reduction_percent = ((original_size - compressed_size) * 100) / original_size;
        (reduction_percent * 21000) / 100
    }
}

pub mod tests {
    use super::*;

    #[test]
    fn test_varint_encoding() {
        let encoded = CalldataEncoder::encode_varint(127);
        assert_eq!(encoded.len(), 1);

        let encoded = CalldataEncoder::encode_varint(128);
        assert_eq!(encoded.len(), 2);
    }

    #[test]
    fn test_bitmask_encoding() {
        let values = vec![true, false, true, false, false, false, false, false];
        let mask = CalldataEncoder::encode_bitmask(&values);
        let decoded = CalldataEncoder::decode_bitmask(mask);

        for (i, &expected) in values.iter().enumerate() {
            assert_eq!(decoded[i], expected);
        }
    }
}
