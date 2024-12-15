#[cfg(test)]
mod normal {
    use llama_link::*;
    use serde_json::Value;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn completion() {
        let link = LlamaLink::new("http://127.0.0.1:3756", Config::builder().build());

        let response = link.completion("In one sentence, tell me a joke.".to_owned()).await.unwrap();

        assert!(!response.is_empty())
    }

    #[tokio::test]
    async fn raw_tool_call() {
        let link = LlamaLink::new("http://127.0.0.1:3756", Config::builder().build());

        let schema = serde_json::json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "array",
            "minItems": 1,
            "maxItems": 1,
            "uniqueItems": true,
            "items": {
                "oneOf": [
                    {
                        "type": "object",
                        "properties": {
                            "function_name": {
                                "const": "calculate_total"
                            },
                            "items": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "name": {
                                            "type": "string"
                                        },
                                        "price": {
                                            "type": "number",
                                            "minimum": 0
                                        }
                                    },
                                    "required": [
                                        "name",
                                        "price"
                                    ]
                                }
                            },
                            "tax_rate": {
                                "type": "number",
                                "minimum": 0,
                                "maximum": 1
                            }
                        },
                        "required": [
                            "function_name",
                            "items"
                        ]
                    },
                    {
                        "type": "object",
                        "properties": {
                            "function_name": {
                                "const": "send_email"
                            },
                            "to": {
                                "type": "string",
                                "format": "email"
                            },
                            "subject": {
                                "type": "string"
                            },
                            "body": {
                                "type": "string"
                            },
                            "attachments": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "filename": {
                                            "type": "string"
                                        },
                                        "content": {
                                            "type": "string"
                                        }
                                    },
                                    "required": [
                                        "filename",
                                        "content"
                                    ]
                                }
                            }
                        },
                        "required": [
                            "function_name",
                            "to",
                            "subject",
                            "body"
                        ]
                    },
                    {
                        "type": "object",
                        "properties": {
                            "function_name": {
                                "const": "create_user"
                            },
                            "username": {
                                "type": "string"
                            },
                            "email": {
                                "type": "string",
                                "format": "email"
                            },
                            "password": {
                                "type": "string",
                                "minLength": 8
                            },
                            "role": {
                                "type": "string",
                                "enum": [
                                    "admin",
                                    "user",
                                    "editor"
                                ]
                            }
                        },
                        "required": [
                            "function_name",
                            "username",
                            "email",
                            "password"
                        ]
                    }
                ]
            }
        });

        let response = link
            .raw_tool_call("create a new user".to_owned(), schema)
            .await
            .unwrap();

        let tool_call: Value = serde_json::from_str(&response).unwrap();
        let tool_name = (|| {
            let tool_call = tool_call.as_array()?;
            let tool_call = tool_call.get(0)?;
            let tool_call = tool_call.as_object()?;
            let function_name = tool_call.get("function_name")?;
            function_name.as_str()
        })();
        assert_eq!(tool_name, Some("create_user"));
    }

    #[tokio::test]
    async fn completion_stream() {
        let link = LlamaLink::new("http://127.0.0.1:3756", Config::builder().build());
        let mut response_stream = link.completion_stream("In one sentence, tell me a joke.".to_owned()).await.unwrap();

        let mut count = 0;
        while let Some(response) = response_stream.next().await {
            match response {
                Ok(_content) => {
                    count += 1;
                    // print!("{}", content);
                }
                Err(err) => {
                    panic!("{}", err);
                }
            }
        }
        assert!(count > 0);
    }
}

#[cfg(test)]
mod toolbox {
    use std::{any::Any, convert::Infallible, ops::Deref};

    use llama_link::*;
    use llmtoolbox::{llmtool, ToolBox};

    #[derive(Debug)]
    struct MyTool;

    #[llmtool]
    impl MyTool {
        fn new() -> Self {
            Self
        }

        /// This
        /// `greeting` - descr
        #[tool_part]
        fn greet(&self, greeting: &str) -> String {
            println!("Greetings!");
            format!("This is the greeting `{greeting}`")
        }

        #[allow(dead_code)]
        fn goodbye(&self) -> u32 {
            println!("Goodbye!");
            1
        }

        /// func descrip
        /// `topic` - field description
        #[tool_part]
        async fn talk(&self, topic: ConverstationTopic) -> u32 {
            let ConverstationTopic { topic, opinion } = topic;
            println!("For {topic} it is {opinion}");
            0
        }
    }

    /// Description
    #[derive(serde::Deserialize, schemars::JsonSchema)]
    pub struct ConverstationTopic {
        pub topic: String,
        pub opinion: String,
    }

    #[tokio::test]
    async fn tool_call() {
        let tool = MyTool::new();
        let mut toolbox: ToolBox<Box<dyn Any>, Infallible> = ToolBox::new();
        toolbox.add_tool(tool).unwrap();
        // println!("Schema: {}", serde_json::to_string_pretty(&toolbox.schema()).unwrap());

        let link = LlamaLink::new("http://127.0.0.1:3756", Config::builder().build());
        let response = link
            .tool_call("call greet".to_owned(), &toolbox)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        match response.downcast::<String>() {
            Ok(message) => assert!(
                message.deref().starts_with("This is the greeting")
            ),
            Err(_) => panic!("Not the corect type"),
        }
    }
}
