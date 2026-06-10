use std::collections::HashMap;
use serde_json::{Value, json};
use std::fmt::Display;
use std::io::{self, Write};
use once_cell::sync::Lazy;
use std::env;

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

