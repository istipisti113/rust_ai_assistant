use crate::process::Command;
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

fn get_config() -> (String, String) {
    let base_url = env::var("OPENROUTER_BASE_URL")
        .unwrap_or_else(|_| "https://openrouter.ai/api/v1".to_string());

    let api_key = env::var("OPENROUTER_API_KEY").unwrap_or_else(|_| {
        eprintln!("OPENROUTER_API_KEY is not set");
        process::exit(1);
    });

    (base_url, api_key)
}

fn create_client<T>(base_url: &str, api_key: &str) -> Client<OpenAIConfig>
{
    let config = OpenAIConfig::new()
        .with_api_base(base_url)
        .with_api_key(api_key);

    Client::with_config(config)
}

async fn handle_tool_call(tool_call: &Value, messages: &mut Vec<Value>) {
    let name = tool_call["function"]["name"].as_str().unwrap();
    let args: Value =
        serde_json::from_str(tool_call["function"]["arguments"].as_str().unwrap()).unwrap();

    match name {
        "Read" => {
            let file_path = args["file_path"].as_str().unwrap();
            println!("file read: {}", file_path);
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
            println!("write tool used: {}, {}", file_path, cont);

            std::fs::write(file_path, cont).unwrap();
            messages.push(json!({
                "role": "tool", "tool_call_id": tool_call["id"], "content": cont
            }));
        }
        "Bash" => {
            let cmd = args["command"].as_str().unwrap();
            println!("shell command ran: {}", cmd);
            let output = Command::new("powershell").arg("-c").arg(cmd).output();
            match &output {
                Ok(out) => {
                    let content = String::from_utf8_lossy(&out.stdout).to_string();
                    messages.push(json!({
                        "role": "tool", "tool_call_id": tool_call["id"], "content": content
                    }));
                }
                Err(error) => {
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
    dotenvy::dotenv().ok();
    let args = Args::parse();

    let (base_url, api_key) = get_config();
    let client = create_client::<serde_json::Value>(&base_url, &api_key);

    let mut messages = vec![json!({"role": "user", "content": args.prompt})];

    loop {
        let response: Value = client
            .chat()
            .create_byot(json!({
                "messages": messages,
                "model": "anthropic/claude-3-haiku",
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
            }))
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

    Ok(())
}