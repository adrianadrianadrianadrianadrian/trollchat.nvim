use anyhow::anyhow;
use axum::{
    Router,
    extract::{Path, State},
    routing::post,
};
use difference::{Changeset, Difference};
use futures::future::join_all;
use rand::seq::IteratorRandom;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{
    Mutex,
    mpsc::{self, UnboundedReceiver, UnboundedSender},
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let openapi_key = std::env::var("OPENAI_API_KEY")?;
    let (tx, rx) = mpsc::unbounded_channel::<ChatResponse>();
    let state = Arc::new(Mutex::new(AppState {
        response_sender: tx,
        buffers: HashMap::new(),
        http_client: Client::new(),
        openapi_key,
        agents: vec![
            Agent {
                username: "yamom12".to_string(),
                agent_type: AgentType::Troll,
            },
            Agent {
                username: "coderg1rl22".to_string(),
                agent_type: AgentType::Normal,
            },
            Agent {
                username: "masdbuddy".to_string(),
                agent_type: AgentType::Helpful,
            },
            Agent {
                username: "galadiccc".to_string(),
                agent_type: AgentType::Troll,
            },
            Agent {
                username: "bigd1kboi".to_string(),
                agent_type: AgentType::Troll,
            },
            Agent {
                username: "starwarslover".to_string(),
                agent_type: AgentType::Helpful,
            },
            Agent {
                username: "cheater2g1234".to_string(),
                agent_type: AgentType::Normal,
            },
        ],
    }));

    handle_chat_responses(rx);

    let app = Router::new()
        .route("/{buffer_name}", post(read_buffer))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:2000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}

struct AppState {
    response_sender: UnboundedSender<ChatResponse>,
    buffers: HashMap<String, String>,
    agents: Vec<Agent>,
    http_client: Client,
    openapi_key: String,
}

struct ChatResponse {
    pub username: String,
    pub message: String,
}

#[derive(PartialEq, Clone)]
enum AgentType {
    Troll,
    Helpful,
    Normal,
}

impl Into<String> for &AgentType {
    fn into(self) -> String {
        match self {
            AgentType::Troll => "troll",
            AgentType::Helpful => "helpful",
            AgentType::Normal => "normal",
        }
        .to_string()
    }
}

#[derive(Clone)]
struct Agent {
    pub username: String,
    pub agent_type: AgentType,
}

#[derive(Deserialize, Debug)]
struct DeepSeekResponse {
    choices: Vec<DeepSeekChoice>,
}

#[derive(Deserialize, Debug)]
struct DeepSeekChoice {
    message: DeepSeekMessage,
}

#[derive(Serialize, Deserialize, Debug)]
struct DeepSeekMessage {
    role: String,
    content: String,
}

#[derive(Serialize)]
struct DeepSeekRequest {
    model: String,
    stream: bool,
    messages: Vec<DeepSeekMessage>,
}

fn handle_chat_responses(mut rx: UnboundedReceiver<ChatResponse>) {
    tokio::spawn(async move {
        while let Some(r) = rx.recv().await {
            println!("[{}]: {}\n", r.username, r.message);
        }
    });
}

async fn get_chat(
    http_client: Client,
    openapi_key: &str,
    code: &str,
    agent: Agent,
) -> anyhow::Result<String> {
    let request = DeepSeekRequest {
        model: "deepseek-chat".to_string(),
        stream: false,
        messages: vec![DeepSeekMessage {
            role: "user".to_string(),
            content: format!(
                "{} this code {}. Respond to me as a {} twitch chat user and keep the response under 50 words please.",
                {
                    if agent.agent_type == AgentType::Troll {
                        "Roast"
                    } else {
                        "Help me fix"
                    }
                },
                code,
                Into::<String>::into(&agent.agent_type)
            ),
        }],
    };

    let mut response = http_client
        .post("https://api.deepseek.com/chat/completions")
        .header("content-type", "application/json")
        .bearer_auth(openapi_key)
        .body(serde_json::to_string(&request)?)
        .send()
        .await?
        .json::<DeepSeekResponse>()
        .await?;

    Ok(response
        .choices
        .pop()
        .ok_or(anyhow!("AI pooped itself.."))?
        .message
        .content)
}

async fn read_buffer(
    Path(buffer_name): Path<String>,
    State(app_state): State<Arc<Mutex<AppState>>>,
    buffer_content: String,
) {
    let mut state = app_state.lock().await;
    if let Some(previous_buffer) = state.buffers.get(&buffer_name) {
        let changeset = Changeset::new(previous_buffer, &buffer_content, "\n");
        let change = get_additions(changeset.diffs).pop();
        let sender = state.response_sender.clone();
        let agents = state.agents.clone();
        let http_client = state.http_client.clone();
        let agent_count = state.agents.len();
        let openapi_key = state.openapi_key.clone();

        tokio::spawn(async move {
            if let Some(c) = change {
                let futs = agents
                    .into_iter()
                    .choose_multiple(&mut rand::rng(), agent_count / 2)
                    .into_iter()
                    .map(|a| async {
                        let response =
                            get_chat(http_client.clone(), &openapi_key, &c, a.clone()).await;
                        if let Ok(message) = response {
                            _ = sender.send(ChatResponse {
                                username: a.username,
                                message,
                            });
                        };
                    })
                    .collect::<Vec<_>>();

                _ = join_all(futs).await;
            }
        });
    }

    state.buffers.insert(buffer_name, buffer_content);
}

fn get_additions(diffs: Vec<Difference>) -> Vec<String> {
    diffs
        .into_iter()
        .filter_map(|d| match d {
            Difference::Add(a) => Some(a),
            _ => None,
        })
        .collect()
}
