use core::panic;

use std::sync::Arc;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;
use tokio::sync::MutexGuard;
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

pub struct ModelOption {
    pub base_url: String,
    pub price: Option<String>,
}

pub struct Model{
    messages: Arc<Mutex<Vec<Value>>>,
    readable_messages: Arc<Mutex<Vec<String>>>,
    client: Client<OpenAIConfig>,
    base_url: String,
    model_name: String,
    api_key: String,
    apca_account: apca::api::v2::account::Account,
    apca_client: apca::Client,
}

impl Model {
    pub fn new(apca_client: apca::Client, apca_account: apca::api::v2::account::Account) -> Model{
        let (base_url, api_key, model) = get_config();
        let ai_client = create_client::<serde_json::Value>(&base_url, &api_key);
        return Model {
            messages: Arc::new(Mutex::new(Vec::new())),
            readable_messages: Arc::new(Mutex::new(Vec::new())),
            client: ai_client,
            api_key: api_key,
            base_url: base_url,
            model_name:model,
            apca_account: apca_account,
            apca_client:apca_client,
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
    
    pub async fn save_hist(&self) -> anyhow::Result<()>{
        let mut file = File::create("chat_history.json")?;
        let msg = self.messages.clone();
        let msg_lock: Vec<serde_json::Value> = msg.lock().await.to_vec();
        file.write_all(serde_json::to_string(&msg_lock)?.to_string().as_bytes())?;
        Ok(())
    }

    //pub async fn send(&mut self, input: String) -> anyhow::Result<(String, Vec<Value>), String> {
    //    let input_message = json!({"role":"user", "content":input});
    //    //let mut messages_lock = self.messages.lock().await;
    //    let messages = Arc::clone(&self.messages);
    //    let mut messages_lock = messages.lock().await;
    //    let readable = Arc::clone(&self.readable_messages);
    //    let mut readable_lock = readable.lock().await;
    //    messages_lock.push(input_message);
    //    readable_lock.push(input);

    //    let new_raw;

    //    let answer: String = match self.send_message(
    //        &mut messages_lock,
    //        &self.model_name
    //    ).await {
    //        Ok((mut raw,readable)) => {
    //            messages_lock.append(&mut raw);
    //            let mut ans_parts = Vec::new();

    //            for asdf in &readable{
    //                for line in asdf.split("\\n"){
    //                    let a = line.to_owned();
    //                    readable_lock.push(a.clone());
    //                    ans_parts.push(a);
    //                }
    //            }
    //            new_raw = raw;
    //            ans_parts.concat()

    //        }
    //        Err(_error) => {
    //            return Err(String::from("error"));
    //        }
    //    };
    //    Ok((answer, new_raw))
    //}

    async fn handle_tool_call(&self, tool_call: &Value) -> (Vec<Value>, Vec<String>){
        let name = tool_call["function"]["name"].as_str().unwrap();
        let mut messages: Vec<Value> = vec![];
        let mut readable_messages = vec![];
        let args: Value = serde_json::from_str(tool_call["function"]["arguments"].as_str().unwrap()).unwrap();

        match name {
            "Read" => {
                let file_path = args["file_path"].as_str().unwrap();
                //println!(">>>> file read: {}", file_path);
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
                //println!(">>>> write tool used: {}, {}", file_path, cont);
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
                //println!(">>>> shell command ran: {}", cmd);
                readable_messages.push(format!(">>>> shell command ran: {}", cmd));
                if !ALLOWED_CMD.contains(&cmd.split(" ").nth(0).unwrap()) || cmd.contains("..") {
                    //eprintln!("{} is not an allowed command", cmd);
                    messages.push(json!({
                        "role": "tool", "tool_call_id": tool_call["id"], "content": format!("{} is not an allowed command", cmd)
                    }));
                    readable_messages.push(format!("{} is not an allowed command", cmd));
                    return (messages, readable_messages);
                }
                let output = Command::new("bash").arg("-c").arg(cmd).output();
                match &output {
                    Ok(out) => {
                        let content = String::from_utf8_lossy(&out.stdout).to_string();
                        messages.push(json!({
                            "role": "tool", "tool_call_id": tool_call["id"], "content": content
                        }));
                    }

                    Err(_error) => {
                        messages.push(json!({
                            "role": "tool", "tool_call_id": tool_call["id"], "content": "content: ".to_owned() + &format!("{}", &output.unwrap_err())
                    }));
                    }
                }
            }

            "Web" => {
                let phrase = args["phrase"].as_str().unwrap();
                //println!("searching for: {}", phrase);
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
                match alpaca_module::make_order(
                    &self.apca_client, symbol,
                    if side {order::Side::Buy} else {order::Side::Sell},
                    order::Amount::quantity(amount)
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

            _ => {
                eprintln!("Unknown tool: {}", name);
            }
        }
        (messages, readable_messages)
    }

    pub async fn send_message<'a>(
        &self,
        message: String,
        //messagess: &mut MutexGuard<'_, Vec<Value>>,
    )-> anyhow::Result<(Vec<Value>, Vec<String>)>{
        let mut returning: (Vec<Value>, Vec<String>) = (vec![], vec![]);

        let messages = Arc::clone(&self.messages);
        let mut messages_lock: Vec<Value> = messages.lock().await.to_vec();
        let readable = Arc::clone(&self.readable_messages);
        let mut readable_lock: Vec<String> = readable.lock().await.to_vec();

        let input_message = json!({"role":"user", "content":message.clone()});
        //returning.0.push(input_message);
        messages_lock.push(input_message);
        //returning.1.push(message);

        loop {
            let response: Value = self.client
                .chat()
                .create_byot(create_byot(&messages_lock, &self.model_name))
                .await.unwrap_or_else(|error|{
                    eprintln!("{:?}", error);
                    panic!("|{:?}|", messages_lock);
                });

            let message = &response["choices"][0]["message"];
            returning.0.push(serde_json::to_value(message).unwrap());

            if let Some(tool_calls) = &message["tool_calls"].as_array() {
                for tool_call in tool_calls.into_iter() {
                    let (mut raw, mut read) = self.handle_tool_call(&tool_call).await;
                    returning.0.append(&mut raw);
                    returning.1.append(&mut read);
                }
            } else if let Some(content) = message["content"].as_str() {
                returning.1.push(serde_json::to_value(content).unwrap().to_string());
                break ;
            }
        }

        //let mut ans_parts = Vec::new();
        for asdf in &returning.1{
            for line in asdf.split("\\n"){
                let a = line.to_owned();
                readable_lock.push(a.clone());
                //ans_parts.push(a);
            }
        }
        //ans_parts.concat();

        messages_lock.append(&mut returning.0.clone());
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
                            }
                        },
                        "required": ["symbol", "side", "amount"]
                    }
                }
            }
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
    "cargo check",
    "cargo check 2>&1",
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
