use std::collections::HashMap;
//use anyhow::Ok;
use serde_json::{Value, json};
use std::fmt::Display;
use std::io::{self, Write};
use once_cell::sync::Lazy;
//use std::result::Result::Ok;

use async_openai::{Client, config::OpenAIConfig};

use std::process::Command;

use clap::Parser;
use std::{env, fs, process};

use crate::ui::App;

pub struct Model {
    pub base_url: String,
    pub price: Option<String>,
}

static MODEL_OPTIONS: Lazy<HashMap<&str, Vec<Model>>> = Lazy::new(|| {
    HashMap::from([
        (
            "openrouter",
            vec![
                Model {
                    base_url: String::from(
                        "nvidia/nemotron-3-nano-omni-30b-a3b-reasoning:free",
                    ),
                    price: None,
                },
                Model {
                    base_url: String::from("nex-agi/nex-n2-pro:free"),
                    price: None,
                },
                Model {
                    base_url: String::from("anthropic/claude-3-haiku"),
                    price: Some(String::from("$0.25 / $1.25per 1M")),
                },
                Model {
                    base_url: String::from("deepseek/deepseek-v4-flash"),
                    price: Some(String::from("$0.0983 / $0.1966per 1M")),
                },
            ],
        ),
        (
            "free_the_ai",
            vec![Model {
                base_url: String::from(
                    "kai/nvidia/nemotron-3-nano-omni-30b-a3b-reasoning:free",
                ),
                price: None,
            }],
        ),
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
        io::stdout().flush().unwrap();
        let mut input = String::new();
        #[allow(unused_assignments)]
        io::stdin().read_line(&mut input).expect("failed to read line");
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
    let model: &Model = MODEL_OPTIONS
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
                        "required": ["command"],
                        "properties": {
                            "command": {
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

async fn handle_tool_call(tool_call: &Value, app: &mut App<'_>) {
    let name = tool_call["function"]["name"].as_str().unwrap();
    let mut messages: Vec<Value> = app.messages.clone();
    let mut readable_messages = app.readable_messages.clone();
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
                app.messages = messages;
                app.readable_messages = readable_messages;
                return;
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
                app.messages = messages;
                app.readable_messages = readable_messages;
                return;
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
                app.messages = messages;
                app.readable_messages = readable_messages;
                return;
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
            let phrase = args["command"].as_str().unwrap();
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

        _ => {
            eprintln!("Unknown tool: {}", name);
        }
    }
    app.messages = messages;
    app.readable_messages = readable_messages;
}

//#[tokio::main]
//async fn mainnn() -> Result<(), Box<dyn std::error::Error>> {
//    match dotenvy::from_filename("/home/istipisti113/config/variables/raa.env") {
//        Ok(_a) => {}
//        Err(_e) => {
//            eprintln!(".env file could not be loaded.");
//            return Ok(());
//        }
//    };
//    dotenvy::dotenv().ok();
//
//    let (base_url, api_key, model) = get_config();
//    let client = create_client::<serde_json::Value>(&base_url, &api_key);
//
//    let mut messages: Vec<Value> = vec![];
//    let mut running = true;
//
//    while running{
//        print!("> ");
//        io::stdout().flush().unwrap();
//        let mut input = String::new(); 
//
//        io::stdin().read_line(&mut input).expect("failed to read line");
//        #[allow(unused_assignments)]
//
//    }
//    Ok(())
//}

pub async fn send_message<'a>(app: &'_ mut App<'_>) -> anyhow::Result<String>{
    let input: String = app.input.clone();
    //&(*app).client;
    let client = (*app).client.as_ref().unwrap().clone();
    if input.trim() == "exit" || input.trim()=="quit"{app.running = false; return Ok("Exited".to_owned());}
    app.messages.push(json!({"role": "user", "content": &input.trim()}));
    loop {
        let response: Value = client
            .chat()
            .create_byot(create_byot(&app.messages, &app.model.as_ref().unwrap()))
        .await.unwrap();

        //eprintln!("Logs from your program will appear here!");
        let message = &response["choices"][0]["message"];
        app.messages.push(serde_json::to_value(message).unwrap());

        if let Some(tool_calls) = &message["tool_calls"].as_array() {
            for tool_call in tool_calls.into_iter() {
                handle_tool_call(&tool_call, app).await;
            }
        } else if let Some(content) = message["content"].as_str() {
            //app.readable_messages.push(serde_json::to_value(content).unwrap().to_string());
            //println!("{}", content);
            return Ok(content.to_owned());
        }
    }
}
