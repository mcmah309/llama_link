#[cfg(test)]
mod normal {
    use llama_link::*;
    use tokio_stream::StreamExt;

    #[tokio::test]
    async fn completion() {
        let link = LlamaLink::new("http://127.0.0.1:3756", Config::builder().build());

        let response = link
            .create_completion("In one sentence, tell me a joke.".to_owned())
            .await
            .unwrap();

        assert!(!response.is_empty())
    }

    #[tokio::test]
    async fn completion_stream() {
        let link = LlamaLink::new("http://127.0.0.1:3756", Config::builder().build());
        let mut response_stream = link.create_formatted_completion_stream(
            "",
            &vec![Message::User("In one sentence, tell me a joke.".to_owned())],
            &PromptFormatter::default(),
        );

        let mut count = 0;
        #[allow(unused_variables)]
        while let Some(content) = response_stream.next().await {
            match content {
                Ok(content) => {
                    // print!("{}", content)
                }
                Err(error) => panic!("{}", error),
            }
            count += 1;
        }
        assert!(count > 0);
    }
}

#[cfg(test)]
mod toolbox {
    use std::{any::Any, convert::Infallible, ops::Deref};

    use llama_link::*;
    use llmtoolbox::{tool, ToolBox};

    #[derive(Debug)]
    struct MyTool;

    #[tool]
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
    async fn function_call() {
        let tool = MyTool::new();
        let mut toolbox: ToolBox<Box<dyn Any>, Infallible> = ToolBox::new();
        toolbox.add_tool(tool).unwrap();
        println!(
            "Schema: {}",
            serde_json::to_string_pretty(&toolbox.schema()).unwrap()
        );

        let link = LlamaLink::new("http://127.0.0.1:3756", Config::builder().build());
        let result = link.call_function("call greet".to_owned(), &toolbox).await;
        match result {
            Ok(Ok(call_result)) => match call_result.downcast::<String>() {
                Ok(message) => assert!(message.deref().starts_with("This is the greeting")),
                Err(_) => panic!("Not the corect type"),
            },
            Err(error) => panic!("{}", error),
        }
    }
}
