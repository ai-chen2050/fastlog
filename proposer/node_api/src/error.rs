use std::path::PathBuf;
use thiserror::Error;

pub struct ErrorCodes;

impl ErrorCodes {
    pub const PROCESS_EXIT: i32 = 42;

    pub const CONFIG_MISSING: u32 = 1001;
    pub const SERIALIZATION_ERROR: u32 = 1002;
    pub const IO_ERROR: u32 = 1003;
    pub const ILLEGAL_NODE_ID: u32 = 1004;
    pub const ILLEGAL_SIGNER: u32 = 1005;

    pub const API_FAIL_TO_JSON: u32 = 2001;

    pub const PRO_CUSTOM_ERROR: u32 = 3001;
    pub const PRO_FAIL_REGISTER: u32 = 3002;
    pub const PRO_CONNECT_TEE_ERROR: u32 = 3003;
    pub const PRO_SEND_VLC_ERROR: u32 = 3004;
    pub const PRO_DECODE_SIGNER_KEY_ERROR: u32 = 3005;
    pub const PRO_NEW_VRF_RANGE_CONTRACT_ERROR: u32 = 3006;
    pub const PRO_GET_RANGE_CONTRACT_ERROR: u32 = 3007;
    pub const PRO_BIND_TXCOMMIT_UDP_ERROR: u32 = 3008;
}

pub type ProposerConfigResult<T> = Result<T, ProposerConfigError>;

#[derive(Error, Debug)]
pub enum ProposerConfigError {
    #[error(
        "No proposer config found at this path: {0} (Error Code: {})",
        ErrorCodes::CONFIG_MISSING
    )]
    ConfigMissing(PathBuf),

    #[error(
        "Config deserialization error: {0} (Error Code: {})",
        ErrorCodes::SERIALIZATION_ERROR
    )]
    SerializationError(#[from] serde_yaml::Error),

    #[error(
        "Error while performing IO for the Proposer: {0} (Error Code: {})",
        ErrorCodes::IO_ERROR
    )]
    IoError(#[from] std::io::Error),

    #[error(
        "Error nodeid illegal, must be hex format, and 40 bits (Error Code: {})",
        ErrorCodes::ILLEGAL_NODE_ID
    )]
    IllegalNodeId,

    #[error(
        "Error signer illegal, must be hex format, and 64 bits (Error Code: {})",
        ErrorCodes::ILLEGAL_SIGNER
    )]
    IllegalSignerKey,
}

pub type ProposerAPIResult<T> = Result<T, ProposerAPIError>;

#[derive(Error, Debug)]
pub enum ProposerAPIError {
    #[error(
        "Error failed to serialize struct to JSON (Error Code: {})",
        ErrorCodes::API_FAIL_TO_JSON
    )]
    APIFailToJson,
}

pub type ProposerResult<T> = Result<T, ProposerError>;

#[derive(Error, Debug)]
pub enum ProposerError {
    #[error(
        "Error: some error happened, detail: {0} (Error Code: {})",
        ErrorCodes::PRO_CUSTOM_ERROR
    )]
    CustomError(String),

    #[error(
        "Error: register to dispatcher failed, detail: {0} (Error Code: {})",
        ErrorCodes::PRO_FAIL_REGISTER
    )]
    PROSetupRegister(#[from] reqwest::Error),

    #[error(
        "Error: connect to tee service failed, detail: {0}  (Error Code: {})",
        ErrorCodes::PRO_CONNECT_TEE_ERROR
    )]
    PROConnectTEEError(String),

    #[error(
        "Error: send promtp to tee service failed, detail: {0}  (Error Code: {})",
        ErrorCodes::PRO_SEND_VLC_ERROR
    )]
    PROSendPromptError(String),

    #[error(
        "Error: decode signer private key error failed, detail: {0}  (Error Code: {})",
        ErrorCodes::PRO_DECODE_SIGNER_KEY_ERROR
    )]
    PRODecodeSignerKeyError(#[from] alloy_primitives::hex::FromHexError),

    #[error(
        "Error: new vrf range contract failed, detail: {0}  (Error Code: {})",
        ErrorCodes::PRO_NEW_VRF_RANGE_CONTRACT_ERROR
    )]
    PRONewVrfRangeContractError(#[from] eyre::ErrReport),

    #[error(
        "Error: get vrf range contract failed, detail: {0}  (Error Code: {})",
        ErrorCodes::PRO_GET_RANGE_CONTRACT_ERROR
    )]
    PROGetVrfRangeContractError(String),

    #[error(
        "Error: bind txs_commit_udp failed, detail: {0}  (Error Code: {})",
        ErrorCodes::PRO_BIND_TXCOMMIT_UDP_ERROR
    )]
    PROBindTxCommitUDPError(String),
}
