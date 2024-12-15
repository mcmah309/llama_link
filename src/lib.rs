mod errors;

use core::panic;
use errors::LlamaError;
use reqwest::{Client, Error};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::io::{self, Write};

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
}

// async fn make_stream_request(client: &Client, url: &str, prompt: &str) -> Result<(), Error> {
//     let request_body = CompletionRequest {
//         prompt: prompt.to_string(),
//         n_predict: 128,
//         stream: true,
//         temperature: Some(0.8),
//         top_k: Some(40),
//         top_p: Some(0.9),
//         stop: Some(vec!["\n".to_string()]),
//     };

//     let response = client.post(url).json(&request_body).send().await?;

//     if response.status().is_success() {
//         let mut stream = response.bytes_stream();
//         while let Some(chunk) = stream.next().await {
//             match chunk {
//                 Ok(bytes) => {
//                     if let Ok(text) = String::from_utf8(bytes.to_vec()) {
//                         print!("{}", text);
//                         io::stdout().flush().unwrap();
//                     }
//                 }
//                 Err(e) => {
//                     eprintln!("Error in stream: {:?}", e);
//                     break;
//                 }
//             }
//         }
//     } else {
//         eprintln!("HTTP Error: {}", response.status());
//     }

//     Ok(())
// }
