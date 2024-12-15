mod errors;

use core::panic;
use errors::LlamaError;
use reqwest::{Client, Error};
use reqwest_eventsource::{Event, EventSource};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::io::{self, Write};
use tokio_stream::StreamExt;

#[derive(Serialize, bon::Builder)]
pub struct RequestConfig {
    n_predict: Option<usize>,
    temperature: Option<f32>,
    top_k: Option<usize>,
    top_p: Option<f32>,
    stop: Option<Vec<String>>,
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

pub type CompletionStream =
    std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<String, LlamaError>> + Send>>;

impl LlamaLink {
    pub fn new(url: &str, request_config: RequestConfig) -> Self {
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

    pub async fn completion(&self, prompt: String) -> Result<String, LlamaError> {
        let mut json = self.request_config.clone();
        json.insert("prompt".to_owned(), Value::String(prompt));
        let json = Value::Object(json);

        let response = self
            .client
            .post(&self.completion_url)
            .json(&json)
            .send()
            .await?;

        if response.status().is_success() {
            let response_body: CompletionResponse = response.json().await?;
            response_body
                .content
                .ok_or_else(|| "No response content".to_string().into())
        } else {
            Err(LlamaError {
                message: format!("HTTP Error: {}", response.status()),
            })
        }
    }

    pub async fn raw_tool_call(&self, prompt: String, schema: Value) -> Result<String, LlamaError> {
        debug_assert!(
            matches!(schema, Value::Object(_)),
            "Schema must be an object"
        );

        let mut json = self.request_config.clone();
        json.insert("prompt".to_owned(), Value::String(prompt));
        json.insert("json_schema".to_owned(), schema);
        let json = Value::Object(json);

        let response = self
            .client
            .post(&self.completion_url)
            .json(&json)
            .send()
            .await?;

        if response.status().is_success() {
            let response_body: CompletionResponse = response.json().await?;
            response_body
                .content
                .ok_or_else(|| "No response content".to_string().into())
        } else {
            Err(LlamaError {
                message: format!("HTTP Error: {}", response.status()),
            })
        }
    }

    pub async fn completion_stream(&self, prompt: String) -> Result<CompletionStream, LlamaError> {
        let mut json = self.request_config.clone();
        json.insert("prompt".to_owned(), Value::String(prompt));
        json.insert("stream".to_owned(), Value::Bool(true));
        let json = Value::Object(json);

        let request = self.client.post(&self.completion_url).json(&json);

        // Why SSE: https://github.com/ggerganov/llama.cpp/blob/89d604f2c87af9db657d8a27a1528bc4b7579c29/examples/server/README.md?plain=1#L450
        let es = EventSource::new(request).map_err(|_| "".to_owned())?;
        // es.set_retry_policy(policy);
        let stream = es
            .map(|event| match event {
                Ok(Event::Open) => {
                    println!("Connection Open!");
                    Some(Ok("".to_owned()))
                }
                Ok(Event::Message(message)) => {
                    let response = serde_json::from_str::<CompletionResponse>(&message.data);
                    match response {
                        Ok(response) => {
                            if response.stop.unwrap_or(false) {
                                return None;
                            }
                            Some(Ok(response.content.unwrap_or_else(|| "".to_owned())))
                        }
                        Err(e) => Some(Err(LlamaError::from(format!("Error in stream: {:?}", e)))),
                    }
                }
                Err(err) => {
                    println!("Error: {}", err);
                    // es.close();
                    None
                    // Err(LlamaError::from(format!("Error in stream: {:?}", err)))
                }
            })
            .take_while(|e| e.is_some())
            .filter_map(|e| e);
        Ok(Box::pin(stream))
    }
}
