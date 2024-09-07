use crate::proposer::Proposer;
use crate::proposer::ProposerArc;
use fastlog::config::AuthorityServerConfig;
use node_api::config;
use node_api::config::ProposerConfig;
use node_api::error::{ErrorCodes, ProposerConfigError};
use structopt::StructOpt;
use tracing::*;

use crate::cli::command::{eth_account, init_db};
use std::path::PathBuf;

#[derive(StructOpt, Debug)]
#[structopt(name = "proposer-runner")]
struct ProposerCli {
    #[structopt(
        short = "c",
        long = "proposer_config",
        parse(from_os_str),
        help = "Yaml file only"
    )]
    config_path: std::path::PathBuf,

    #[structopt(
        short = "ac",
        long = "authority_config",
        parse(from_os_str),
        help = "Json file only"
    )]
    authority_path: std::path::PathBuf,

    #[structopt(
        short = "i",
        long = "init_pg",
        help = "Init & refresh pg, caution: new db & new table"
    )]
    init_pg: Option<String>,

    #[structopt(
        short = "k",
        long = "eth_account",
        help = "Gen a eth account, and keypair"
    )]
    eth_account: bool,
}

pub async fn run_cli() {
    let mut help_info = true;
    let args = ProposerCli::from_args();

    // init pg db
    if let Some(pg_conn_str) = args.init_pg {
        help_info = false;
        info!("PostgreSQL connection addr: {}", pg_conn_str);
        // Use the PostgreSQL connection string here for initialization
        if !init_db(pg_conn_str).await {
            return;
        }
    }

    // gen eth account
    if args.eth_account {
        help_info = false;
        info!("Gen a eth account, and keypair");
        if !eth_account() {
            return;
        }
    }

    // setup node
    if args.config_path.is_file() {
        help_info = false;

        let auth_config_path = args.authority_path.to_str().expect("Authority path not exist");
        let proposer_config = construct_node_config(args.config_path);
        let server_config = AuthorityServerConfig::read(auth_config_path).expect("Fail to read authority server config");
        let _proposer = build_proposer(proposer_config, server_config).await;
    }

    if help_info {
        info!("\nPlease exec: proposer -h for help info.\n")
    }
}

pub fn construct_node_config(config_path: PathBuf) -> config::ProposerConfig {
    match config::ProposerConfig::load_config(config_path) {
        Err(ProposerConfigError::ConfigMissing(_)) => {
            error!("config path can't found.");
            std::process::exit(ErrorCodes::PROCESS_EXIT);
        }
        Err(ProposerConfigError::SerializationError(_)) => {
            error!("config file can't be serialize, bad yaml format or incomplete field");
            std::process::exit(ErrorCodes::PROCESS_EXIT);
        }
        Err(ProposerConfigError::IllegalNodeId) => {
            error!("nodeid illegal, must be hex format, and 64 bits");
            std::process::exit(ErrorCodes::PROCESS_EXIT);
        }
        result => result.expect("failed to load proposer config"),
    }
}

pub async fn build_proposer(config: ProposerConfig, auth: AuthorityServerConfig) -> ProposerArc {
    Proposer::proposer_factory()
        .set_config(config, auth)
        .initialize_node()
        .await
        .map_err(|e| {
            panic!("Failed to build proposer due to error, detail {:?}", e);
        })
        .unwrap()
}
