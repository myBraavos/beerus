use eyre::Result;
use starknet_api::block_hash::block_hash_calculator::{
    calculate_block_commitments, calculate_block_hash,
};

use crate::gen::{BlockWithReceipts, Felt, StateUpdate};

pub fn validate_block_hash(
    block: &BlockWithReceipts,
    state_update: &StateUpdate,
    block_hash: &Felt,
) -> Result<()> {
    let block_header: starknet_api::block::BlockHeaderWithoutHash =
        block.block_header.clone().try_into()?;

    let transactions_data = block.block_body_with_receipts.transactions.clone().into_iter().map(|transaction_and_receipt| {
        transaction_and_receipt.try_into()
    }).collect::<Result<Vec<starknet_api::block_hash::block_hash_calculator::TransactionHashingData>, crate::exe::err::Error>>()?;

    // then calculate block commitments
    let block_commitments = calculate_block_commitments(
        &transactions_data,
        &state_update.state_diff.clone().try_into()?,
        block_header.l1_da_mode,
        &block_header.starknet_version,
    );

    // then calculate block hash
    let calculated_block_hash =
        calculate_block_hash(block_header, block_commitments)?;
    tracing::debug!(calculated_block_hash=?calculated_block_hash, "calculated block hash");

    // it should match the provided hash
    if calculated_block_hash.0
        != starknet_api::hash::StarkHash::from_hex_unchecked(
            block_hash.as_ref(),
        )
    {
        eyre::bail!("Block hash mismatch: expected {block_hash:?} but got {calculated_block_hash:?}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gen::{
        Address, BlockBodyWithReceipts, BlockHash, BlockHeader,
        BlockHeaderL1DaMode, BlockHeaderTimestamp, BlockNumber, BlockStatus,
        CommonReceiptProperties, ContractStorageDiffItem, DaMode, Event,
        EventContent, ExecutionResourcesDataAvailability, FeePayment,
        InvokeTxn, InvokeTxnReceipt, InvokeTxnReceiptType, InvokeTxnV3,
        InvokeTxnV3Type, InvokeTxnV3Version, NonceUpdate, PriceUnit,
        ResourceBounds, ResourceBoundsMapping, ResourcePrice,
        ResultCommonReceiptProperties, StateDiff, StorageDiffItem,
        SuccessfulCommonReceiptProperties,
        SuccessfulCommonReceiptPropertiesExecutionStatus,
        TransactionAndReceipt, Txn, TxnFinalityStatus, TxnHash, TxnReceipt,
        U128, U64,
    };

    fn create_felt(value: &str) -> Felt {
        Felt::try_new(value).expect("Failed to create Felt")
    }

    fn create_test_block_with_receipts() -> BlockWithReceipts {
        let tx1 = Txn::InvokeTxn(InvokeTxn::InvokeTxnV3(InvokeTxnV3{
            calldata : vec ![
                create_felt("0x1"),
                create_felt("0x1a9fda7208cd4743f3d0ef37e09f633934cf66325691ecadd5b9fa246d4252d"),
                create_felt("0x34cc13b274446654ca3233ed2c1620d4c5d1d32fd20b47146a3371064bdc57d"),
                create_felt("0x3b"),
                create_felt("0x414e595f43414c4c4552"),
                create_felt("0x19a81d5bba6"),
                create_felt("0x6916150b"),
                create_felt("0x6917e9cb"),
                create_felt("0x4"),
                create_felt("0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8"),
                create_felt("0x219209e083275171774dab1df80982e9df2096516f06319c5c6d71ae0a8480c"),
                create_felt("0x3"),
                create_felt("0xd8d6dfec4d33bfb6895de9f3852143a17c6f92fd2a21da3d6924d34870160"),
                create_felt("0x1304480ba"),
                create_felt("0x0"),
                create_felt("0xd8d6dfec4d33bfb6895de9f3852143a17c6f92fd2a21da3d6924d34870160"),
                create_felt("0x39b9c84d6a72745116ecdbd7f122af6d51a7183b6e764d621583713bcceb8cd"),
                create_felt("0xf"),
                create_felt("0x4dc4f0ca6ea4961e4c8373265bfd5317678f4fe374d76f3fd7135f57763bf28"),
                create_felt("0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7"),
                create_felt("0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8"),
                create_felt("0x1a9fda7208cd4743f3d0ef37e09f633934cf66325691ecadd5b9fa246d4252d"),
                create_felt("0x0"),
                create_felt("0x0"),
                create_felt("0x0"),
                create_felt("0x0"),
                create_felt("0x0"),
                create_felt("0x1"),
                create_felt("0x0"),
                create_felt("0x0"),
                create_felt("0x0"),
                create_felt("0x0"),
                create_felt("0x0"),
                create_felt("0xd8d6dfec4d33bfb6895de9f3852143a17c6f92fd2a21da3d6924d34870160"),
                create_felt("0x1e4ab451cc39af5db2c89121a0d0a7189a7e91028fb5cd345953b6a04cb85e3"),
                create_felt("0x11"),
                create_felt("0x4dc4f0ca6ea4961e4c8373265bfd5317678f4fe374d76f3fd7135f57763bf28"),
                create_felt("0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7"),
                create_felt("0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8"),
                create_felt("0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7"),
                create_felt("0x0"),
                create_felt("0x1a9fda7208cd4743f3d0ef37e09f633934cf66325691ecadd5b9fa246d4252d"),
                create_felt("0x4e06e04b8d624d039aa1c3ca8e0aa9e21dc1ccba1d88d0d650837159e0ee054"),
                create_felt("0x0"),
                create_felt("0x0"),
                create_felt("0x4f5b8a25426cb346"),
                create_felt("0x0"),
                create_felt("0x0"),
                create_felt("0x0"),
                create_felt("0x0"),
                create_felt("0x0"),
                create_felt("0x0"),
                create_felt("0x0"),
                create_felt("0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8"),
                create_felt("0x219209e083275171774dab1df80982e9df2096516f06319c5c6d71ae0a8480c"),
                create_felt("0x3"),
                create_felt("0xd8d6dfec4d33bfb6895de9f3852143a17c6f92fd2a21da3d6924d34870160"),
                create_felt("0x0"),
                create_felt("0x0"),
                create_felt("0x3"),
                create_felt("0x1"),
                create_felt("0x7804dbc2e1d29a9157ddb1190e87ba9e0f0a7109a8441d030db162ba0f45f07"),
                create_felt("0x5916ba2c028dcad1dc44875669f51c7da05058846fbf7e896b28b0c98448ae7"),
            ],
            resource_bounds : ResourceBoundsMapping{
                l1_gas : ResourceBounds{
                    max_amount : U64::try_new("0x0").unwrap(),
                    max_price_per_unit : U128::try_new("0x2a836a35414b").unwrap(),
                },
                l2_gas : ResourceBounds{
                    max_amount : U64::try_new("0x23f4a3ec").unwrap(),
                    max_price_per_unit : U128::try_new("0x10c388d00").unwrap(),
                },
                l1_data_gas : ResourceBounds{
                    max_amount : U64::try_new("0x0").unwrap(),
                    max_price_per_unit : U128::try_new("0x0").unwrap(),
                },
            },
            tip : U64::try_new("0x55d4a80").unwrap(),
            paymaster_data : vec ![],
            account_deployment_data : vec ![],
            nonce_data_availability_mode : DaMode::L1,
            fee_data_availability_mode : DaMode::L1,
            r#type : InvokeTxnV3Type::Invoke,
            sender_address : Address(create_felt("0x761a5d53b8133d70140845fbc522f63adf80f3b9ed979d2eb7f772f76c1b206")),
            signature : vec ![
                create_felt("0x1"),
                create_felt("0x7cbf6a205d90633d62859ff7b875a660d4413957f666225f6dee870eb611bec"),
                create_felt("0x59c7669a89d615142695eb28db5b46ad0f08556ef9f38e2949ecfc7197e7a69"),
            ],
            version : InvokeTxnV3Version::V0x3,
            nonce : create_felt("0x12b78"),
        }));
        let receipt1 = TxnReceipt::InvokeTxnReceipt(InvokeTxnReceipt{
            r#type : InvokeTxnReceiptType::Invoke,
            common_receipt_properties : CommonReceiptProperties{
                actual_fee : FeePayment{
                    amount : create_felt("0xa78e5fe3145f900"),
                    unit : PriceUnit::Fri,
                },
                events : vec ![
                    Event{
                        from_address :
                            Address(create_felt("0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0x134692b230b9e1ffa39098904722134159652b09c5bc41d88d6698779d228ff"),
                            ],
                            data : vec ![
                                create_felt("0x1a9fda7208cd4743f3d0ef37e09f633934cf66325691ecadd5b9fa246d4252d"),
                                create_felt("0xd8d6dfec4d33bfb6895de9f3852143a17c6f92fd2a21da3d6924d34870160"),
                                create_felt("0x1304480ba"),
                                create_felt("0x0"),
                            ]
                        }
                    },
                    Event{
                        from_address : Address(create_felt("0xd8d6dfec4d33bfb6895de9f3852143a17c6f92fd2a21da3d6924d34870160")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0xe623beb06d0cfbe7f7877cf06290a77c803ca8fde4b54a68b241607c7cc8cc"),
                                create_felt("0x4dc4f0ca6ea4961e4c8373265bfd5317678f4fe374d76f3fd7135f57763bf28"),
                                create_felt("0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7"),
                                create_felt("0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8"),
                            ],
                            data : vec ![
                                create_felt("0x98bc72f9079d4874fa"),
                                create_felt("0x0"),
                                create_felt("0x68c1eb9513caf8eb3a"),
                                create_felt("0x0"),
                                create_felt("0x2fdf999bc3fee7986b"),
                                create_felt("0x0"),
                                create_felt("0xde0b6b3a7640000"),
                                create_felt("0x0"),
                                create_felt("0x8ac7230489e80000"),
                                create_felt("0x0"),
                                create_felt("0xde0b6b3a7640000"),
                                create_felt("0x0"),
                                create_felt("0x0"),
                                create_felt("0x6916ff6d"),
                                create_felt("0xdf4c1554804d093"),
                                create_felt("0x0"),
                                create_felt("0x6b49d200"),
                                create_felt("0x0"),
                                create_felt("0x0"),
                                create_felt("0x0"),
                                create_felt("0x22ce28e27879a4e79e7d"),
                                create_felt("0x0"),
                                create_felt("0x1ab262a270845729aeda"),
                                create_felt("0x0"),
                                create_felt("0x8f7106423"),
                                create_felt("0x0"),
                                create_felt("0xd2f13f7789f0000"),
                                create_felt("0x0"),
                                create_felt("0x8ac7230489e80000"),
                                create_felt("0x0"),
                                create_felt("0xf4240"),
                                create_felt("0x0"),
                                create_felt("0x0"),
                                create_felt("0x6916ff6d"),
                                create_felt("0xe58d66873802a50"),
                                create_felt("0x0"),
                                create_felt("0x14b52bd5d"),
                                create_felt("0x0"),
                                create_felt("0x0"),
                                create_felt("0x0"),
                                create_felt("0xab637d7ab1be728400"),
                                create_felt("0x0"),
                                create_felt("0x1"),
                                create_felt("0xddf1c850898d000"),
                                create_felt("0x0"),
                                create_felt("0x1"),
                            ]
                        }
                    },
                    Event{
                        from_address : Address(create_felt("0xd8d6dfec4d33bfb6895de9f3852143a17c6f92fd2a21da3d6924d34870160")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0x3dfe6670b0f4e60f951b8a326e7467613b2470d81881ba2deb540262824f1e"),
                                create_felt("0x4dc4f0ca6ea4961e4c8373265bfd5317678f4fe374d76f3fd7135f57763bf28"),
                                create_felt("0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7"),
                                create_felt("0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8"),
                                create_felt("0x1a9fda7208cd4743f3d0ef37e09f633934cf66325691ecadd5b9fa246d4252d"),
                            ],
                            data : vec ![
                                create_felt("0x0"),
                                create_felt("0x0"),
                                create_felt("0x0"),
                                create_felt("0x0"),
                                create_felt("0x0"),
                                create_felt("0x0"),
                                create_felt("0x12a4d3535"),
                                create_felt("0x0"),
                                create_felt("0x1"),
                                create_felt("0x1066e326d4163d1cea1"),
                                create_felt("0x0"),
                                create_felt("0x1"),
                            ]
                        }
                    },
                    Event{
                        from_address :
                            Address(create_felt("0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0x134692b230b9e1ffa39098904722134159652b09c5bc41d88d6698779d228ff"),
                            ],
                            data : vec ![
                                create_felt("0x1a9fda7208cd4743f3d0ef37e09f633934cf66325691ecadd5b9fa246d4252d"),
                                create_felt("0xd8d6dfec4d33bfb6895de9f3852143a17c6f92fd2a21da3d6924d34870160"),
                                create_felt("0x5f74b85"),
                                create_felt("0x0"),
                            ]
                        }
                    },
                    Event{
                        from_address :
                            Address(create_felt("0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0x99cd8bde557814842a3121e8ddfd433a539b8c9f14bf31ebf108d12e6196e9"),
                            ],
                            data : vec ![
                                create_felt("0x1a9fda7208cd4743f3d0ef37e09f633934cf66325691ecadd5b9fa246d4252d"),
                                create_felt("0xd8d6dfec4d33bfb6895de9f3852143a17c6f92fd2a21da3d6924d34870160"),
                                create_felt("0x12a4d3535"),
                                create_felt("0x0"),
                            ]
                        }
                    },
                    Event{
                        from_address :
                            Address(create_felt("0x391bd9b58695b952aa15cffce50ba4650c954105df405ca8fc976ad7a65d646")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0x99cd8bde557814842a3121e8ddfd433a539b8c9f14bf31ebf108d12e6196e9"),
                                create_felt("0x0"),
                                create_felt("0x1a9fda7208cd4743f3d0ef37e09f633934cf66325691ecadd5b9fa246d4252d"),
                            ],
                            data : vec ![
                                create_felt("0x4f5b8a25426cb346"),
                                create_felt("0x0"),
                            ]
                        }
                    },
                    Event{
                        from_address : Address(create_felt("0xd8d6dfec4d33bfb6895de9f3852143a17c6f92fd2a21da3d6924d34870160")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0x193750dd24b142c85cff259f0357a58cfde97f77af04b0ed5bd7132aedbd5c6"),
                                create_felt("0x4dc4f0ca6ea4961e4c8373265bfd5317678f4fe374d76f3fd7135f57763bf28"),
                                create_felt("0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7"),
                                create_felt("0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8"),
                                create_felt("0x49d36570d4e46f48e99674bd3fcc84644ddd6b96f7c741b1562b82f9e004dc7"),
                                create_felt("0x0"),
                                create_felt("0x1a9fda7208cd4743f3d0ef37e09f633934cf66325691ecadd5b9fa246d4252d"),
                                create_felt("0x4e06e04b8d624d039aa1c3ca8e0aa9e21dc1ccba1d88d0d650837159e0ee054"),
                            ],
                            data : vec ![
                                create_felt("0x4f9c26d6fa808917"),
                                create_felt("0x0"),
                                create_felt("0x1"),
                                create_felt("0x4f5b8a25426cb346"),
                                create_felt("0x0"),
                                create_felt("0x1"),
                                create_felt("0x0"),
                                create_felt("0x0"),
                                create_felt("0x0"),
                                create_felt("0x0"),
                                create_felt("0x0"),
                                create_felt("0x0"),
                            ]
                        }
                    },
                    Event{
                        from_address :
                            Address(create_felt("0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0x134692b230b9e1ffa39098904722134159652b09c5bc41d88d6698779d228ff"),
                            ],
                            data : vec ![
                                create_felt("0x1a9fda7208cd4743f3d0ef37e09f633934cf66325691ecadd5b9fa246d4252d"),
                                create_felt("0xd8d6dfec4d33bfb6895de9f3852143a17c6f92fd2a21da3d6924d34870160"),
                                create_felt("0x0"),
                                create_felt("0x0"),
                            ]
                        }
                    },
                    Event{
                        from_address :
                            Address(create_felt("0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0x99cd8bde557814842a3121e8ddfd433a539b8c9f14bf31ebf108d12e6196e9"),
                            ],
                            data : vec ![
                                create_felt("0x761a5d53b8133d70140845fbc522f63adf80f3b9ed979d2eb7f772f76c1b206"),
                                create_felt("0x1176a1bd84444c89232ec27754698e5d2e7e1a7f1539f12027f28b23ec9f3d8"),
                                create_felt("0xa78e5fe3145f900"),
                                create_felt("0x0"),
                            ]
                        }
                    },
                ],
                execution_resources : ExecutionResourcesDataAvailability{
                    l1_data_gas : 1280,
                    l1_gas : 0,
                    l2_gas : 244209002,
                },
                finality_status : TxnFinalityStatus::AcceptedOnL2,
                messages_sent : vec ![],
                transaction_hash : TxnHash(create_felt("0x282c4db84f834fc23649c8ad0b2fd4af9174921d0d2728033fd3f36bf7dd198")),
                result_common_receipt_properties :
                    ResultCommonReceiptProperties::SuccessfulCommonReceiptProperties(SuccessfulCommonReceiptProperties{
                        execution_status : SuccessfulCommonReceiptPropertiesExecutionStatus::Succeeded,
                    }, ),
            },
        });
        let tx2 = Txn::InvokeTxn(InvokeTxn::InvokeTxnV3(InvokeTxnV3{
            calldata : vec ![
                create_felt("0x3"),
                create_felt("0x51fea4450da9d6aee758bdeba88b2f665bcbf549d2c61421aa724e9ac0ced8f"),
                create_felt("0x32d688805385a47f61c040502e37cfe653e66ff4cb9faf5a0a7b447527a9f01"),
                create_felt("0x6"),
                create_felt("0x63275168425ac2fb3b20e2b88086796ffbbf3021aed8813c3ac383e812fb895"),
                create_felt("0x4a1f93a0eb8b3517142d57d25e759f946a068f6294315691ce6aa1b5d2399d2"),
                create_felt("0x191a47a1d7fd652fee299c5aa22c232219c78c0896be7f092e6c25b7744dc3b"),
                create_felt("0x6c8723780c0f5d3a4ab4ca0240cbf77b587104597724dcc06d8a4f29461a7f6"),
                create_felt("0x57145c71ba1535eebff71112c627ec54a64cbace97087c78bd892b1365379f9"),
                create_felt("0x14ddd724a7c58e14ed311b385fd0ab5748bb42dbc0f4ceeb5198b71df9bff7e"),
                create_felt("0x1634a1121191f06618c4eaa490550b897e4f3d6ba07bb2f05956c8ae430a503"),
                create_felt("0x3dbc508ba4afd040c8dc4ff8a61113a7bcaf5eae88a6ba27b3c50578b3587e3"),
                create_felt("0x2b"),
                create_felt("0x414e595f43414c4c4552"),
                create_felt("0x5f9d8a2b0c137da8ad73cb724f5d42be7166e66aec9ecd7418dcdccbf9ddaae"),
                create_felt("0x10000000000000"),
                create_felt("0x0"),
                create_felt("0x691701c7"),
                create_felt("0x2"),
                create_felt("0x51fea4450da9d6aee758bdeba88b2f665bcbf549d2c61421aa724e9ac0ced8f"),
                create_felt("0x12a5a2e008479001f8f1a5f6c61ab6536d5ce46571fcdc0c9300dca0a9e532f"),
                create_felt("0x3"),
                create_felt("0x6f7c4350d6d5ee926b3ac4fa0c9c351055456e75c92227468d84232fc493a9c"),
                create_felt("0x1"),
                create_felt("0x43a71862489cf1dc5cbe0fcb31197a85283426aeaf91c8aac8fc73a45833174"),
                create_felt("0x6f7c4350d6d5ee926b3ac4fa0c9c351055456e75c92227468d84232fc493a9c"),
                create_felt("0x1f64d317ff277789ba74de95db50418ab0fa47c09241400b7379b50d6334c3a"),
                create_felt("0x2"),
                create_felt("0x15c1f"),
                create_felt("0x0"),
                create_felt("0x19"),
                create_felt("0x73657373696f6e2d746f6b656e"),
                create_felt("0x69200295"),
                create_felt("0x77696c64636172642d706f6c696379"),
                create_felt("0x0"),
                create_felt("0x622c02cb37cf7195e5fe3612f39f766590c1daad841c77f65c082349941d304"),
                create_felt("0x0"),
                create_felt("0x1"),
                create_felt("0x8"),
                create_felt("0x1"),
                create_felt("0x3"),
                create_felt("0xe3cc9db7c856b9f05920fc42c6914f76e6958f68"),
                create_felt("0x2b93ffd1da17e4e7693a36ec27da9a09"),
                create_felt("0x164b3f3bd1a7e0083f9ca766ee65407b"),
                create_felt("0xfdf4ba8ffc0acefdac213d4d426cbb18"),
                create_felt("0xfa5e4cb0fc589fda91358458ac10ae5"),
                create_felt("0x1"),
                create_felt("0x0"),
                create_felt("0x11e7648f916c3f7090989ebde03c908574e8b533c794f1bbfaf0a6ae832c506"),
                create_felt("0x255414c37616db5c955f9b900c0781b9965585c085a103018fe5d65b0317bb"),
                create_felt("0x47cb1e98f626a14379c0e53f9656b263f65ec335ccb3009488784a323775698"),
                create_felt("0x0"),
                create_felt("0x1e6a6f52e47fe42e024287b729bc47e58019fcc7e1cc8b141bb8d669b779b49"),
                create_felt("0x184ef6d2b0e5950260b88f2e0f13db1d6faff96110de1b9a83af34b1d820a00"),
                create_felt("0x55541707bfc96985b9c705bf64b6cfda77bc866978a071a885b963301e2c802"),
                create_felt("0x0"),
                create_felt("0x51fea4450da9d6aee758bdeba88b2f665bcbf549d2c61421aa724e9ac0ced8f"),
                create_felt("0x112b534028a89c7062d4f90ab082e3bb5a7c63c1af793c03bd040a66dc50839"),
                create_felt("0x1"),
                create_felt("0x63275168425ac2fb3b20e2b88086796ffbbf3021aed8813c3ac383e812fb895"),
            ],
            resource_bounds : ResourceBoundsMapping{
                l1_gas : ResourceBounds{
                    max_amount : U64::try_new("0x0").unwrap(),
                    max_price_per_unit : U128::try_new("0x46e3fd10c176").unwrap(),
                },
                l2_gas : ResourceBounds{
                    max_amount : U64::try_new("0x48a92c0").unwrap(),
                    max_price_per_unit : U128::try_new("0x1bf08eb00").unwrap(),
                },
                l1_data_gas : ResourceBounds{
                    max_amount : U64::try_new("0x0").unwrap(),
                    max_price_per_unit : U128::try_new("0x0").unwrap(),
                },
            },
            tip : U64::try_new("0x1").unwrap(),
            paymaster_data : vec ![],
            account_deployment_data : vec ![],
            nonce_data_availability_mode : DaMode::L1,
            fee_data_availability_mode : DaMode::L1,
            r#type : InvokeTxnV3Type::Invoke,
            sender_address : Address(create_felt("0x285cbc386563791682cd06f40e2c7fb856ee3cfa0ef7727bd7e7b410b7e71d1")),
            signature : vec ![
                create_felt("0x4b5319f59ddc5684f2c462d70adccff5b1d89f9353e98694d9f5ca5afead51c"),
                create_felt("0x6f90075a399644f30dafeabf0ce38c88ab445b5343432a7e25cbec30f87c6fe"),
            ],
            version : InvokeTxnV3Version::V0x3,
            nonce : create_felt("0x26ddd"),
        }));
        let receipt2 = TxnReceipt::InvokeTxnReceipt(InvokeTxnReceipt{
            r#type : InvokeTxnReceiptType::Invoke,
            common_receipt_properties : CommonReceiptProperties{
                actual_fee : FeePayment{
                    amount : create_felt("0x1267a7cfecd4d80"),
                    unit : PriceUnit::Fri,
                },
                events : vec ![
                    Event{
                        from_address :
                            Address(create_felt("0x51fea4450da9d6aee758bdeba88b2f665bcbf549d2c61421aa724e9ac0ced8f")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0x178cb81b0d146e06862f5058263b5f4aabc132c03456a207484c6d425282af8"),
                                create_felt("0x63275168425ac2fb3b20e2b88086796ffbbf3021aed8813c3ac383e812fb895"),
                            ],
                            data : vec ![
                                create_felt("0x4a1f93a0eb8b3517142d57d25e759f946a068f6294315691ce6aa1b5d2399d2"),
                                create_felt("0x191a47a1d7fd652fee299c5aa22c232219c78c0896be7f092e6c25b7744dc3b"),
                                create_felt("0x6c8723780c0f5d3a4ab4ca0240cbf77b587104597724dcc06d8a4f29461a7f6"),
                                create_felt("0x57145c71ba1535eebff71112c627ec54a64cbace97087c78bd892b1365379f9"),
                                create_felt("0x14ddd724a7c58e14ed311b385fd0ab5748bb42dbc0f4ceeb5198b71df9bff7e"),
                            ]
                        }
                    },
                    Event{
                        from_address :
                            Address(create_felt("0x2ef591697f0fd9adc0ba9dbe0ca04dabad80cf95f08ba02e435d9cb6698a28a")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0x1c93f6e4703ae90f75338f29bffbe9c1662200cee981f49afeec26e892debcd"),
                                create_felt("0x53f716bb3e8a7730e612809aa0416ce2cbf4aff462e4784d5f2f29d5e96605c"),
                                create_felt("0x6f7c4350d6d5ee926b3ac4fa0c9c351055456e75c92227468d84232fc493a9c"),
                            ],
                            data : vec ![
                                create_felt("0x1"),
                                create_felt("0x15c1f"),
                                create_felt("0x8"),
                                create_felt("0xb4"),
                                create_felt("0x4"),
                                create_felt("0x17"),
                                create_felt("0x0"),
                                create_felt("0x3f"),
                                create_felt("0x2"),
                                create_felt("0x1"),
                                create_felt("0x14"),
                            ]
                        }
                    },
                    Event{
                        from_address :
                            Address(create_felt("0x2ef591697f0fd9adc0ba9dbe0ca04dabad80cf95f08ba02e435d9cb6698a28a")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0x1c93f6e4703ae90f75338f29bffbe9c1662200cee981f49afeec26e892debcd"),
                                create_felt("0x53f716bb3e8a7730e612809aa0416ce2cbf4aff462e4784d5f2f29d5e96605c"),
                                create_felt("0x6f7c4350d6d5ee926b3ac4fa0c9c351055456e75c92227468d84232fc493a9c"),
                            ],
                            data : vec ![
                                create_felt("0x1"),   create_felt("0x15c1f"), create_felt("0x20"),  create_felt("0xb4"),
                                create_felt("0x0"),   create_felt("0x124"),   create_felt("0x2c8"), create_felt("0x4d"),
                                create_felt("0x0"),   create_felt("0x0"),     create_felt("0xb"),   create_felt("0x12"),
                                create_felt("0x11"),  create_felt("0x2"),     create_felt("0x6"),   create_felt("0xd"),
                                create_felt("0x4c"),  create_felt("0x9"),     create_felt("0x190"), create_felt("0x11"),
                                create_felt("0x101"), create_felt("0x16"),    create_felt("0x84"),  create_felt("0x1b"),
                                create_felt("0x190"), create_felt("0x20"),    create_felt("0x84"),  create_felt("0x25"),
                                create_felt("0x190"), create_felt("0x2"),     create_felt("0x145"), create_felt("0x7"),
                                create_felt("0x161"), create_felt("0xcbf"),   create_felt("0xb4"),
                            ]
                        }
                    },
                    Event{
                        from_address :
                            Address(create_felt("0x2ef591697f0fd9adc0ba9dbe0ca04dabad80cf95f08ba02e435d9cb6698a28a")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0x1a2f334228cee715f1f0f54053bb6b5eac54fa336e0bc1aacf7516decb0471d"),
                                create_felt("0x48f9eedcda02e2ed3ebc286dab3e38e7129b444bdef510b0ebbeecdfc547be0"),
                                create_felt("0x7b0f2bfc489975acf101219091fe1d4fbedbb07f7231866b03565923b6e274d"),
                            ],
                            data : vec ![
                                create_felt("0x1"),
                                create_felt("0x15c1f"),
                                create_felt("0x1"),
                                create_felt("0x2d032fec21e8a0b20950883206d085a02472025630b9e300009a0b2124"),
                            ]
                        }
                    },
                    Event{
                        from_address :
                            Address(create_felt("0x2ef591697f0fd9adc0ba9dbe0ca04dabad80cf95f08ba02e435d9cb6698a28a")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0x1c93f6e4703ae90f75338f29bffbe9c1662200cee981f49afeec26e892debcd"),
                                create_felt("0x3e509804fbdba096142d78c1563c907a80c266c5dfcbda494d1d4e4d13a2215"),
                                create_felt("0x2f1c516fa4d2c41f2021edc3b46f33326e73755e55982374381150e6d8d12df"),
                            ],
                            data : vec ![
                                create_felt("0x1"),
                                create_felt("0x15c1f"),
                                create_felt("0x1"),
                                create_felt("0x2c8"),
                            ]
                        }
                    },
                    Event{
                        from_address :
                            Address(create_felt("0x36017e69d21d6d8c13e266eabb73ef1f1d02722d86bdcabe5f168f8e549d3cd")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0x290118457a640990dbcdeb696bd7f53f1d7d71d19b7d566efd42da398c908d3"),
                                create_felt("0x15c1f"),
                                create_felt("0x0"),
                            ],
                            data : vec ![]
                        }
                    },
                    Event{
                        from_address :
                            Address(create_felt("0x1634a1121191f06618c4eaa490550b897e4f3d6ba07bb2f05956c8ae430a503")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0x1dcde06aabdbca2f80aa51392b345d7549d7757aa855f7e37f5d335ac8243b1"),
                                create_felt("0x187972568e20e68b23a1dd491f01a50de954ca94c4d9b6c89fb490b4977f5"),
                            ],
                            data : vec ![
                                create_felt("0x2"),
                                create_felt("0x0"),
                                create_felt("0x0"),
                            ]
                        }
                    },
                    Event{
                        from_address :
                            Address(create_felt("0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0x99cd8bde557814842a3121e8ddfd433a539b8c9f14bf31ebf108d12e6196e9"),
                            ],
                            data : vec ![
                                create_felt("0x285cbc386563791682cd06f40e2c7fb856ee3cfa0ef7727bd7e7b410b7e71d1"),
                                create_felt("0x1176a1bd84444c89232ec27754698e5d2e7e1a7f1539f12027f28b23ec9f3d8"),
                                create_felt("0x1267a7cfecd4d80"),
                                create_felt("0x0"),
                            ]
                        }
                    },
                ],
                execution_resources : ExecutionResourcesDataAvailability{
                    l1_data_gas : 384,
                    l1_gas : 0,
                    l2_gas : 27629440,
                },
                finality_status : TxnFinalityStatus::AcceptedOnL2,
                messages_sent : vec ![],
                transaction_hash : TxnHash(create_felt("0x68129c141379fbec384e2526e0aa0478a0ba217070cdddca988280c45006fae")),
                result_common_receipt_properties :
                    ResultCommonReceiptProperties::SuccessfulCommonReceiptProperties(SuccessfulCommonReceiptProperties{
                        execution_status : SuccessfulCommonReceiptPropertiesExecutionStatus::Succeeded,
                    }, ),
            },
        });
        let tx3 = Txn::InvokeTxn(InvokeTxn::InvokeTxnV3(InvokeTxnV3{
            calldata : vec ![
                create_felt("0x3"),
                create_felt("0x51fea4450da9d6aee758bdeba88b2f665bcbf549d2c61421aa724e9ac0ced8f"),
                create_felt("0x32d688805385a47f61c040502e37cfe653e66ff4cb9faf5a0a7b447527a9f01"),
                create_felt("0x6"),
                create_felt("0x68eab3f72028faef9bea029af7fb2920a45e11dba1d3710d39c5d276afe351d"),
                create_felt("0x5298d89dd99be491087be7049b9403504a946e856f77f58b134767efa4bd6a0"),
                create_felt("0x6a62d1cb440eaa713c93aa03e8000fbc8f8da11a6a3fa58c509b0407be7c95a"),
                create_felt("0x15e2bc07587f6693b3015dd8fc9fa482138739501cebcacbe222829f6956c71"),
                create_felt("0x4a28f59241873f58dc711e50ebb52ea4a88f8ad992badb5f3fbd38d15b560be"),
                create_felt("0x5f79eff56676319318486e0e79f97f0f59b22d5b7100ad2ceb96dc0973c370e"),
                create_felt("0x15f125aaeb8429544c8db516ebfcdfd447c6cd2b6c91fbbc2ffa52c18611931"),
                create_felt("0x3dbc508ba4afd040c8dc4ff8a61113a7bcaf5eae88a6ba27b3c50578b3587e3"),
                create_felt("0x2b"),
                create_felt("0x414e595f43414c4c4552"),
                create_felt("0x5bb88b3ba630cb5908e3cafb0beca3690021799e6da01e67b5b0865a23ee040"),
                create_felt("0x20000000000000"),
                create_felt("0x0"),
                create_felt("0x691701bb"),
                create_felt("0x2"),
                create_felt("0x51fea4450da9d6aee758bdeba88b2f665bcbf549d2c61421aa724e9ac0ced8f"),
                create_felt("0x12a5a2e008479001f8f1a5f6c61ab6536d5ce46571fcdc0c9300dca0a9e532f"),
                create_felt("0x3"),
                create_felt("0x6f7c4350d6d5ee926b3ac4fa0c9c351055456e75c92227468d84232fc493a9c"),
                create_felt("0x1"),
                create_felt("0x175b5dfd93e68ee8ae478eb19744c6289f45ea9b621c7732e859e6171a1e4f2"),
                create_felt("0x6f7c4350d6d5ee926b3ac4fa0c9c351055456e75c92227468d84232fc493a9c"),
                create_felt("0x11a7c59c924cc874e20358db055478acbf359f83bbe68547e90cb94dae65a5c"),
                create_felt("0x2"),
                create_felt("0x15c91"),
                create_felt("0x0"),
                create_felt("0x19"),
                create_felt("0x73657373696f6e2d746f6b656e"),
                create_felt("0x691e1d4b"),
                create_felt("0x77696c64636172642d706f6c696379"),
                create_felt("0x0"),
                create_felt("0x5a807679863cd2bace12b1cf89bf402458c2d374ab9a87e475830a3d5691f74"),
                create_felt("0x0"),
                create_felt("0x1"),
                create_felt("0x8"),
                create_felt("0x1"),
                create_felt("0x3"),
                create_felt("0x170b1c0a1c248c713c27c40935e492a304f1d552"),
                create_felt("0xec908a6f1641e324f41e355619981de2"),
                create_felt("0xabfdcd1f0fb76262dfb3d1e0c7e423b9"),
                create_felt("0xef9e229b4182449bd98e20237a44c737"),
                create_felt("0x14d1dc234e3ecfced5987bf52025bc7b"),
                create_felt("0x1"),
                create_felt("0x0"),
                create_felt("0x701884015a7a94a35665c3084df492de5224a133ae1e7d1906871efa69bd1b2"),
                create_felt("0x50350997f9c6d9cb89ead4053c4dac5673cd88755c2a81f811ca24f89740f94"),
                create_felt("0x679e32d6e8eef210d93d02f373d9c4e591bbfbe8a47c049c4766ec709e05cd2"),
                create_felt("0x0"),
                create_felt("0x1e6a6f52e47fe42e024287b729bc47e58019fcc7e1cc8b141bb8d669b779b49"),
                create_felt("0x7b604f42bbde7f6588bdef24c861839c6ca08649f8de85cd5e4cf1a63b9b31b"),
                create_felt("0x4179e0ad96a601a6601268fe9bf91ffdc628a14561ce6bbeea19cb750ad4aee"),
                create_felt("0x0"),
                create_felt("0x51fea4450da9d6aee758bdeba88b2f665bcbf549d2c61421aa724e9ac0ced8f"),
                create_felt("0x112b534028a89c7062d4f90ab082e3bb5a7c63c1af793c03bd040a66dc50839"),
                create_felt("0x1"),
                create_felt("0x68eab3f72028faef9bea029af7fb2920a45e11dba1d3710d39c5d276afe351d"),
            ],
            resource_bounds : ResourceBoundsMapping{
                l1_gas : ResourceBounds{
                    max_amount : U64::try_new("0x0").unwrap(),
                    max_price_per_unit : U128::try_new("0x46e3fd10c176").unwrap(),
                },
                l2_gas : ResourceBounds{
                    max_amount : U64::try_new("0x3d46220").unwrap(),
                    max_price_per_unit : U128::try_new("0x1bf08eb00").unwrap(),
                },
                l1_data_gas : ResourceBounds{
                    max_amount : U64::try_new("0x0").unwrap(),
                    max_price_per_unit : U128::try_new("0x0").unwrap(),
                },
            },
            tip : U64::try_new("0x1").unwrap(),
            paymaster_data : vec ![],
            account_deployment_data : vec ![],
            nonce_data_availability_mode : DaMode::L1,
            fee_data_availability_mode : DaMode::L1,
            r#type : InvokeTxnV3Type::Invoke,
            sender_address : Address(create_felt("0x498631aa7ebcb7be6acaf5990eb41488bef2597967885cc5ca96ccb5493a59e")),
            signature : vec ![
                create_felt("0x3b01effdc57a3c0832794569a7291523c184141f573bc3a1e09dfecf658109d"),
                create_felt("0x72adcc8c9b1b42c33d5fb00c9952980bec8ac0b8e285ae149cca5469d99d983"),
            ],
            version : InvokeTxnV3Version::V0x3,
            nonce : create_felt("0x4b5d5"),
        }));
        let receipt3 = TxnReceipt::InvokeTxnReceipt(InvokeTxnReceipt{
            r#type : InvokeTxnReceiptType::Invoke,
            common_receipt_properties : CommonReceiptProperties{
                actual_fee : FeePayment{
                    amount : create_felt("0x112e0b264e8bdc0"),
                    unit : PriceUnit::Fri,
                },
                events : vec ![
                    Event{
                        from_address :
                            Address(create_felt("0x51fea4450da9d6aee758bdeba88b2f665bcbf549d2c61421aa724e9ac0ced8f")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0x178cb81b0d146e06862f5058263b5f4aabc132c03456a207484c6d425282af8"),
                                create_felt("0x68eab3f72028faef9bea029af7fb2920a45e11dba1d3710d39c5d276afe351d"),
                            ],
                            data : vec ![
                                create_felt("0x5298d89dd99be491087be7049b9403504a946e856f77f58b134767efa4bd6a0"),
                                create_felt("0x6a62d1cb440eaa713c93aa03e8000fbc8f8da11a6a3fa58c509b0407be7c95a"),
                                create_felt("0x15e2bc07587f6693b3015dd8fc9fa482138739501cebcacbe222829f6956c71"),
                                create_felt("0x4a28f59241873f58dc711e50ebb52ea4a88f8ad992badb5f3fbd38d15b560be"),
                                create_felt("0x5f79eff56676319318486e0e79f97f0f59b22d5b7100ad2ceb96dc0973c370e"),
                            ]
                        }
                    },
                    Event{
                        from_address :
                            Address(create_felt("0x2ef591697f0fd9adc0ba9dbe0ca04dabad80cf95f08ba02e435d9cb6698a28a")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0x1c93f6e4703ae90f75338f29bffbe9c1662200cee981f49afeec26e892debcd"),
                                create_felt("0x53f716bb3e8a7730e612809aa0416ce2cbf4aff462e4784d5f2f29d5e96605c"),
                                create_felt("0x6f7c4350d6d5ee926b3ac4fa0c9c351055456e75c92227468d84232fc493a9c"),
                            ],
                            data : vec ![
                                create_felt("0x1"),
                                create_felt("0x15c91"),
                                create_felt("0x3"),
                                create_felt("0x20"),
                                create_felt("0x10"),
                                create_felt("0x1"),
                            ]
                        }
                    },
                    Event{
                        from_address :
                            Address(create_felt("0x2ef591697f0fd9adc0ba9dbe0ca04dabad80cf95f08ba02e435d9cb6698a28a")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0x1c93f6e4703ae90f75338f29bffbe9c1662200cee981f49afeec26e892debcd"),
                                create_felt("0x53f716bb3e8a7730e612809aa0416ce2cbf4aff462e4784d5f2f29d5e96605c"),
                                create_felt("0x6f7c4350d6d5ee926b3ac4fa0c9c351055456e75c92227468d84232fc493a9c"),
                            ],
                            data : vec ![
                                create_felt("0x1"),
                                create_felt("0x15c91"),
                                create_felt("0x4"),
                                create_felt("0x20"),
                                create_felt("0x6"),
                                create_felt("0x24"),
                                create_felt("0x1"),
                            ]
                        }
                    },
                    Event{
                        from_address :
                            Address(create_felt("0x2ef591697f0fd9adc0ba9dbe0ca04dabad80cf95f08ba02e435d9cb6698a28a")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0x1c93f6e4703ae90f75338f29bffbe9c1662200cee981f49afeec26e892debcd"),
                                create_felt("0x53f716bb3e8a7730e612809aa0416ce2cbf4aff462e4784d5f2f29d5e96605c"),
                                create_felt("0x6f7c4350d6d5ee926b3ac4fa0c9c351055456e75c92227468d84232fc493a9c"),
                            ],
                            data : vec ![
                                create_felt("0x1"),  create_felt("0x15c91"), create_felt("0x20"), create_felt("0x20"),
                                create_felt("0x0"),  create_felt("0x82"),    create_felt("0x27"), create_felt("0x7"),
                                create_felt("0x0"),  create_felt("0x0"),     create_felt("0x1"),  create_felt("0x6"),
                                create_felt("0x2"),  create_felt("0x1"),     create_felt("0x4"),  create_felt("0x3"),
                                create_felt("0x2"),  create_felt("0xd"),     create_felt("0x19"), create_felt("0x14"),
                                create_felt("0x12"), create_felt("0x19"),    create_felt("0x21"), create_felt("0x1e"),
                                create_felt("0x21"), create_felt("0x23"),    create_felt("0x12"), create_felt("0x27"),
                                create_felt("0x12"), create_felt("0x0"),     create_felt("0x0"),  create_felt("0x0"),
                                create_felt("0x0"),  create_felt("0x0"),     create_felt("0x20"),
                            ]
                        }
                    },
                    Event{
                        from_address :
                            Address(create_felt("0x2ef591697f0fd9adc0ba9dbe0ca04dabad80cf95f08ba02e435d9cb6698a28a")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0x1a2f334228cee715f1f0f54053bb6b5eac54fa336e0bc1aacf7516decb0471d"),
                                create_felt("0x48f9eedcda02e2ed3ebc286dab3e38e7129b444bdef510b0ebbeecdfc547be0"),
                                create_felt("0x646e828fe2447ee4c66e49face4ccd4cbc2fd8ce93252679cec9bf479a52d7b"),
                            ],
                            data : vec ![
                                create_felt("0x1"),
                                create_felt("0x15c91"),
                                create_felt("0x1"),
                                create_felt("0x80000000000000249c248c427842642450323464088c100000e009c82"),
                            ]
                        }
                    },
                    Event{
                        from_address :
                            Address(create_felt("0x2ef591697f0fd9adc0ba9dbe0ca04dabad80cf95f08ba02e435d9cb6698a28a")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0x1c93f6e4703ae90f75338f29bffbe9c1662200cee981f49afeec26e892debcd"),
                                create_felt("0x3e509804fbdba096142d78c1563c907a80c266c5dfcbda494d1d4e4d13a2215"),
                                create_felt("0x2f1c516fa4d2c41f2021edc3b46f33326e73755e55982374381150e6d8d12df"),
                            ],
                            data : vec ![
                                create_felt("0x1"),
                                create_felt("0x15c91"),
                                create_felt("0x1"),
                                create_felt("0x27"),
                            ]
                        }
                    },
                    Event{
                        from_address :
                            Address(create_felt("0x36017e69d21d6d8c13e266eabb73ef1f1d02722d86bdcabe5f168f8e549d3cd")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0x290118457a640990dbcdeb696bd7f53f1d7d71d19b7d566efd42da398c908d3"),
                                create_felt("0x15c91"),
                                create_felt("0x0"),
                            ],
                            data : vec ![]
                        }
                    },
                    Event{
                        from_address :
                            Address(create_felt("0x15f125aaeb8429544c8db516ebfcdfd447c6cd2b6c91fbbc2ffa52c18611931")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0x1dcde06aabdbca2f80aa51392b345d7549d7757aa855f7e37f5d335ac8243b1"),
                                create_felt("0x2e1bc11cf78afaea41e62ffa30a92500056c28b673c5e05366076a245a0a048"),
                            ],
                            data : vec ![
                                create_felt("0x2"),
                                create_felt("0x0"),
                                create_felt("0x0"),
                            ]
                        }
                    },
                    Event{
                        from_address :
                            Address(create_felt("0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d")),
                        event_content : EventContent{
                            keys : vec ![
                                create_felt("0x99cd8bde557814842a3121e8ddfd433a539b8c9f14bf31ebf108d12e6196e9"),
                            ],
                            data : vec ![
                                create_felt("0x498631aa7ebcb7be6acaf5990eb41488bef2597967885cc5ca96ccb5493a59e"),
                                create_felt("0x1176a1bd84444c89232ec27754698e5d2e7e1a7f1539f12027f28b23ec9f3d8"),
                                create_felt("0x112e0b264e8bdc0"),
                                create_felt("0x0"),
                            ]
                        }
                    },
                ],
                execution_resources : ExecutionResourcesDataAvailability{
                    l1_data_gas : 384,
                    l1_gas : 0,
                    l2_gas : 25790400,
                },
                finality_status : TxnFinalityStatus::AcceptedOnL2,
                messages_sent : vec ![],
                transaction_hash : TxnHash(create_felt("0x1b3fe87721d0d84535e6047263abd1f74535f5fc07c9681f8cbd58ef5eda895")),
                result_common_receipt_properties :
                    ResultCommonReceiptProperties::SuccessfulCommonReceiptProperties(SuccessfulCommonReceiptProperties{
                        execution_status : SuccessfulCommonReceiptPropertiesExecutionStatus::Succeeded,
                    }, ),
            },
        });

        let block_header = BlockHeader {
            block_hash: BlockHash(create_felt("0x6b492c7a1a03c422451e1fa8c246573c8c7239d050b41620f03b2b5fa3a461f")),
            block_number: BlockNumber::try_new(3573626).expect("Failed to create BlockNumber"),
            l1_da_mode: Some(BlockHeaderL1DaMode::Blob),
            l1_data_gas_price: Some(ResourcePrice {
                price_in_fri: create_felt("0xfaf24"),
                price_in_wei: create_felt("0x2d"),
            }),
            l1_gas_price: ResourcePrice {
                price_in_fri: create_felt("0x1c5b3206b3c9"),
                price_in_wei: create_felt("0x515ba424"),
            },
            l2_gas_price: ResourcePrice {
                price_in_fri: create_felt("0xb2d05e00"),
                price_in_wei: create_felt("0x2010a"),
            },
            new_root: create_felt("0x5bc87df12fc2a96a350c31cf8b93601c3b33521879df49a107a426e36b71e68"),
            parent_hash: BlockHash(create_felt("0x4c95cb5f7c602a5e78da1618573a06fa06cd895e90318f55b041613c1fa6e0a")),
            sequencer_address: create_felt("0x1176a1bd84444c89232ec27754698e5d2e7e1a7f1539f12027f28b23ec9f3d8"),
            starknet_version: "0.14.0".to_string(),
            timestamp: BlockHeaderTimestamp::try_new(1763114861).expect("Failed to create BlockHeaderTimestamp"),
        };

        BlockWithReceipts {
            status: BlockStatus::AcceptedOnL2,
            block_header,
            block_body_with_receipts: BlockBodyWithReceipts {
                transactions: vec![
                    TransactionAndReceipt {
                        transaction: tx1,
                        receipt: receipt1,
                    },
                    TransactionAndReceipt {
                        transaction: tx2,
                        receipt: receipt2,
                    },
                    TransactionAndReceipt {
                        transaction: tx3,
                        receipt: receipt3,
                    },
                ],
            },
        }
    }

    fn create_test_state_update() -> StateUpdate {
        StateUpdate {
            block_hash: BlockHash(create_felt("0x6b492c7a1a03c422451e1fa8c246573c8c7239d050b41620f03b2b5fa3a461f")),
            new_root: create_felt("0x5bc87df12fc2a96a350c31cf8b93601c3b33521879df49a107a426e36b71e68"),
            old_root: create_felt("0x5340acb42e122c008dc3102168d560f0a71c38ef6f86af27ab2ad029d8f3acd"),
            state_diff: StateDiff {
                declared_classes: vec![],
                deployed_contracts: vec![],
                deprecated_declared_classes: vec![],
                nonces: vec![
                    NonceUpdate {
                        contract_address: Some(Address(create_felt("0x285cbc386563791682cd06f40e2c7fb856ee3cfa0ef7727bd7e7b410b7e71d1"))),
                        nonce: Some(create_felt("0x26dde")),
                    },
                    NonceUpdate {
                        contract_address: Some(Address(create_felt("0x761a5d53b8133d70140845fbc522f63adf80f3b9ed979d2eb7f772f76c1b206"))),
                        nonce: Some(create_felt("0x12b79")),
                    },
                    NonceUpdate {
                        contract_address: Some(Address(create_felt("0x498631aa7ebcb7be6acaf5990eb41488bef2597967885cc5ca96ccb5493a59e"))),
                        nonce: Some(create_felt("0x4b5d6")),
                    }
                ],
                replaced_classes: vec![],
                storage_diffs: vec![
                    ContractStorageDiffItem {
                        address: create_felt("0x2"),
                        storage_entries: vec![
                            StorageDiffItem {
                                key: Some(create_felt("0x485ccebbe72181affcc04c1ed9208428084ed9d71f0d9fce63e2e9450e4dae8")),
                                value: Some(create_felt("0x66f3bca")),
                            },
                            StorageDiffItem {
                                key: Some(create_felt("0x0")),
                                value: Some(create_felt("0x66f3bcb")),
                            }
                        ],
                    },
                    ContractStorageDiffItem {
                        address: create_felt("0x1"),
                        storage_entries: vec![
                            StorageDiffItem {
                                key: Some(create_felt("0x368770")),
                                value: Some(create_felt("0x460e9c0d461444c2dba7bb8fe7fd59651287414741368a307a11c0a0bd7530d")),
                            }
                        ]
                    },
                    ContractStorageDiffItem {
                        address: create_felt("0x2ef591697f0fd9adc0ba9dbe0ca04dabad80cf95f08ba02e435d9cb6698a28a"),
                        storage_entries: vec![
                            StorageDiffItem {
                                key: Some(create_felt("0x1c14c108b2af06354e8abab7949a2efe4100367c61af1722667d2f8a7dc3a70")),
                                value: Some(create_felt("0x80000000000000249c248c427842642450323464088c100000e009c82")),
                            },
                            StorageDiffItem {
                                key: Some(create_felt("0x6062c9a3d43f308a8d185cd70ae1ad161fd1147d16c85aa2aeccab960c514e0")),
                                value: Some(create_felt("0x2d032fec21e8a0b20950883206d085a02472025630b9e300009a0b2124")),
                            }
                        ]
                    },
                    ContractStorageDiffItem {
                        address: create_felt("0x68f5c6a61780768455de69077e07e89787839bf8166decfbf92b645209c0fb8"),
                        storage_entries: vec![
                            StorageDiffItem {
                                key: Some(create_felt("0x527a86fc9c86c73d05051692afdfff7d989003b44dda11372e6d9fadd5d403d")),
                                value: Some(create_felt("0x5080b")),
                            },
                            StorageDiffItem {
                                key: Some(create_felt("0x795ea861f79356808e825d60f2f3a62e97b344ade5a9e2949a3f61df2dc074b")),
                                value: Some(create_felt("0x8f766c999")),
                            }
                        ]
                    },
                    ContractStorageDiffItem {
                        address: create_felt("0x15f125aaeb8429544c8db516ebfcdfd447c6cd2b6c91fbbc2ffa52c18611931"),
                        storage_entries: vec![
                            StorageDiffItem {
                                key: Some(create_felt("0x1a94b3504078f726b3ad082a04d184edee4a19fcbb9210737a543edbb862856")),
                                value: Some(create_felt("0x3fffffffffffff")),
                            }
                        ]
                    },
                    ContractStorageDiffItem {
                        address: create_felt("0x1634a1121191f06618c4eaa490550b897e4f3d6ba07bb2f05956c8ae430a503"),
                        storage_entries: vec![
                            StorageDiffItem {
                                key: Some(create_felt("0x7b6effa68a264879286569ad703efb95b05d67fdc332ba85670b16269bfe665")),
                                value: Some(create_felt("0x1fffffffffffff")),
                            }
                        ]
                    },
                    ContractStorageDiffItem {
                        address: create_felt("0x1a9fda7208cd4743f3d0ef37e09f633934cf66325691ecadd5b9fa246d4252d"),
                        storage_entries: vec![
                            StorageDiffItem {
                                key: Some(create_felt("0x485ccebbe72181affcc04c1ed9208428084ed9d71f0d9fce63e2e9450e4dae8")),
                                value: Some(create_felt("0x1")),
                            }
                        ]
                    },
                    ContractStorageDiffItem {
                        address: create_felt("0x4e06e04b8d624d039aa1c3ca8e0aa9e21dc1ccba1d88d0d650837159e0ee054"),
                        storage_entries: vec![
                            StorageDiffItem {
                                key: Some(create_felt("0x664f403aa66fd1d6ae882e7012d93f50493198563a0ed7f633e37b7febfd034")),
                                value: Some(create_felt("0x6483cb9a4362d25a7c3000000000000000168f3ef95b63fab5d")),
                            }
                        ]
                    },
                    ContractStorageDiffItem {
                        address: create_felt("0x391bd9b58695b952aa15cffce50ba4650c954105df405ca8fc976ad7a65d646"),
                        storage_entries: vec![
                            StorageDiffItem {
                                key: Some(create_felt("0x110e2f729c9c2b988559994a3daccd838cf52faf88e18101373e67dd061455a")),
                                value: Some(create_felt("0x3c6d9cc55ff0342e36")),
                            },
                            StorageDiffItem {
                                key: Some(create_felt("0x527a86fc9c86c73d05051692afdfff7d989003b44dda11372e6d9fadd5d403d")),
                                value: Some(create_felt("0x4f5b8a25426cb346")),
                            }
                        ]
                    },
                    ContractStorageDiffItem {
                        address: create_felt("0x4718f5a0fc34cc1af16a1cdee98ffb20c31f5cd61d6ab07201858f4287c938d"),
                        storage_entries: vec![
                            StorageDiffItem {
                                key: Some(create_felt("0x40d7faccfef431632e6839785801493f157215c90b25480249ad657e618ab3c")),
                                value: Some(create_felt("0x392429b320dda54e3")),
                            },
                            StorageDiffItem {
                                key: Some(create_felt("0x3cdd695b27d9a10cc5517e01a41147b4914bda7c0684c57945aeff37c9c0b6b")),
                                value: Some(create_felt("0x5bf3ec0e09aff844")),
                            },
                            StorageDiffItem {
                                key: Some(create_felt("0x5496768776e3db30053404f18067d81a6e06f5a2b0de326e21298fd9d569a9a")),
                                value: Some(create_felt("0x1d1c22827ee9ca97c3c20")),
                            },
                            StorageDiffItem {
                                key: Some(create_felt("0x188516bcd62ad8b85cf54665e5e796b6502618cf5ec524533135fcc5e939fe5")),
                                value: Some(create_felt("0x70881ac569078317b")),
                            }
                        ]
                    },
                    ContractStorageDiffItem {
                        address: create_felt("0xd8d6dfec4d33bfb6895de9f3852143a17c6f92fd2a21da3d6924d34870160"),
                        storage_entries: vec![
                            StorageDiffItem {
                                key: Some(create_felt("0x372e86e355fda754fad152bb84ee22e847e9b336c96e414d59efd546e3804ca")),
                                value: Some(create_felt("0x40e3e20c83773ead00")),
                            },
                            StorageDiffItem {
                                key: Some(create_felt("0x5478c4733f224a066388257c39e01d9a0aa11442be54e658410bd4bba33103b")),
                                value: Some(create_felt("0x14b52bd5d0e58d66873802a506916ff6d")),
                            },
                            StorageDiffItem {
                                key: Some(create_felt("0x5478c4733f224a066388257c39e01d9a0aa11442be54e658410bd4bba33103a")),
                                value: Some(create_felt("0x6135f000000000000000000000008f7106423")),
                            },
                            StorageDiffItem {
                                key: Some(create_felt("0x5478c4733f224a066388257c39e01d9a0aa11442be54e658410bd4bba331039")),
                                value: Some(create_felt("0x1ab262a270845729aeda00000000000022ce28e27879a4e79e7d")),
                            },
                            StorageDiffItem {
                                key: Some(create_felt("0x5279b8d217915410ad102ad1b4650dfbe95d243f66a3cd010fdbf316c4faa8a")),
                                value: Some(create_felt("0x6b49d2000df4c1554804d0936916ff6d")),
                            },
                            StorageDiffItem {
                                key: Some(create_felt("0x5ff22447f4b1b630afc7fa6e0461d6443a31dddf5946773b5587142afcd6211")),
                                value: Some(create_felt("0x0")),
                            }
                        ]
                    }
                ],
            },
        }
    }

    #[test]
    fn test_validate_block_hash() {
        // Create test data structures
        let block = create_test_block_with_receipts();
        let state_update = create_test_state_update();

        let block_hash = create_felt(
            "0x6b492c7a1a03c422451e1fa8c246573c8c7239d050b41620f03b2b5fa3a461f",
        );

        let result = validate_block_hash(&block, &state_update, &block_hash);

        assert!(result.is_ok());
    }
    #[test]
    fn test_validate_block_hash_invalid() {
        // Create test data structures
        let block = create_test_block_with_receipts();
        let state_update = create_test_state_update();

        let block_hash = create_felt("0x321");

        let result = validate_block_hash(&block, &state_update, &block_hash);

        assert!(result.is_err());
    }
}
