mod config;

use crate::process::Command;
use std::io::{self, Write};
use async_openai::{Client, config::OpenAIConfig};
use clap::Parser;
use serde_json::{Value, json};
use std::{env, fs, process};

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

fn create_client<T>(base_url: &str, api_key: &str) -> Client<OpenAIConfig> {
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

async fn handle_tool_call(tool_call: &Value, messages: &mut Vec<Value>) {
    let name = tool_call["function"]["name"].as_str().unwrap();
    let args: Value =
    serde_json::from_str(tool_call["function"]["arguments"].as_str().unwrap()).unwrap();

    match name {
        "Read" => {
            let file_path = args["file_path"].as_str().unwrap();
            println!(">>>> file read: {}", file_path);
            if !is_file_allowed(file_path.to_string()){
                eprintln!("File access not allowed '{}'", file_path);
                messages.push(json!({
                    "role": "tool", "tool_call_id": tool_call["id"], "content": format!("File access not allowed '{}'", file_path)
                }));
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
            println!(">>>> write tool used: {}, {}", file_path, cont);

            if !is_file_allowed(file_path.to_string()){
                eprintln!("File access not allowed '{}'", file_path);
                messages.push(json!({
                    "role": "tool", "tool_call_id": tool_call["id"], "content": format!("File access not allowed '{}'", file_path)
                }));
                return;
            }

            std::fs::write(file_path, cont).unwrap();
            messages.push(json!({
                "role": "tool", "tool_call_id": tool_call["id"], "content": cont
            }));
        }

        "Bash" => {
            let cmd = args["command"].as_str().unwrap();
            println!(">>>> shell command ran: {}", cmd);
            if !ALLOWED_CMD.contains(&cmd.split(" ").nth(0).unwrap()) || cmd.contains("..") {
                eprintln!("{} is not an allowed command", cmd);
                messages.push(json!({
                    "role": "tool", "tool_call_id": tool_call["id"], "content": format!("{} is not an allowed command", cmd)
                }));
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
            println!("searching for: {}", phrase);
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
                    eprintln!("Error: {}", e);
                    messages.push(json!({ "role": "tool", "tool_call_id": tool_call["id"], "content": format!("Error: {}", e) }));
                }
            }
        }

        _ => {
            eprintln!("Unknown tool: {}", name);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    match dotenvy::from_filename("/home/istipisti113/config/variables/raa.env") {
        Ok(_a) => {}
        Err(_e) => {
            eprintln!(".env file could not be loaded.");
            return Ok(());
        }
    };
    dotenvy::dotenv().ok();

    let (base_url, api_key, model) = config::get_config();
    let client = create_client::<serde_json::Value>(&base_url, &api_key);

    let mut messages = vec![];
    let mut running = true;

    while running{
        print!("> ");
        io::stdout().flush().unwrap();
        let mut input = String::new(); 

        #[allow(unused_assignments)]
        if input.trim() == "exit" || input.trim()=="quit"{running = false; break;}
        io::stdin().read_line(&mut input).expect("failed to read line");
        messages.push(json!({"role": "user", "content": &input.trim()}));
        loop {
            let response: Value = client
                .chat()
                .create_byot(config::create_byot(&messages, &model))
            .await?;

            eprintln!("Logs from your program will appear here!");
            let message = &response["choices"][0]["message"];
            messages.push(serde_json::to_value(message).unwrap());

            if let Some(tool_calls) = &message["tool_calls"].as_array() {
                for tool_call in tool_calls.into_iter() {
                    handle_tool_call(&tool_call, &mut messages).await;
                }
            } else if let Some(content) = message["content"].as_str() {
                println!("{}", content);
                break;
            }
        }
    }
    Ok(())
}
