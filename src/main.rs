use ethers::contract::EthLogDecode;
use ethers::contract::abigen;
use ethers::core::abi::Address;
use ethers::core::types::Filter;
use ethers::providers::{Middleware, Provider, Ws};
use futures::StreamExt;
use reqwest::Client;
use serde_json::json;
use std::env;
use dotenv::dotenv;

abigen!(
    CryptoPunksMarket,
    r#"[
        event Assign(address indexed to, uint256 punkIndex)
        event Transfer(address indexed from, address indexed to, uint256 value)
        event PunkTransfer(address indexed from, address indexed to, uint256 punkIndex)
        event PunkOffered(uint256 indexed punkIndex, uint256 minValue, address indexed toAddress)
        event PunkBidEntered(uint256 indexed punkIndex, uint256 value, address indexed fromAddress)
        event PunkBidWithdrawn(uint256 indexed punkIndex, uint256 value, address indexed fromAddress)
        event PunkBought(uint256 indexed punkIndex, uint256 value, address indexed fromAddress, address indexed toAddress)
        event PunkNoLongerForSale(uint256 indexed punkIndex)
    ]"#
);

const CONTRACT_ADDRESS: &str = "0xb47e3cd837ddf8e4c57f05d70ab865de6e193bbb";

async fn send_discord_notification(client: &Client, webhook_url: &str, message: &str) -> Result<(), reqwest::Error> {
    let payload = json!({ "content": message });
    let res = client
        .post(webhook_url)
        .json(&payload)
        .send()
        .await?;
    println!("Discord response: {:?}", res.status());
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

    dotenv().ok();

    let infura_project_id: String = env::var("INFURA_PROJECT_ID").expect("INFURA_PROJECT_ID must be set");
    let discord_webhook_url = env::var("DISCORD_WEBHOOK_URL").expect("DISCORD_WEBHOOK_URL must be set");

    let ws_url = format!("wss://mainnet.infura.io/ws/v3/{}" , &infura_project_id);

    let ws = Ws::connect(ws_url).await?;
    let provider = Provider::new(ws);

    let contract_address = CONTRACT_ADDRESS.parse::<Address>()?;

    let filter = Filter::new().address(contract_address);

    let http_client = Client::new();

    let mut stream = provider.subscribe_logs(&filter).await?;
    println!("Listening for events from contract {}...", contract_address);

    while let Some(log) = stream.next().await {

        let log = ethers::core::abi::RawLog {
            topics: log.topics.clone(),
            data: log.data.to_vec(),
        };
        let message = if let Ok(event) = crypto_punks_market::AssignFilter::decode_log(&log) {
            format!(
                "Assign: punk {} assigned to {:?}",
                event.punk_index, event.to
            )
        } else if let Ok(event) = crypto_punks_market::TransferFilter::decode_log(&log) {
            format!(
                "Transfer: from {:?} to {:?}, value: {}",
                event.from, event.to, event.value
            )
        } else if let Ok(event) = crypto_punks_market::PunkTransferFilter::decode_log(&log) {
            format!(
                "PunkTransfer: punk {} transferred from {:?} to {:?}",
                event.punk_index, event.from, event.to
            )
        } else if let Ok(event) = crypto_punks_market::PunkOfferedFilter::decode_log(&log) {
            format!(
                "PunkOffered: punk {} offered at minimum {} wei to {:?}",
                event.punk_index, event.min_value, event.to_address
            )
        } else if let Ok(event) = crypto_punks_market::PunkBidEnteredFilter::decode_log(&log) {
            format!(
                "PunkBidEntered: punk {} bid {} wei by {:?}",
                event.punk_index, event.value, event.from_address
            )
        } else if let Ok(event) = crypto_punks_market::PunkBidWithdrawnFilter::decode_log(&log) {
            format!(
                "PunkBidWithdrawn: punk {} bid withdrawn by {:?} for {} wei",
                event.punk_index, event.from_address, event.value
            )
        } else if let Ok(event) = crypto_punks_market::PunkBoughtFilter::decode_log(&log) {
            format!(
                "PunkBought: punk {} bought for {} wei from {:?} to {:?}",
                event.punk_index, event.value, event.from_address, event.to_address
            )
        } else if let Ok(event) = crypto_punks_market::PunkNoLongerForSaleFilter::decode_log(&log) {
            format!(
                "PunkNoLongerForSale: punk {} is no longer for sale",
                event.punk_index
            )
        } else {
            format!("Unknown event: {:?}", log)
        };

        println!("{}", message);

        if let Err(err) = send_discord_notification(&http_client, &discord_webhook_url, &message).await {
            eprintln!("Failed to send Discord notification: {:?}", err);
        }
    }

    Ok(())
}
