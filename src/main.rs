//use tokio::sync::Mutex;

use std::str::Chars;

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

#[tokio::main]
async fn main() {
    dotenv().unwrap();

    let api_info = ApiInfo::from_env().expect("dotenv files not found");
    let client = Client::new(api_info);
    let account = client.issue::<account::Get>(&()).await.expect("alpaca account could not be loaded");

    //alpaca_module::make_order(&client, "AAPL", order::Side::Buy, order::Amount::quantity(1)).await.unwrap();

    let mut model = ai_module::Model::new(client, account);
    match model.load_hist().await {
        Ok(_)=>{}
        Err(e) => {println!("history was not loaded: {}", e)}
    }

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
                c if c.starts_with("save") => {
                    model.save_hist().await.unwrap();
                }
                _ => {

                }
            }

        } else {
            let response = model.send_message(input).await.unwrap();
            println!("{}", response.1.concat());
        }
        //println!("raw: {:?}", response.1);
    }
}

fn find_matching_prefix<'a>(text: &str, options: &'a [&str]) -> Option<&'a str> {
    // Iterate through the options and find the first one that matches the beginning of the text
    options.iter().find(|&&option| text.starts_with(option)).copied()
}
