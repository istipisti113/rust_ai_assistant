#![allow(clippy::let_unit_value)]

use core::panic;

use serde;
//use apca::data::v2::stream::drive;
//use apca::data::v2::stream::Bar;
//use apca::data::v2::stream::MarketData;
//use apca::data::v2::stream::Quote;
//use apca::data::v2::stream::RealtimeData;
//use apca::data::v2::stream::IEX;
use apca::api::v2::{calendar};
//use apca::Error;
use apca::data::v2::bars::{ListReq, TimeFrame};

use chrono::{DateTime, NaiveDate, Duration, Utc};

use serde::Deserialize;
use serde::Serialize;

/// A trade for an equity.
///
/// This is a user-defined type with additional fields compared to
/// [`apca::data::v2::stream::Trade`] (the default), illustrating how to
/// work with custom types.
#[allow(unused)]
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

use std::sync::Arc;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use tokio::sync::Mutex;
//use anyhow::Ok;
use serde_json::{Value, json};
use std::fmt::Display;
use std::io::{Write};
use once_cell::sync::Lazy;
//use std::result::Result::Ok;

use async_openai::{Client, config::OpenAIConfig};

use std::process::Command;

use clap::Parser;
use std::{env, fs};

use crate::alpaca_module;
use apca::api::v2::order;

pub enum Role {
    User,
    System
}

pub struct ModelOption {
    pub base_url: String,
    pub price: Option<String>,
}

#[allow(unused)]
pub struct Model<'a>{
    messages: Arc<Mutex<Vec<Value>>>,
    readable_messages: Arc<Mutex<Vec<String>>>,
    client: Client<OpenAIConfig>,
    base_url: String,
    model_name: String,
    api_key: String,
    apca_account: Option<apca::api::v2::account::Account>,
    apca_client: Option<&'a apca::Client>,
    memory: Arc<Mutex<Vec<Value>>>
}

impl<'a> Model<'a> {
    pub fn _new() -> Model<'a> {
        let (base_url, api_key, model) = get_config();
        let ai_client: Client<OpenAIConfig> = create_client::<serde_json::Value>(&base_url, &api_key);
        return Model {
            messages: Arc::new(Mutex::new(Vec::new())),
            readable_messages: Arc::new(Mutex::new(Vec::new())),
            client: ai_client,
            api_key: api_key,
            base_url: base_url,
            model_name:model,
            apca_account: None,
            apca_client:None,
            memory: Arc::new(Mutex::new(Vec::new())),
        };
    }

    pub fn new_with_apca(apca_client: &'a apca::Client, apca_account: apca::api::v2::account::Account) -> Model<'a>{
        let (base_url, api_key, model) = get_config();
        let ai_client: Client<OpenAIConfig> = create_client::<serde_json::Value>(&base_url, &api_key);
        return Model {
            messages: Arc::new(Mutex::new(Vec::new())),
            readable_messages: Arc::new(Mutex::new(Vec::new())),
            client: ai_client,
            api_key: api_key,
            base_url: base_url,
            model_name:model,
            apca_account: Some(apca_account),
            apca_client: Some(apca_client),
            memory: Arc::new(Mutex::new(Vec::new())),
        };
    }

    pub async fn load_hist(&mut self) -> anyhow::Result<()>{
        let mut file = File::open("chat_history.json")?;
        let mut hist = String::new();
        file.read_to_string(&mut hist)?;
        let messages = Arc::clone(&self.messages);
        let mut messages_lock = messages.lock().await;
        let mut history: Vec<Value> = serde_json::from_str(&hist)?;
        messages_lock.append(&mut history);
        Ok(())
    }

    pub async fn debug_hist(&self) -> anyhow::Result<()> {
        let hist = Arc::clone(&self.messages);
        let hist_lock : Vec<String> = hist.lock().await.to_vec().into_iter().map(|m|m.to_string()).collect();
        println!("{}",hist_lock.join("\n"));
        Ok(())
    }

    pub async fn save_hist(&self) -> anyhow::Result<()>{
        let mut file = File::create("chat_history.json")?;
        let msg = self.messages.clone();
        let msg_lock: Vec<serde_json::Value> = msg.lock().await.to_vec();
        file.write_all(serde_json::to_string(&msg_lock.into_iter().filter_map(|m|{
            if m["role"].as_str().unwrap() != "system"{
                Some(m)
            } else {None}
        }).collect::<Vec<Value>>())?.to_string().as_bytes())?;
        Ok(())
    }

    async fn handle_tool_call(&self, tool_call: &Value) -> (Vec<Value>, Vec<String>){
        let name = tool_call["function"]["name"].as_str().unwrap();
        let mut messages: Vec<Value> = vec![];
        let mut readable_messages = vec![];
        let args: Value = serde_json::from_str(tool_call["function"]["arguments"].as_str().unwrap()).unwrap();

        match name {
            "Read" => {
                let file_path = args["file_path"].as_str().unwrap();
                println!(">>>> file read: {}", file_path);
                readable_messages.push(format!(">>>> file read: {}", file_path));
                if !is_file_allowed(file_path.to_string()){
                    messages.push(json!({
                        "role": "tool", "tool_call_id": tool_call["id"], "content": format!("File access not allowed '{}'", file_path)
                    }));
                    readable_messages.push(format!("File access not allowed '{}'", file_path));
                    //app.messages = messages;
                    //app.readable_messages = readable_messages;
                    return (messages, readable_messages);
                }

                match fs::read_to_string(file_path) {
                    Ok(contents) => {
                        messages.push(json!({
                            "role": "tool", "tool_call_id": tool_call["id"], "content": contents
                        }));
                    }
                    Err(e) => {
                        eprintln!("Error reading file '{}': {}", file_path, e);
                        messages.push(json!({
                            "role": "tool", "tool_call_id": tool_call["id"], "content": format!("Error reading file '{}': {}", file_path, e)
                        }));
                    }
                }
            }

            "Write" => {
                let file_path = args["file_path"].as_str().unwrap();
                let cont = args["content"].as_str().unwrap();
                println!(">>>> write tool used: {}, {}", file_path, cont);
                readable_messages.push(format!(">>>> write tool used: {}, {}", file_path, cont));

                if !is_file_allowed(file_path.to_string()){
                    //eprintln!("File access not allowed '{}'", file_path);
                    messages.push(json!({
                        "role": "tool", "tool_call_id": tool_call["id"], "content": format!("File access not allowed '{}'", file_path)
                    }));
                    readable_messages.push(format!("File access not allowed '{}'", file_path));
                    //app.messages = messages;
                    //app.readable_messages = readable_messages;
                    return (messages, readable_messages);
                }

                std::fs::write(file_path, cont).unwrap();
                messages.push(json!({
                    "role": "tool", "tool_call_id": tool_call["id"], "content": cont
                }));
            }

            "Bash" => {
                let cmd = args["command"].as_str().unwrap();
                println!(">>>> shell command ran: {}", cmd);
                //readable_messages.push(format!(">>>> shell command ran: {}", cmd));
                if !ALLOWED_CMD.contains(&cmd.split(" ").nth(0).unwrap()) || cmd.contains("..") {
                    //eprintln!("{} is not an allowed command", cmd);
                    messages.push(json!({
                        "role": "tool", "tool_call_id": tool_call["id"], "content": format!("{} is not an allowed command", cmd)
                    }));
                    //readable_messages.push(format!("{} is not an allowed command", cmd));
                    println!("!!!! {} is not an allowed command", cmd);
                    return (messages, readable_messages);
                }
                let output = Command::new("bash").arg("-c").arg(cmd).output();
                match &output {
                    Ok(out) => {
                        let content = String::from_utf8_lossy(&out.stdout).to_string();
                        messages.push(json!({
                            "role": "tool", "tool_call_id": tool_call["id"].as_str().unwrap(), "content": content
                        }));
                    }

                    Err(error) => {
                        messages.push(json!({
                            "role": "tool", "tool_call_id": tool_call["id"].as_str().unwrap(), "content": format!("error: {}", error)
                        }));
                    }
                }
            }

            "Web" => {
                let phrase = args["phrase"].as_str().unwrap();
                println!(">>>> searching for: {}", phrase);
                readable_messages.push(format!("searching for: {}", phrase));
                let query = json!({
                    "query": phrase,
                    "numResults": 10,
                    "type": "auto",
                    "contents": {
                        "highlights": true
                    }
                });
                let mut cmd = Command::new("curl");
                cmd.arg("-X").arg("POST").arg("https://api.exa.ai/search")
                    .arg("--header").arg("content-type: application/json").arg("--header").arg("x-api-key: ".to_owned() + &env::var("EXA_KEY").unwrap())
                    .arg("--data").arg(&query.to_string());

                let web = cmd.output();
                match web {
                    Ok(search) => {
                        let stdout = String::from_utf8_lossy(&search.stdout).to_string();
                        messages.push(json!({ "role": "tool", "tool_call_id": tool_call["id"], "content": stdout }));
                    }
                    Err(e) => {
                        //eprintln!("Error: {}", e);
                        messages.push(json!({ "role": "tool", "tool_call_id": tool_call["id"], "content": format!("Error: {}", e) }));
                        readable_messages.push(format!("Error: {}", e));
                    }
                }
            }

            "make_order" =>{
                let symbol = args["symbol"].as_str().unwrap();
                let side : bool = args["side"].as_bool().unwrap();
                let amount: u64 = args["amount"].as_u64().unwrap();
                let limit_price = args["limit_price"].as_u64();
                let stop_price = args["stop_price"].as_u64();
                let trail_price = args["trail_price"].as_u64();
                let trail_percent = args["trail_percent"].as_u64();
                let order_type = args["order_type"].as_str().unwrap().to_string();

                println!(">>>> order tool called, {}, {}, {}", symbol, side, amount);
                match alpaca_module::make_order(
                    &self.apca_client.as_ref().unwrap(),
                    symbol,
                    if side {order::Side::Buy} else {order::Side::Sell},
                    order::Amount::quantity(amount), limit_price, 
                    stop_price, trail_price, trail_percent, order_type
                ).await {

                    Ok(_) =>{
                        messages.push(json!({ "role": "tool", "tool_call_id": tool_call["id"], "content": 
                            format!("order placed: {}, {}, {}", symbol, if side {"buy"} else {"sell"}, amount)
                        }));
                        readable_messages.push(format!("Order placed: symbol: {}, side: {}, amount: {}", symbol, side, amount));
                    }
                    Err(e) =>{
                        messages.push(json!(
                            { "role": "tool", "tool_call_id": tool_call["id"], "content": format!("Error: {}", e) }));
                        readable_messages.push(format!("FAILED order: symbol: {}, side: {}, amount: {}", symbol, side, amount));
                    }
                }
            }

            "get_market_open_hours" => {
                println!(">>>> fetching market open hours");
                let start = NaiveDate::parse_from_str(args["start"].as_str().unwrap(), "%Y-%m-%d").unwrap();
                let end = NaiveDate::parse_from_str(args["end"].as_str().unwrap(), "%Y-%m-%d").unwrap();

                let request = calendar::ListReq {start,end, _non_exhaustive:()};
                let response:Vec<serde_json::Value>  = self.apca_client.unwrap()
                    .issue::<calendar::List>(&request).await.unwrap().into_iter().map(|day|{
                        json!({
                            "day": day.date,
                            "open": day.open,
                            "close": day.close
                        })
                    }).collect();

                messages.push(json!({ 
                    "role": "tool", "tool_call_id": tool_call["id"].as_str().unwrap(),
                    "content": serde_json::to_string(&response).unwrap() 
                }));
                //readable_messages.push(">>>> market open hours fetched\n".to_string());
            }

            "get_market_data" => {
                let symbol = args["symbol"].as_str().unwrap();
                let timeframe = match args["timeframe"].as_str().unwrap() {
                    "Minute" => TimeFrame::OneMinute,
                    "Hour" =>  TimeFrame::OneHour,
                    "Day" => TimeFrame::OneDay,
                    _ => TimeFrame::OneMinute
                };
                //readable_messages.push(format!(">>>> market data requested for {}", symbol));
                println!(">>>> market data tool called for {}", symbol);
                //let end = Utc.with_ymd_and_hms(2026, 11, 6, 21, 0, 0).unwrap();
                let end = match args["end"].as_str() {
                    Some(s) => DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc),
                    None => Utc::now()
                };

                let start = match args["start"].as_str() {
                    Some(s) => DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc),
                    None => end - Duration::minutes(15)
                };

                let request = ListReq {
                    symbol: symbol.to_string(),
                    start: start,
                    end: end,
                    limit: Some(15),
                    timeframe: timeframe,
                    adjustment: None,
                    feed: Some(apca::data::v2::Feed::IEX),
                    page_token: None,
                    _non_exhaustive: ()
                };

                //let mut data: Vec<Value> = vec![];
                let data : Vec<serde_json::Value> = match self.apca_client {
                    Some(c) => {
                        let bars = c.issue::<apca::data::v2::bars::List>(&request).await.unwrap();
                        //println!("bars: {:?}", bars.bars);
                        bars.bars.into_iter().map(|bar|{
                            json!({
                                "time": bar.time,
                                "open": bar.open,
                                "close": bar.close,
                                "high": bar.high,
                                "low": bar.low,
                                "volume": bar.volume,
                                "weighted_average": bar.weighted_average,
                            })
                        }).collect::<Vec<Value>>()
                    }
                    None => {
                        println!("model was created without apca");
                        vec![json!({"error":"model cant access apca client"})]
                    }
                };

                let data_string = serde_json::to_string(&data).unwrap();
                messages.push(json!(
                    { "role": "tool", "tool_call_id": tool_call["id"].as_str().unwrap(), "content": data_string }));
                readable_messages.push(format!("market data fetched for symbol: {}\n\n", symbol));
            }

            "python" => {
                let source_code : String = args["code"].as_str().unwrap().to_string();
                let output = Command::new("python").arg("-c").arg(&source_code).output();
                println!(">>>> python");
                match &output {
                    Ok(out) => {
                        let content = String::from_utf8_lossy(&out.stdout).to_string();
                        messages.push(json!({
                            "role": "tool", "tool_call_id": tool_call["id"].as_str().unwrap(), "content": content
                        }));
                    }

                    Err(error) => {
                        messages.push(json!({
                            "role": "tool", "tool_call_id": tool_call["id"].as_str().unwrap(), "content": format!("error: {}", error)
                        }));
                    }
                }

            }

            _ => {
                eprintln!("Unknown tool: {}", name);
            }
        }
        (messages, readable_messages)
    }

    pub async fn send_message<'b>(
        &self,
        message: String,
        //messagess: &mut MutexGuard<'_, Vec<Value>>,
        role: Role,
    )-> anyhow::Result<(Vec<Value>, Vec<String>)>{
        let mut returning: (Vec<Value>, Vec<String>) = (vec![], vec![]);

        let messages = Arc::clone(&self.messages);
        let mut messages_lock = messages.lock().await;
        let readable = Arc::clone(&self.readable_messages);
        let mut readable_lock= readable.lock().await;

        let input_message = json!({"role": match role {
            Role::User => "user",
            Role::System => "system"
        }, "content": &message});
        returning.0.append(&mut messages_lock);
        returning.0.push(input_message.clone());
        //messages_lock.push(input_message);
        //returning.1.push(message.clone());
        //readable_lock.push(message);

        loop {
            let response: Value = self.client
                .chat()
                .create_byot(create_byot(&returning.0, &self.model_name))
                .await.unwrap_or_else(|error|{
                    match error {
                        async_openai::error::OpenAIError::JSONDeserialize(_,ref message) => {
                            let message = &serde_json::from_str::<Value>(&message).unwrap()["error"]["message"].to_string();
                            if message == "Rate limit exceeded: free-models-per-min. " {
                                println!("Rate limit exceeded")
                            } else {
                                eprintln!("message: {}, error: {}", message, error);
                            }
                        }
                        _ => {
                            eprintln!("{:?}", error);
                        }
                    }
                    panic!("|{:?}|", messages_lock);
                });

            let message = response["choices"][0]["message"].clone();

            if let Some(tool_calls) = &message["tool_calls"].as_array() {
                for tool_call in tool_calls.into_iter() {
                    let (mut raw, mut read) = self.handle_tool_call(&tool_call).await;
                    returning.0.append(&mut raw);
                    returning.1.append(&mut read);
                }
            } else if let Some(content) = message["content"].as_str() {
                returning.1.push(content.to_string());
                returning.0.push(json!({
                    "role": "assistant",
                    "content": content
                }));
                break ;
            }
        }

        //let mut ans_parts = Vec::new();
        for chunk in &returning.1{
            for line in chunk.split('\n'){
                let a = line.to_owned();
                readable_lock.push(a.clone());
                //ans_parts.push(a);
            }
        }
        //ans_parts.concat();

        messages_lock.append(&mut returning.0);
        return Ok(returning);
    }
}

static MODEL_OPTIONS: Lazy<HashMap<&str, Vec<ModelOption>>> = Lazy::new(|| {
    HashMap::from([
        (
            "openrouter",
            vec![
                ModelOption {
                    base_url: String::from(
                        "nvidia/nemotron-3-nano-omni-30b-a3b-reasoning:free",
                    ),
                    price: None,
                },
                ModelOption {
                    base_url: String::from("nex-agi/nex-n2-pro:free"),
                    price: None,
                },
                ModelOption {
                    base_url: String::from("anthropic/claude-3-haiku"),
                    price: Some(String::from("$0.25 / $1.25per 1M")),
                },
                ModelOption {
                    base_url: String::from("deepseek/deepseek-v4-flash"),
                    price: Some(String::from("$0.0983 / $0.1966per 1M")),
                },
            ],
        ),
        (
            "free_the_ai",
            vec![
                ModelOption {
                    base_url: String::from("kai/nvidia/nemotron-3-nano-omni-30b-a3b-reasoning:free"),
                    price: None,
                },
                ModelOption {
                    base_url: String::from("fta/zai/glm-5.3"),
                    price: None
                },
                ModelOption {
                    base_url: String::from("fta/olf/gemma4:31b"),
                    price: None
                },
                ModelOption {
                    base_url: String::from("fta/olm/kimi-k2.7-code"),
                    price: None
                }
            ],
        ),
        (
            "groq",
            vec![
                ModelOption {
                    base_url: String::from("openai/gpt-oss-20b"),
                    price: None
                }
            ]
       )
    ])
});

static CREDENTIALS: Lazy<HashMap<&str, Vec<String>>> = Lazy::new(|| {
    HashMap::from([
        (
            "openrouter",
            vec![
                env::var("OPENROUTER_BASE_URL")
                    .unwrap_or("error in reading OPENROUTER_BASE_URL".to_owned()),
                env::var("OPENROUTER_API_KEY")
                    .unwrap_or("error in reading OPENROUTER_API_KEY".to_owned()),
            ],
        ),
        (
            "free_the_ai",
            vec![
                env::var("FREE_THE_AI_BASE_URL")
                    .unwrap_or("error in reading FREE_THE_AI_BASE_URL".to_owned()),
                env::var("FREE_THE_AI")
                    .unwrap_or("error in reading FREE_THE_AI".to_owned()),
            ],
        ),
        (
            "groq",
            vec![
                env::var("GROQ_BASE_URL").unwrap(),
                env::var("GROQ_API_KEY").unwrap(),
            ]
    )
    ])
});

fn option_selector<T: Display>(options: Vec<T>, question: &str) -> T {
    let len = options.len();
    loop {
        println!("{}", question);
        for i in 0..len {
            println!("{}: {}", i, options[i]);
        }
        println!();
        print!("> ");
        std::io::stdout().flush().unwrap();
        let mut input = String::new();
        #[allow(unused_assignments)]
        std::io::stdin().read_line(&mut input).expect("failed to read line");
        match input.trim().parse::<usize>() {
            Ok(num) => {
                if len > num {
                    return options.into_iter().nth(num).unwrap();
                } else {
                    println!("number must be in the list!");
                }
            }
            Err(_) => {
                eprintln!("invalid input!")
            }
        }
    }
}

pub fn get_config() -> (String, String, String) {
    let selected: &str = *option_selector(MODEL_OPTIONS.keys().collect(), "select the provider");
    let selected_provider: &Vec<String> = CREDENTIALS.get(selected).unwrap();

    let model_names: Vec<&str> = MODEL_OPTIONS
        .get(selected)
        .unwrap()
        .iter()
        .map(|x| x.base_url.as_ref())
        .collect();
    let model_name: &str = option_selector(model_names, "select the model");
    let model: &ModelOption = MODEL_OPTIONS
        .get(selected)
        .unwrap()
        .iter()
        .find(|x| x.base_url == model_name)
        .unwrap();

    println!("selected: {}, costs: {}", &model.base_url, &model.price.clone().unwrap_or("free".to_owned()));

    (
        selected_provider[0].to_owned(),
        selected_provider[1].to_owned(),
        model.base_url.to_owned(),
    )
}

pub fn create_byot(messages: &Vec<Value>, model: &str) -> Value{
    json!({
        "messages": messages,
        "model": model,
        "tools": [
            {
                "type": "function",
                "function": {
                    "name": "Web",
                    "description": "Search the web",
                    "parameters": {
                        "type": "object",
                        "required": ["phrase"],
                        "properties": {
                            "phrase": {
                                "type": "string",
                                "description": "The phrase to search the web with"
                            }
                        }
                    }
                }
            },
            {
                "type": "function",
                "function": {
                    "name": "Bash",
                    "description": "Execute a shell command",
                    "parameters": {
                        "type": "object",
                        "required": ["command"],
                        "properties": {
                            "command": {
                                "type": "string",
                                "description": "The command to execute"
                            }
                        }
                    }
                }
            },
            {
                "type": "function",
                "function": {
                    "name": "Write",
                    "description": "Write content to a file",
                    "parameters": {
                        "type": "object",
                        "required": ["file_path", "content"],
                        "properties": {
                            "file_path": {
                                "type": "string",
                                "description": "The path of the file to write to"
                            },
                            "content": {
                                "type": "string",
                                "description": "The content to write to the file"
                            }
                        }
                    }
                }
            },
            {
                "type": "function",
                "function": {
                    "name": "Read",
                    "description": "Read and return the contents of a file",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "file_path": {
                                "type": "string",
                                "description": "The path to the file to read"
                            }
                        },
                        "required": ["file_path"]
                    }
                }
            },

            {
                "type": "function",
                "function": {
                    "name": "make_order",
                    "description": "make order of the given symbol. use when the user wants to buy or sell stocks",
                    "parameters": {
                        "type": "object",
                        "required": ["symbol", "side", "amount", "order_type"],
                        "properties": {
                            "symbol": {
                                "type": "string",
                                "description": "the symbol of the stock to make order"
                            },
                            "side": {
                                "type": "boolean",
                                "description": "if side is true it is buy side, if it is false it is sell"
                            },
                            "amount": {
                                "type": "number",
                                "description": "amount of the symbol subject to the order. it is the amount of stock not amount of money"
                            },
                            "order_type": {
                                "type": "string",
                                "description":"it is always one of [market, limit, stop, stop_limit, trailing_stop]. inputs should be based on the description in order_types.md",
                            },
                            "limit_price": {
                                "type": "number",
                                "description": "it is used for limit and stop_limit orders. see order_types.md"
                            },
                            "stop_price": {
                                "type": "number",
                                "description": "it is used for stop and stop_limit orders. see order_types.md"
                            },
                            "trail_price": {
                                "type": "number",
                                "description": "used for trailing_stop orders. see order_types.md"
                            },
                            "trail_percent": {
                                "type": "number",
                                "description": "used for trailing_stop orders. see order_types.md"
                            }
                        }
                    }
                }
            },
            {
                "type": "function",
                "function": {
                    "name": "get_market_data",
                    "description": "get the market data of some symbol. before you use this tool, get the time and date first.",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "symbol": { "type": "string", "description": "the symbol of the stock to make order" },
                            "start":    { "type": "string", "format": "date-time",
                                "description": "RFC 3339 UTC start time, e.g. 2026-11-06T20:45:00Z" },
                            "end":      { "type": "string", "format": "date-time",
                                "description": "RFC 3339 UTC end time. Omit for now." },
                            "timeframe": { "type": "string", "format": "string",
                                        "description": "timeframe for the candles, it is either Minute, Hour, Day"}
                        },
                        "required": ["symbol"]
                    }
                }
            },
            

            {
                "type": "function",
                "function": {
                    "name": "get_market_open_hours",
                    "description": "get the open hours of the market. When the tool's return is empty, the market is closed",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "start": {
                                "type": "string",
                                "format": "date",
                                "description": "starting day, formatted: YYYY-MM-DD"
                            },
                            "end": {
                                "type": "string",
                                "format": "date",
                                "description": "starting day, formatted: YYYY-MM-DD"
                            }
                        },
                        "required": ["start", "end"]
                    }
                }
            },

            {
                "type": "function",
                "function": {
                    "name": "python",
                    "description": "run python scripts",
                    "parameters": {
                        "type": "object",
                        "properties": {
                            "code": {
                                "type": "string",
                                "description": "python source code to be ran"
                            },
                        },
                        "required": ["code"]
                    }
                }
            },
        ]
    })
}

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    #[arg(short = 'p', long)]
    prompt: String,
}

static ALLOWED_CMD: [&str; 4]= [
    "ls",
    "pwd",
    "cargo",
    "date",
];

pub fn create_client<T>(base_url: &str, api_key: &str) -> Client<OpenAIConfig> {
    let config = OpenAIConfig::new()
        .with_api_base(base_url)
        .with_api_key(api_key);
    Client::with_config(config)
}

fn is_file_allowed(path: String) -> bool {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");
    let abs_path = if std::path::Path::new(&path).is_absolute() {
        std::path::Path::new(&path).to_path_buf()
    } else {
        current_dir.join(&path).to_path_buf()
    };
    if !abs_path.starts_with(current_dir) || path.contains("..") {
        return false;
    }
    true
}
