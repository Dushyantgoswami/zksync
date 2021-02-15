use ethabi::{ParamType, Token};

use crate::{contract::default::get_rollup_ops_from_data, rollup_ops::RollupOpsBlock};
use zksync_types::{AccountId, BlockNumber};

fn decode_commitment_parameters(input_data: Vec<u8>) -> anyhow::Result<Vec<Token>> {
    let commit_operation = ParamType::Tuple(vec![
        Box::new(ParamType::FixedBytes(32)), // bytes32 encoded_root,
        Box::new(ParamType::Bytes),          // bytes calldata _publicData,
        Box::new(ParamType::Uint(256)),      // uint64 _timestamp,
        Box::new(ParamType::Array(Box::new(ParamType::Tuple(vec![
            Box::new(ParamType::Bytes),    // bytes eht_witness
            Box::new(ParamType::Uint(32)), //uint32 public_data_offset
        ])))),
        Box::new(ParamType::Uint(32)), // uint32 _blockNumber,
        Box::new(ParamType::Uint(32)), // uint32 _feeAccount,
    ]);
    let stored_block = ParamType::Tuple(vec![
        Box::new(ParamType::Uint(32)),       // uint32 _block_number
        Box::new(ParamType::Uint(64)),       // uint32 _number_of_processed_prior_ops
        Box::new(ParamType::FixedBytes(32)), //bytes32  processable_ops_hash
        Box::new(ParamType::Uint(256)),      // uint256 timestamp
        Box::new(ParamType::FixedBytes(32)), // bytes32 eth_encoded_root
        Box::new(ParamType::FixedBytes(32)), // commitment
    ]);
    ethabi::decode(
        vec![stored_block, ParamType::Array(Box::new(commit_operation))].as_slice(),
        input_data.as_slice(),
    )
    .map_err(|_| {
        anyhow::Error::from(Box::new(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "can't get decoded parameters from commitment transaction",
        )))
    })
}

pub fn rollup_ops_blocks_from_bytes(data: Vec<u8>) -> anyhow::Result<Vec<RollupOpsBlock>> {
    let fee_account_argument_id = 5;
    let public_data_argument_id = 1;

    let decoded_commitment_parameters = decode_commitment_parameters(data)?;
    assert_eq!(decoded_commitment_parameters.len(), 2);

    if let (ethabi::Token::Tuple(block), ethabi::Token::Array(operations)) = (
        &decoded_commitment_parameters[0],
        &decoded_commitment_parameters[1],
    ) {
        let mut blocks = vec![];
        if let ethabi::Token::Uint(block_num) = block[0] {
            for operation in operations {
                if let ethabi::Token::Tuple(operation) = operation {
                    if let (ethabi::Token::Uint(fee_acc), ethabi::Token::Bytes(public_data)) = (
                        &operation[fee_account_argument_id],
                        &operation[public_data_argument_id],
                    ) {
                        let ops = get_rollup_ops_from_data(public_data.as_slice())?;
                        blocks.push(RollupOpsBlock {
                            block_num: BlockNumber(block_num.as_u32()),
                            ops,
                            fee_account: AccountId(fee_acc.as_u32()),
                        })
                    } else {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::NotFound,
                            "can't parse operation parameters",
                        )
                        .into());
                    }
                }
            }
        } else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "can't parse block parameters",
            )
            .into());
        }
        Ok(blocks)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "can't parse commitment parameters",
        )
        .into())
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_decode_commitment() {
        let input_data = hex::decode(
            "45269298000000000000000000000000000000000000000000\
            00000000000000000000180000000000000000000000000000\
            000000000000000000000000000000000001c5d2460186f723\
            3c927e7db2dcc703c0e500b653ca82273b7bfad8045d85a470\
            00000000000000000000000000000000000000000000000000\
            00000060180bd21ebc71244dfd0ec72156cabe55ae2e5dd35e\
            1b0a1cffe0b52a158f27c1dd34314cebb54dbafb6885b8628c\
            a09d8f4992f4efd7f04e2dda0121896e88a5158f8100000000\
            00000000000000000000000000000000000000000000000000\
            0000e000000000000000000000000000000000000000000000\
            00000000000000000001000000000000000000000000000000\
            000000000000000000000000000000002026bb57dafd75ff97\
            f3c664c511c5e334f0266c6bd0e29e9a69f5c36152fef48100\
            00000000000000000000000000000000000000000000000000\
            0000000000c000000000000000000000000000000000000000\
            00000000000000000060183511000000000000000000000000\
            00000000000000000000000000000000000001400000000000\
            00000000000000000000000000000000000000000000000000\
            00190000000000000000000000000000000000000000000000\
            00000000000000000000000000000000000000000000000000\
            0000000000000000000000000000005a010000000e00000000\
            00000000006c6b935b8bbd4000001e65c448e0486449a0b446\
            bc9a340b933237f6e000000000000000000000000000000000\
            00000000000000000000000000000000000000000000000000\
            00000000000000000000000000000000000000000000000000\
            00000000000000000000000000000000000001000000000000\
            00000000000000000000000000000000000000000000000000\
            20000000000000000000000000000000000000000000000000\
            00000000000000400000000000000000000000000000000000\
            00000000000000000000000000000000000000000000000000\
            00000000000000000000000000000000000000000000",
        )
        .expect("Failed to decode commit tx data");
        let blocks = rollup_ops_blocks_from_bytes(input_data[4..].to_vec()).unwrap();
        assert_eq!(blocks.len(), 1);
        let block = blocks[0].clone();
        assert_eq!(block.block_num, BlockNumber(24));
        assert_eq!(block.fee_account, AccountId(0));
        assert_eq!(block.ops.len(), 5);
    }
}
