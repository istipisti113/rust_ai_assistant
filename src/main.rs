use std::io;
use std::sync::Arc;
use tokio::sync::Mutex;

use async_openai::config;
//use clap::builder::Resettable::Value;
use tokio;

use apca::{ApiInfo, Client};
use apca::api::v2::order;
use apca::api::v2::account;

use dotenvy::dotenv;

use serde_json::{json, Value};

use tokio_util::sync::CancellationToken;

use anyhow;

mod ai_module;
mod alpaca_module;

#[tokio::main]
async fn main() {
    dotenv();
    let api_info = ApiInfo::from_env().unwrap();
    let client = Client::new(api_info);
    let account = client.issue::<account::Get>(&()).await.unwrap();
    let currency = account.currency;

    //alpaca_module::make_order(&client, "AAPL", order::Side::Buy, order::Amount::quantity(1)).await.unwrap();

    println!("cash:\t{} {currency}", account.cash);
    println!("buying power:\t{} {currency}", account.buying_power);


    let mut model = ai_module::Model::new();
    loop {
        let mut input = String::new();
        std::io::stdin().read_line(&mut input).unwrap();
        let response: String = model.send(input).await.unwrap();
        println!("{response}");
    }
}
