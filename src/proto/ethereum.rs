use serde::Deserialize;
use serde::Serialize;

// @generated
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message, Serialize, Deserialize)]
pub struct Block {
    #[prost(uint64, tag = "1")]
    pub chain_id: u64,
    #[prost(string, tag = "2")]
    pub block_hash: ::prost::alloc::string::String,
    #[prost(string, tag = "3")]
    pub parent_hash: ::prost::alloc::string::String,
    #[prost(uint64, tag = "4")]
    pub block_number: u64,
    #[prost(message, optional, tag = "5")]
    pub header: ::core::option::Option<Header>,
    #[prost(message, repeated, tag = "6")]
    pub transactions: ::prost::alloc::vec::Vec<Transaction>,
    #[prost(message, repeated, tag = "7")]
    pub logs: ::prost::alloc::vec::Vec<Log>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message, Serialize, Deserialize)]
pub struct Header {
    /// Miner/author’s address. None if pending.
    #[prost(string, tag = "1")]
    pub author: ::prost::alloc::string::String,
    /// State root hash
    #[prost(string, tag = "2")]
    pub state_root: ::prost::alloc::string::String,
    /// Transactions root hash
    #[prost(string, tag = "3")]
    pub transactions_root: ::prost::alloc::string::String,
    /// Transactions receipts root hash
    #[prost(string, tag = "4")]
    pub receipts_root: ::prost::alloc::string::String,
    /// Gas Used (bigint) as numeric string
    #[prost(string, tag = "5")]
    pub gas_used: ::prost::alloc::string::String,
    /// Gas limit (bigint) as numeric string
    #[prost(string, tag = "6")]
    pub gas_limit: ::prost::alloc::string::String,
    #[prost(string, tag = "7")]
    pub extra_data: ::prost::alloc::string::String,
    /// Logs bloom
    #[prost(string, optional, tag = "8")]
    pub logs_bloom: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(string, tag = "9")]
    pub timestamp: ::prost::alloc::string::String,
    /// difficulty in numeric string
    #[prost(string, tag = "10")]
    pub difficulty: ::prost::alloc::string::String,
    /// total-difficulty - bigint in numeric string
    #[prost(string, tag = "11")]
    pub total_difficulty: ::prost::alloc::string::String,
    #[prost(string, repeated, tag = "12")]
    pub seal_fields: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    /// Block size in number
    #[prost(uint64, optional, tag = "13")]
    pub size: ::core::option::Option<u64>,
    /// Base fee per unit of gas (if past London)
    #[prost(string, optional, tag = "14")]
    pub base_fee_per_gas: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(string, tag = "15")]
    pub nonce: ::prost::alloc::string::String,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message, Serialize, Deserialize)]
pub struct Transaction {
    #[prost(string, tag = "1")]
    pub hash: ::prost::alloc::string::String,
    #[prost(uint64, tag = "2")]
    pub nonce: u64,
    /// Block hash. None when pending.
    #[prost(string, optional, tag = "3")]
    pub block_hash: ::core::option::Option<::prost::alloc::string::String>,
    /// Block number. None when pending.
    #[prost(uint64, optional, tag = "4")]
    pub block_number: ::core::option::Option<u64>,
    /// Transaction Index. None when pending.
    #[prost(uint64, optional, tag = "5")]
    pub transaction_index: ::core::option::Option<u64>,
    /// Sender
    #[prost(string, tag = "6")]
    pub from_address: ::prost::alloc::string::String,
    /// Recipient (None when contract creation)
    #[prost(string, optional, tag = "7")]
    pub to_address: ::core::option::Option<::prost::alloc::string::String>,
    /// Transferred value
    #[prost(string, tag = "8")]
    pub value: ::prost::alloc::string::String,
    /// Gas Price, null for Type 2 transactions
    #[prost(string, optional, tag = "9")]
    pub gas_price: ::core::option::Option<::prost::alloc::string::String>,
    /// Gas amount
    #[prost(string, tag = "10")]
    pub gas: ::prost::alloc::string::String,
    /// Input data
    #[prost(string, tag = "11")]
    pub input: ::prost::alloc::string::String,
    /// Signature
    #[prost(uint64, tag = "12")]
    pub v: u64,
    #[prost(string, tag = "13")]
    pub r: ::prost::alloc::string::String,
    #[prost(string, tag = "14")]
    pub s: ::prost::alloc::string::String,
    /// Transaction type, Some(2) for EIP-1559 transaction, Some(1) for AccessList transaction, None for Legacy
    #[prost(enumeration = "TransactionType", optional, tag = "15")]
    pub transaction_type: ::core::option::Option<i32>,
    #[prost(message, optional, tag = "16")]
    pub access_list: ::core::option::Option<AccessList>,
    /// <https://docs.rs/ethers/latest/ethers/types/struct.Transaction.html#structfield.max_priority_fee_per_gas>
    #[prost(string, optional, tag = "17")]
    pub max_priority_fee_per_gas: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(string, optional, tag = "18")]
    pub max_fee_per_gas: ::core::option::Option<::prost::alloc::string::String>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message, Serialize, Deserialize)]
pub struct AccessList {
    #[prost(message, repeated, tag = "1")]
    pub item: ::prost::alloc::vec::Vec<AccessListItem>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message, Serialize, Deserialize)]
pub struct AccessListItem {
    #[prost(string, tag = "1")]
    pub address: ::prost::alloc::string::String,
    #[prost(string, repeated, tag = "2")]
    pub storage_keys: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
#[allow(clippy::derive_partial_eq_without_eq)]
#[derive(Clone, PartialEq, ::prost::Message, Serialize, Deserialize)]
pub struct Log {
    /// <https://docs.rs/ethers/latest/ethers/types/struct.Log.html>
    #[prost(string, tag = "1")]
    pub address: ::prost::alloc::string::String,
    #[prost(string, repeated, tag = "2")]
    pub topics: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(string, tag = "3")]
    pub data: ::prost::alloc::string::String,
    #[prost(string, optional, tag = "4")]
    pub block_hash: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(uint64, optional, tag = "5")]
    pub block_number: ::core::option::Option<u64>,
    #[prost(string, optional, tag = "6")]
    pub transaction_hash: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(uint64, optional, tag = "7")]
    pub transaction_index: ::core::option::Option<u64>,
    #[prost(uint64, optional, tag = "8")]
    pub log_index: ::core::option::Option<u64>,
    #[prost(uint64, optional, tag = "9")]
    pub transaction_log_index: ::core::option::Option<u64>,
    #[prost(string, optional, tag = "10")]
    pub log_type: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(bool, optional, tag = "11")]
    pub removed: ::core::option::Option<bool>,
}
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    ::prost::Enumeration,
    Serialize,
    Deserialize,
)]
#[repr(i32)]
pub enum TransactionType {
    /// All transactions that ever existed prior Berlin fork before EIP-2718 was implemented.
    ///
    /// Transaction that specicy an access list of contract/storage_keys that is going to be used
    /// in this transaction.
    Legacy = 0,
    /// Added in Berlin fork (EIP-2930).
    AccessList = 1,
    /// Transaction that specifis an access list just like TRX_TYPE_ACCESS_LIST but in addition defines the
    /// max base gas gee and max priority gas fee to pay for this transaction. Transaction's of those type are
    /// executed against EIP-1559 rules which dictates a dynamic gas cost based on the congestion of the network.
    DynamicFee = 2,
}
impl TransactionType {
    /// String value of the enum field names used in the ProtoBuf definition.
    ///
    /// The values are not transformed in any way and thus are considered stable
    /// (if the ProtoBuf definition does not change) and safe for programmatic use.
    pub fn as_str_name(&self) -> &'static str {
        match self {
            TransactionType::Legacy => "LEGACY",
            TransactionType::AccessList => "ACCESS_LIST",
            TransactionType::DynamicFee => "DYNAMIC_FEE",
        }
    }
    /// Creates an enum from field names used in the ProtoBuf definition.
    pub fn from_str_name(value: &str) -> ::core::option::Option<Self> {
        match value {
            "LEGACY" => Some(Self::Legacy),
            "ACCESS_LIST" => Some(Self::AccessList),
            "DYNAMIC_FEE" => Some(Self::DynamicFee),
            _ => None,
        }
    }
}
// @@protoc_insertion_point(module)
