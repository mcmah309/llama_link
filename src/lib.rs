mod errors;

use errors::CompletionStreamError;
pub use errors::{CompletionError, FunctionCallError};

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

pub enum Message {
    User(String),
    Assistant(String),
}

/// The result from calling the function and the raw input used for the function call.
pub struct FunctionCallContext<O, E> {
    pub output_result: Result<O, E>,
    pub raw_input: String,
}

pub type CompletionStream = std::pin::Pin<
    Box<dyn tokio_stream::Stream<Item = Result<String, CompletionStreamError>> + Send>,
>;

impl LlamaLink {
    pub fn new(url: &str, request_config: Config) -> Self {
        let request_config = match serde_json::to_value(request_config).unwrap() {
            Value::Object(map) => map,
            _ => unreachable!("RequestConfig should always be created as an object"),
        };
        Self {
            client: Client::new(),
            completion_url: format!("{url}/completion"),
            request_config,
        }
    }

    pub async fn create_completion_with_format(
        &self,
        system: &str,
        messages: &[Message],
        formatter: &PromptFormatter,
    ) -> Result<String, CompletionError> {
        let prompt = (formatter.0)(system, messages);
        self.create_completion(prompt).await
    }

    pub async fn create_completion(&self, prompt: String) -> Result<String, CompletionError> {
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
            issue: "No `content` field in response body".to_owned(),
        })
    }

    pub async fn call_function<O, E>(
        &self,
        prompt: String,
        toolbox: &ToolBox<O, E>,
    ) -> Result<Result<O, E>, FunctionCallError> {
        self.call_function_full(prompt, toolbox)
            .await
            .map(|e| e.output_result)
    }

    pub async fn call_function_with_format<O, E>(
        &self,
        system: &str,
        messages: &[Message],
        formatter: &PromptFormatter,
        toolbox: &ToolBox<O, E>,
    ) -> Result<Result<O, E>, FunctionCallError> {
        let prompt = (formatter.0)(system, messages);
        self.call_function(prompt, toolbox).await
    }

    pub async fn call_function_with_format_full<O, E>(
        &self,
        system: &str,
        messages: &[Message],
        formatter: &PromptFormatter,
        toolbox: &ToolBox<O, E>,
    ) -> Result<FunctionCallContext<O, E>, FunctionCallError> {
        let prompt = (formatter.0)(system, messages);
        self.call_function_full(prompt, toolbox).await
    }

    pub async fn call_function_full<O, E>(
        &self,
        prompt: String,
        toolbox: &ToolBox<O, E>,
    ) -> Result<FunctionCallContext<O, E>, FunctionCallError> {
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
            return Err(FunctionCallError::Api {
                issue: format!("HTTP Error Calling: {}", response.status()),
            });
        }

        let response_body: CompletionResponse = response.json().await?;
        let content = response_body.content.ok_or_else(|| FunctionCallError::Api {
            issue: "No `content` field in response body".to_owned(),
        })?;
        #[cfg(feature = "tracing")]
        tracing::debug!("Raw tool_call response:\n`{}`", &content);
        let tool_call = serde_json::from_str(&content).map_err(|_| FunctionCallError::Parsing {
            issue: "Could not parse tool call response into valid json".to_owned(),
        })?;
        let tool_call_result: Result<Result<O, E>, FunctionCallError> = 
            toolbox.call_from_value(tool_call).await.map_err(|error| error.into());
        tool_call_result.map(|e| FunctionCallContext {
            output_result: e,
            raw_input: content,
        })
    }

    pub fn create_formatted_completion_stream(
        &self,
        system: &str,
        messages: &[Message],
        formatter: &PromptFormatter,
    ) -> CompletionStream {
        let prompt = (formatter.0)(system, messages);
        self.create_completion_stream(prompt)
    }

    pub fn create_completion_stream(&self, prompt: String) -> CompletionStream {
        let mut json = self.request_config.clone();
        json.insert("prompt".to_owned(), Value::String(prompt));
        json.insert("stream".to_owned(), Value::Bool(true));
        let json = Value::Object(json);

        let request = self.client.post(&self.completion_url).json(&json);

        // Why SSE: https://github.com/ggerganov/llama.cpp/blob/89d604f2c87af9db657d8a27a1528bc4b7579c29/examples/server/README.md?plain=1#L450
        let es: Result<EventSource, reqwest_eventsource::CannotCloneRequestError> =
            EventSource::new(request);
        let es = match es {
            Ok(value) => value,
            Err(_) => {
                // Dev Note: We do not expose this since this should never be possible, but would rather log and be safe, then unwrap and panic, or make the caller handle
                #[cfg(feature = "tracing")]
                tracing::error!("Could not create event source for SSE in completion stream");
                return Box::pin(tokio_stream::empty());
            }
        };
        let stream = es
            .map(|event| match event {
                Ok(Event::Open) => {
                    #[cfg(feature = "tracing")]
                    tracing::trace!("Completion stream SSE connection open.");
                    Some(Ok(String::new()))
                }
                Ok(Event::Message(message)) => {
                    let response = serde_json::from_str::<CompletionResponse>(&message.data);
                    match response {
                        Ok(response) => {
                            if response.stop.unwrap_or(false) {
                                #[cfg(feature = "tracing")]
                                tracing::trace!("Completion stream received stop");
                                return None;
                            }
                            Some(Ok(response.content.unwrap_or_else(|| String::new())))
                        }
                        #[allow(unused_variables)]
                        Err(err) => {
                            #[cfg(feature = "tracing")]
                            tracing::error!("Error in completion stream: {:?}", e);
                            Some(Err(CompletionStreamError::from(err)))
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
                    Some(Err(CompletionStreamError::from(err)))
                }
            })
            .take_while(|e: &Option<Result<String, CompletionStreamError>>| e.is_some())
            .filter_map(|e| e);
        Box::pin(stream)
    }
}

/// The formatter used to create the prompt for the llm
pub struct PromptFormatter(fn(&str, &[Message]) -> String);

impl PromptFormatter {
    pub fn new(formatter: fn(&str, &[Message]) -> String) -> Self {
        Self(formatter)
    }

    // https://www.llama.com/docs/model-cards-and-prompt-formats/meta-llama-3/
    pub const fn default_const() -> Self {
        Self(|system, messages| {
            debug_assert!(messages.len() > 0, "Messages must not be empty");
            debug_assert!(
                matches!(messages.first().unwrap(), Message::User(_)),
                "First message must be a user message"
            );
            debug_assert!(
                matches!(messages.last().unwrap(), Message::User(_)),
                "Last message must be a user message"
            );
            let mut formatted = String::new();
            formatted.push_str(&format!(
                "<|begin_of_text|><|start_header_id|>system<|end_header_id|>\n\n{}<|eot_id|>",
                system
            ));
            for message in messages {
                match message {
                    Message::User(text) => {
                        formatted.push_str(&format!(
                            "<|start_header_id|>user<|end_header_id|>\n\n{}<|eot_id|>",
                            text
                        ));
                    }
                    Message::Assistant(text) => {
                        formatted.push_str(&format!(
                            "<|start_header_id|>assistant<|end_header_id|>\n\n{}<|eot_id|>",
                            text
                        ));
                    }
                }
            }
            formatted.push_str("<|start_header_id|>assistant<|end_header_id|>\n\n");
            formatted
        })
    }
}

impl Default for PromptFormatter {
    fn default() -> Self {
        Self::default_const()
    }
}
