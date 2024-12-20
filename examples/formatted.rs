use std::convert::Infallible;

use llama_link::*;
use llmtoolbox::{tool, ToolBox};

#[derive(Debug)]
struct ConversationTool {
    user_name: String,
}

#[tool]
impl ConversationTool {
    fn new(user_name: String) -> Self {
        Self { user_name }
    }

    /// Give a negative opinion about the topic
    /// `topic` - What the opinion is about
    #[tool_part]
    fn give_negative_opinion(&self, topic: ConverstationTopic) -> String {
        format!(
            "Hello {}, I don't like `{}`, because `{}`",
            self.user_name, topic.topic, topic.opinion
        )
    }

    /// Give a positive opinion about the topic
    /// `topic` - What the opinion is about
    #[tool_part]
    async fn give_positive_opinion(&self, topic: ConverstationTopic) -> String {
        format!(
            "Hello {}, I like `{}`, because `{}`",
            self.user_name, topic.topic, topic.opinion
        )
    }
}

#[derive(serde::Deserialize, schemars::JsonSchema)]
pub struct ConverstationTopic {
    /// The topic being discussed
    pub topic: String,
    /// The opinion about the topic
    pub opinion: String,
}

#[tokio::main]
async fn main() {
    let mut toolbox: ToolBox<String, Infallible> = ToolBox::new();
    let tool = ConversationTool::new("Dave".to_owned());
    toolbox.add_tool(tool).unwrap();

    let link = LlamaLink::new("http://127.0.0.1:3756", Config::builder().build());
    let system = format!("You are a helpful AI assistant. Respond to the user in this json function calling format: {}",serde_json::to_string(toolbox.schema()).unwrap());
    let messages = vec![Message::User("What do you think about the rust programming language".to_owned())];
    let result = link
        .formatted_tool_call(
            &system, &messages, &PromptFormatter::default(), &toolbox)
        .await;
    match result {
        Ok(Ok(call_result)) => println!("{}", call_result),
        Err(error) => panic!("{}", error),
    }
}

// Out: Hello Dave, I like `Rust programming language`, because `Rust is a great programming language that offers a unique combination of safety, performance, and concurrency features, making it an excellent choice for systems programming.`
// Out: Hello Dave, I like `Rust programming language`, because `I think Rust is a fantastic programming language that offers a unique combination of performance, safety, and concurrency features. Its ownership model and borrow checker help prevent common programming errors like null pointer dereferences and data races, making it a great choice for systems programming.`
// Out: Hello Dave, I like `Rust programming language`, because `I think Rust is a great programming language due to its strong focus on safety and performance. Its ownership system and borrow checker help prevent common programming errors like null pointer dereferences and data races, making it a reliable choice for systems programming.`