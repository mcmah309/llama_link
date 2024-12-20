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
    let result = link
        .call_function(format_prompt("What do you think about canadians", &toolbox), &toolbox)
        .await;
    match result {
        Ok(Ok(call_result)) => println!("{}", call_result),
        Err(error) => panic!("{}", error),
    }
}

// Out: "Hello Dave, I don't like `Canadians`, because `Canadians are too boring`"
// Out: "Hello Dave, I like `Canadians`, because `I love Canadians for their kindness and respect for other cultures`"
// Out: "Hello Dave, I don't like `Canadians`, because `Canadians are very rude to tourists.`"

fn format_prompt<O, E>(user: &str, toolbox: &ToolBox<O, E>) -> String {
    format!(
        r#"<|begin_of_text|><|start_header_id|>system<|end_header_id|>
You are a helpful AI assistant. Respond to the user in this json function calling format:
    {}<|eot_id|><|start_header_id|>user<|end_header_id|>
    {}<|eot_id|><|start_header_id|>assistant<|end_header_id|>
    "#,
        serde_json::to_string(toolbox.schema()).unwrap(),
        user
    )
}
