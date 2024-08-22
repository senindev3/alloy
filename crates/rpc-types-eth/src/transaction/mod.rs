//! RPC types for transactions

use alloy_consensus::Transaction as TxTrait;
use alloy_network_primitives::TransactionResponse;
use alloy_primitives::{Address, BlockHash, Bytes, B256, U256};
use serde::{Deserialize, Serialize};

pub use alloy_consensus::BlobTransactionSidecar;
pub use alloy_eips::{
    eip2930::{AccessList, AccessListItem, AccessListResult},
    eip7702::Authorization,
};

mod common;
pub use common::TransactionInfo;

mod error;
pub use error::ConversionError;

mod receipt;
pub use receipt::{AnyTransactionReceipt, TransactionReceipt};

pub mod request;
pub use request::{TransactionInput, TransactionRequest};

mod any;
pub use any::AnyTxEnvelope;

pub use alloy_consensus::{
    AnyReceiptEnvelope, Receipt, ReceiptEnvelope, ReceiptWithBloom, TxEnvelope,
};

/// Transaction object used in RPC
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(any(test, feature = "arbitrary"), derive(arbitrary::Arbitrary))]
#[serde(rename_all = "camelCase")]
#[doc(alias = "Tx")]
pub struct Transaction<T = TxEnvelope> {
    /// Transaction envelope, containing consensus data.
    #[serde(flatten)]
    pub tx: T,
    /// Block hash
    #[serde(default)]
    pub block_hash: Option<BlockHash>,
    /// Block number
    #[serde(default, with = "alloy_serde::quantity::opt")]
    pub block_number: Option<u64>,
    /// Transaction Index
    #[serde(default, with = "alloy_serde::quantity::opt")]
    pub transaction_index: Option<u64>,
    /// Sender
    pub from: Address,
}

impl Transaction {
    /// Returns true if the transaction is a legacy or 2930 transaction.
    pub const fn is_legacy_gas(&self) -> bool {
        matches!(self.tx, TxEnvelope::Legacy(_) | TxEnvelope::Eip2930(_))
    }

    /// Converts [Transaction] into [TransactionRequest].
    ///
    /// During this conversion data for [TransactionRequest::sidecar] is not populated as it is not
    /// part of [Transaction].
    pub fn into_request(self) -> TransactionRequest {
        self.tx.into()
    }
}

impl From<Transaction> for TxEnvelope {
    fn from(value: Transaction) -> Self {
        value.tx
    }
}

impl TransactionResponse for Transaction {
    fn tx_hash(&self) -> B256 {
        *self.tx.tx_hash()
    }

    fn from(&self) -> Address {
        self.from
    }

    fn to(&self) -> Option<Address> {
        self.tx.to().to().copied()
    }

    fn value(&self) -> U256 {
        self.tx.value()
    }

    fn gas(&self) -> u128 {
        self.tx.gas_limit()
    }

    fn input(&self) -> &Bytes {
        self.tx.input()
    }
}

impl TransactionResponse for Transaction<AnyTxEnvelope> {
    fn tx_hash(&self) -> B256 {
        self.tx.hash
    }

    fn from(&self) -> Address {
        self.from
    }

    fn to(&self) -> Option<Address> {
        self.tx.to
    }

    fn value(&self) -> U256 {
        self.tx.value
    }

    fn gas(&self) -> u128 {
        self.tx.gas
    }

    fn input(&self) -> &Bytes {
        &self.tx.input
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_consensus::{Signed, TxEip7702, TxLegacy};
    use alloy_primitives::{Parity, Signature, TxKind};
    use arbitrary::Arbitrary;
    use rand::Rng;
    use std::str::FromStr;

    #[test]
    #[cfg(feature = "k256")]
    fn arbitrary_transaction() {
        let mut bytes = [0u8; 1024];
        rand::thread_rng().fill(bytes.as_mut_slice());
        let _: Transaction =
            Transaction::arbitrary(&mut arbitrary::Unstructured::new(&bytes)).unwrap();
    }

    #[test]
    fn serde_transaction() {
        let transaction = Transaction {
            tx: Signed::new_unchecked(TxEip7702 {
                    nonce: 2,
                    chain_id: 17,
                    max_fee_per_gas: 21,
                    max_priority_fee_per_gas: 22,
                    gas_limit: 10,
                    to: TxKind::Call(Address::with_last_byte(7)),
                    value: U256::from(8),
                    authorization_list: vec![Authorization {
                        chain_id: U256::from(1u64),
                        address: Address::left_padding_from(&[6]),
                        nonce: 1u64,
                    }.into_signed(Signature::from_str("48b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353efffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c8041b").unwrap())],
                    input: vec![11, 12, 13].into(),
                    access_list: AccessList::default(),
                },
                Signature::from_rs_and_parity(U256::from(14), U256::from(14), 36).unwrap(),
                B256::with_last_byte(1)).into(),
            block_hash: Some(B256::with_last_byte(3)),
            block_number: Some(4),
            transaction_index: Some(5),
            from: Address::with_last_byte(6),
        };
        let serialized = serde_json::to_string(&transaction).unwrap();
        assert_eq!(
            serialized,
            r#"{"type":"0x4","chainId":"0x11","nonce":"0x2","gas":"0xa","maxFeePerGas":"0x15","maxPriorityFeePerGas":"0x16","to":"0x0000000000000000000000000000000000000007","value":"0x8","accessList":[],"authorizationList":[{"chainId":"0x1","address":"0x0000000000000000000000000000000000000006","nonce":"0x1","r":"0x48b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353","s":"0xefffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c804","v":27}],"input":"0x0b0c0d","r":"0xe","s":"0xe","v":"0x24","hash":"0x0000000000000000000000000000000000000000000000000000000000000001","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000003","blockNumber":"0x4","transactionIndex":"0x5","from":"0x0000000000000000000000000000000000000006"}"#
        );
        let deserialized: Transaction = serde_json::from_str(&serialized).unwrap();
        assert_eq!(transaction, deserialized);
    }

    #[test]
    fn serde_transaction_with_parity_bit() {
        let transaction = Transaction {
            tx: Signed::new_unchecked(TxEip7702 {
                    nonce: 2,
                    chain_id: 17,
                    max_fee_per_gas: 21,
                    max_priority_fee_per_gas: 22,
                    gas_limit: 10,
                    to: TxKind::Call(Address::with_last_byte(7)),
                    value: U256::from(8),
                    authorization_list: vec![Authorization {
                        chain_id: U256::from(1u64),
                        address: Address::left_padding_from(&[6]),
                        nonce: 1u64,
                    }.into_signed(Signature::from_str("48b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353efffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c8041b").unwrap())],
                    input: vec![11, 12, 13].into(),
                    access_list: AccessList::default(),
                },
                Signature::from_rs_and_parity(U256::from(14), U256::from(14), Parity::Parity(true)).unwrap(),
                B256::with_last_byte(1)).into(),
            block_hash: Some(B256::with_last_byte(3)),
            block_number: Some(4),
            transaction_index: Some(5),
            from: Address::with_last_byte(6),
        };
        let serialized = serde_json::to_string(&transaction).unwrap();
        assert_eq!(
            serialized,
            r#"{"type":"0x4","chainId":"0x11","nonce":"0x2","gas":"0xa","maxFeePerGas":"0x15","maxPriorityFeePerGas":"0x16","to":"0x0000000000000000000000000000000000000007","value":"0x8","accessList":[],"authorizationList":[{"chainId":"0x1","address":"0x0000000000000000000000000000000000000006","nonce":"0x1","r":"0x48b55bfa915ac795c431978d8a6a992b628d557da5ff759b307d495a36649353","s":"0xefffd310ac743f371de3b9f7f9cb56c0b28ad43601b4ab949f53faa07bd2c804","v":27}],"input":"0x0b0c0d","r":"0xe","s":"0xe","yParity":"0x1","hash":"0x0000000000000000000000000000000000000000000000000000000000000001","blockHash":"0x0000000000000000000000000000000000000000000000000000000000000003","blockNumber":"0x4","transactionIndex":"0x5","from":"0x0000000000000000000000000000000000000006"}"#
        );
        let deserialized: Transaction = serde_json::from_str(&serialized).unwrap();
        assert_eq!(transaction, deserialized);
    }

    #[test]
    fn serde_minimal_transaction() {
        let transaction = Transaction {
            tx: Signed::new_unchecked(
                TxLegacy { ..Default::default() },
                Signature::from_rs_and_parity(U256::from(14), U256::from(14), 36).unwrap(),
                B256::with_last_byte(1),
            )
            .into(),
            from: Default::default(),
            block_hash: None,
            block_number: None,
            transaction_index: None,
        };
        let serialized = serde_json::to_string(&transaction).unwrap();
        assert_eq!(
            serialized,
            r#"{"type":"0x0","nonce":"0x0","gasPrice":"0x0","gas":"0x0","value":"0x0","input":"0x","r":"0xe","s":"0xe","v":"0x24","hash":"0x0000000000000000000000000000000000000000000000000000000000000001","blockHash":null,"blockNumber":null,"transactionIndex":null,"from":"0x0000000000000000000000000000000000000000"}"#
        );
        let deserialized: Transaction = serde_json::from_str(&serialized).unwrap();
        assert_eq!(transaction, deserialized);
    }

    #[test]
    fn into_request_legacy() {
        // cast rpc eth_getTransactionByHash
        // 0xe9e91f1ee4b56c0df2e9f06c2b8c27c6076195a88a7b8537ba8313d80e6f124e --rpc-url mainnet
        let rpc_tx = r#"{"blockHash":"0x8e38b4dbf6b11fcc3b9dee84fb7986e29ca0a02cecd8977c161ff7333329681e","blockNumber":"0xf4240","hash":"0xe9e91f1ee4b56c0df2e9f06c2b8c27c6076195a88a7b8537ba8313d80e6f124e","transactionIndex":"0x1","type":"0x0","nonce":"0x43eb","input":"0x","r":"0x3b08715b4403c792b8c7567edea634088bedcd7f60d9352b1f16c69830f3afd5","s":"0x10b9afb67d2ec8b956f0e1dbc07eb79152904f3a7bf789fc869db56320adfe09","chainId":"0x0","v":"0x1c","gas":"0xc350","from":"0x32be343b94f860124dc4fee278fdcbd38c102d88","to":"0xdf190dc7190dfba737d7777a163445b7fff16133","value":"0x6113a84987be800","gasPrice":"0xdf8475800"}"#;

        let tx = serde_json::from_str::<Transaction>(rpc_tx).unwrap();
        let request = tx.into_request();
        assert!(request.gas_price.is_some());
        assert!(request.max_fee_per_gas.is_none());
    }

    #[test]
    fn into_request_eip1559() {
        // cast rpc eth_getTransactionByHash
        // 0x0e07d8b53ed3d91314c80e53cf25bcde02084939395845cbb625b029d568135c --rpc-url mainnet
        let rpc_tx = r#"{"blockHash":"0x883f974b17ca7b28cb970798d1c80f4d4bb427473dc6d39b2a7fe24edc02902d","blockNumber":"0xe26e6d","hash":"0x0e07d8b53ed3d91314c80e53cf25bcde02084939395845cbb625b029d568135c","accessList":[],"transactionIndex":"0xad","type":"0x2","nonce":"0x16d","input":"0x5ae401dc00000000000000000000000000000000000000000000000000000000628ced5b000000000000000000000000000000000000000000000000000000000000004000000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000016000000000000000000000000000000000000000000000000000000000000000e442712a6700000000000000000000000000000000000000000000b3ff1489674e11c40000000000000000000000000000000000000000000000000000004a6ed55bbcc18000000000000000000000000000000000000000000000000000000000000000800000000000000000000000003cf412d970474804623bb4e3a42de13f9bca54360000000000000000000000000000000000000000000000000000000000000002000000000000000000000000c02aaa39b223fe8d0a0e5c4f27ead9083c756cc20000000000000000000000003a75941763f31c930b19c041b709742b0b31ebb600000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000412210e8a00000000000000000000000000000000000000000000000000000000","r":"0x7f2153019a74025d83a73effdd91503ceecefac7e35dd933adc1901c875539aa","s":"0x334ab2f714796d13c825fddf12aad01438db3a8152b2fe3ef7827707c25ecab3","chainId":"0x1","v":"0x0","gas":"0x46a02","maxPriorityFeePerGas":"0x59682f00","from":"0x3cf412d970474804623bb4e3a42de13f9bca5436","to":"0x68b3465833fb72a70ecdf485e0e4c7bd8665fc45","maxFeePerGas":"0x7fc1a20a8","value":"0x4a6ed55bbcc180","gasPrice":"0x50101df3a"}"#;

        let tx = serde_json::from_str::<Transaction>(rpc_tx).unwrap();
        let request = tx.into_request();
        assert!(request.gas_price.is_none());
        assert!(request.max_fee_per_gas.is_some());
    }
}
