mod errors;

use core::panic;
use errors::LlamaError;
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

pub type CompletionStream =
    std::pin::Pin<Box<dyn tokio_stream::Stream<Item = Result<String, LlamaError>> + Send>>;

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

        if !response.status().is_success() {
            return Err(LlamaError {
                message: format!("HTTP Error: {}", response.status()),
            });
        }

        let response_body: CompletionResponse = response.json().await?;
        response_body
            .content
            .ok_or_else(|| "No response content".to_string().into())
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

        if !response.status().is_success() {
            return Err(LlamaError {
                message: format!("HTTP Error: {}", response.status()),
            });
        }

        let response_body: CompletionResponse = response.json().await?;
        response_body
            .content
            .ok_or_else(|| "No response content".to_string().into())
    }

    pub async fn tool_call<O, E>(
        &self,
        prompt: String,
        toolbox: &ToolBox<O, E>,
    ) -> Result<Result<Result<O, E>, llmtoolbox::CallError>, LlamaError> {
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
            return Err(LlamaError {
                message: format!("HTTP Error: {}", response.status()),
            });
        }

        let response_body: CompletionResponse = response.json().await?;
        let content = response_body
            .content
            .ok_or_else(|| LlamaError::from("No response content".to_string()))?;
        println!("Content: {}", &content);
        let tool_call = serde_json::from_str(&content).unwrap(); //todo
        let tool_call_result: Result<Result<O, E>, llmtoolbox::CallError> =
            toolbox.call(tool_call).await;
        Ok(tool_call_result)
    }

    pub async fn completion_stream(&self, prompt: String) -> Result<CompletionStream, LlamaError> {
        let mut json = self.request_config.clone();
        json.insert("prompt".to_owned(), Value::String(prompt));
        json.insert("stream".to_owned(), Value::Bool(true));
        let json = Value::Object(json);

        let request = self.client.post(&self.completion_url).json(&json);

        // Why SSE: https://github.com/ggerganov/llama.cpp/blob/89d604f2c87af9db657d8a27a1528bc4b7579c29/examples/server/README.md?plain=1#L450
        let es = EventSource::new(request)
            .map_err(|_| "Could not create event source for SSE".to_owned())?;
        let stream = es
            .map(|event| match event {
                Ok(Event::Open) => {
                    #[cfg(feature = "tracing")]
                    tracing::trace!("SSE connection open.");
                    Some(Ok(String::new()))
                }
                Ok(Event::Message(message)) => {
                    let response = serde_json::from_str::<CompletionResponse>(&message.data);
                    match response {
                        Ok(response) => {
                            if response.stop.unwrap_or(false) {
                                #[cfg(feature = "tracing")]
                                tracing::trace!("Stop recieved in response");
                                return None;
                            }
                            Some(Ok(response.content.unwrap_or_else(|| String::new())))
                        }
                        Err(e) => Some(Err(LlamaError::from(format!("Error in stream: {:?}", e)))),
                    }
                }
                Err(err) => {
                    if matches!(err, reqwest_eventsource::Error::StreamEnded) {
                        #[cfg(feature = "tracing")]
                        tracing::trace!("Stream ended.");
                        return None;
                    }
                    #[cfg(feature = "tracing")]
                    tracing::error!("Error in stream: {}", err);
                    None
                }
            })
            .take_while(|e| e.is_some())
            .filter_map(|e| e);
        Ok(Box::pin(stream))
    }
}
