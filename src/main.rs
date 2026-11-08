//use tokio::sync::Mutex;

//use async_openai::config;
//use clap::builder::Resettable::Value;
use tokio;
use std::io::prelude::*;

use apca::{ApiInfo, Client};
//use apca::api::v2::order;
use apca::api::v2::account;

use dotenvy::dotenv;

mod ai_module;
mod alpaca_module;

use ai_module::Role;

use serde;

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde::Serialize;


/// A trade for an equity.
///
/// This is a user-defined type with additional fields compared to
/// [`apca::data::v2::stream::Trade`] (the default), illustrating how to
/// work with custom types.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct Trade {
  /// The trade's symbol.
  #[serde(rename = "S")]
  pub symbol: String,
  /// The trade's ID.
  #[serde(rename = "i")]
  pub trade_id: u64,
  /// The trade's price.
  #[serde(rename = "p")]
  pub trade_price: u32,
  /// The trade's size.
  #[serde(rename = "s")]
  pub trade_size: u64,
  /// The trade's conditions.
  #[serde(rename = "c")]
  pub conditions: Vec<String>,
  /// The trade's time stamp.
  #[serde(rename = "t")]
  pub timestamp: DateTime<Utc>,
  /// The trade's exchange.
  #[serde(rename = "x")]
  pub exchange: String,
  /// The trade's tape.
  #[serde(rename = "z")]
  pub tape: String,
  /// The trade's update, may be "canceled", "corrected", or
  /// "incorrect".
  #[serde(rename = "u", default)]
  pub update: Option<String>,
}


#[tokio::main]
async fn main() {
    dotenv().unwrap();

    let api_info = ApiInfo::from_env().expect("dotenv files not found");
    let client = Client::new(api_info);
    let account = client.issue::<account::Get>(&()).await.expect("alpaca account could not be loaded");

    //alpaca_module::make_order(&client, "AAPL", order::Side::Buy, order::Amount::quantity(1)).await.unwrap();

    let mut model = ai_module::Model::new_with_apca(&client, account);
    //match model.load_hist().await {
    //    Ok(_)=>{}
    //    Err(e) => {println!("history was not loaded: {}", e)}
    //}
    model.send_message(std::fs::read_to_string("system_prompt.txt")
        .unwrap_or("Don't use markdown in your responses.".to_string()), Role::System
    ).await.unwrap();

    loop {
        let mut input: String = String::new();
        println!();
        print!("> ");
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_line(&mut input).unwrap();
        //let response = model.send(input).await.unwrap();
        let mut chars = input.chars().into_iter();
        if chars.next().unwrap() == '!' {
            match input {
                c if c.starts_with("!save") => {
                    model.save_hist().await.unwrap();
                    println!("history saved");
                }

                c if c.starts_with("!load") => {
                    model.load_hist().await.unwrap();
                    println!("history loaded");
                }

                c if c.starts_with("!debug_hist") => {
                    model.debug_hist().await.unwrap();
                    println!("hist");
                }

                c if c.starts_with("!help") => {
                    println!("!save !load !debug_hist");
                }

                _ => {
                    println!("wrong command: {}", input);
                }
            }

        } else {
            let response = model.send_message(input, Role::User).await.unwrap();
            println!("{}", response.1.concat());
        }
        //println!("raw: {:?}", response.1);
    }
}
