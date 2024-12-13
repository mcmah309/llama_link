mod errors;

use core::panic;
use error_set::ResultContext;
use errors::{CompletionError, ToolCallError};
use llmtoolbox::ToolBox;
use reqwest::Client;
use reqwest_eventsource::{Event, EventSource};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio_stream::StreamExt;

#[derive(Serialize, bon::Builder)]
pub struct Config {
    n_predict: Option<usize>,
    temperature: Option<f32>,
    top_k: Option<usize>,
    top_p: Option<f32>,
    stop: Option<Vec<String>>,
    // json_schema: Value,
}

#[derive(Deserialize, Debug)]
struct CompletionResponse {
    content: Option<String>,
    stop: Option<bool>,
}

pub struct LlamaLink {
    client: Client,
    completion_url: String,
    request_config: Map<String, Value>,
}

pub type CompletionStream = std::pin::Pin<Box<dyn tokio_stream::Stream<Item = String> + Send>>;

impl LlamaLink {
    pub fn new(url: &str, request_config: Config) -> Self {
        let request_config = match serde_json::to_value(request_config).unwrap() {
            Value::Object(map) => map,
            _ => panic!("RequestConfig is not an object"),
        };
        Self {
            client: Client::new(),
            completion_url: format!("{url}/completion"),
            request_config,
        }
    }

    pub async fn completion(&self, prompt: String) -> Result<String, CompletionError> {
        let mut json = self.request_config.clone();
        json.insert("prompt".to_owned(), Value::String(prompt));
        let json = Value::Object(json);

        let response = self
            .client
            .post(&self.completion_url)
            .json(&json)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(CompletionError::Api {
                issue: format!("HTTP Error: {}", response.status()),
            });
        }

        let response_body: CompletionResponse = response.json().await?;
        response_body.content.ok_or_else(|| CompletionError::Api {
            issue: "No `content` field in reposne body".to_owned(),
        })
    }

    pub async fn tool_call<O, E>(
        &self,
        prompt: String,
        toolbox: &ToolBox<O, E>,
    ) -> Result<Result<O, E>, ToolCallError> {
        let mut json = self.request_config.clone();
        json.insert("prompt".to_owned(), Value::String(prompt));
        json.insert(
            "json_schema".to_owned(),
            Value::Object(toolbox.schema().clone()),
        );
        let json = Value::Object(json);

        let response = self
            .client
            .post(&self.completion_url)
            .json(&json)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(ToolCallError::Api {
                issue: format!("HTTP Error: {}", response.status()),
            });
        }

        let response_body: CompletionResponse = response.json().await?;
        let content = response_body.content.ok_or_else(|| ToolCallError::Api {
            issue: "No `content` field in reposne body".to_owned(),
        })?;
        println!("Content: {}", &content);
        let tool_call = serde_json::from_str(&content).unwrap(); //todo
        let tool_call_result: Result<Result<O, E>, ToolCallError> =
            toolbox.call(tool_call).await.map_err(|error| error.into());
        tool_call_result
    }

    pub fn completion_stream(&self, prompt: String) -> Option<CompletionStream> {
        let mut json = self.request_config.clone();
        json.insert("prompt".to_owned(), Value::String(prompt));
        json.insert("stream".to_owned(), Value::Bool(true));
        let json = Value::Object(json);

        let request = self.client.post(&self.completion_url).json(&json);

        // Why SSE: https://github.com/ggerganov/llama.cpp/blob/89d604f2c87af9db657d8a27a1528bc4b7579c29/examples/server/README.md?plain=1#L450
        let es = EventSource::new(request).consume_with_error(|_| {
            "Could not create event source for SSE in completion stream".to_owned()
        })?;
        let stream = es
            .map(|event| match event {
                Ok(Event::Open) => {
                    #[cfg(feature = "tracing")]
                    tracing::trace!("Completion stream SSE connection open.");
                    Some(String::new())
                }
                Ok(Event::Message(message)) => {
                    let response = serde_json::from_str::<CompletionResponse>(&message.data);
                    match response {
                        Ok(response) => {
                            if response.stop.unwrap_or(false) {
                                #[cfg(feature = "tracing")]
                                tracing::trace!("Completion stream recieved stop");
                                return None;
                            }
                            Some(response.content.unwrap_or_else(|| String::new()))
                        }
                        #[allow(unused_variables)]
                        Err(e) => {
                            #[cfg(feature = "tracing")]
                            tracing::error!("Error in completion stream: {:?}", e);
                            None
                        }
                    }
                }
                Err(err) => {
                    if matches!(err, reqwest_eventsource::Error::StreamEnded) {
                        #[cfg(feature = "tracing")]
                        tracing::trace!("Completion stream ended.");
                        return None;
                    }
                    #[cfg(feature = "tracing")]
                    tracing::error!("Error in completion stream: {}", err);
                    None
                }
            })
            .take_while(|e| e.is_some())
            .filter_map(|e| e);
        Some(Box::pin(stream))
    }
}
