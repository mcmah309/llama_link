error_set::error_set! {
    CompletionError = {
        #[display("ApiError: An error occurred related to calling llama.cpp: {issue}")]
        Api {
            issue: String
        },
        #[display("ParsingError: {issue}")]
        Parsing {
            issue: String,
        }
    };
    ToolCallError = {
        #[display("The function with name `{function_name}` was not found in the toolbox")]
        FunctionNotFound {
            function_name: String,
        },
    } || CompletionError;
}

impl From<serde_json::Error> for CompletionError {
    fn from(error: serde_json::Error) -> Self {
        Self::Parsing {
            issue: error.to_string(),
        }
    }
}

impl From<reqwest::Error> for CompletionError {
    fn from(error: reqwest::Error) -> Self {
        Self::Api {
            issue: error.to_string(),
        }
    }
}

//************************************************************************//

impl From<serde_json::Error> for ToolCallError {
    fn from(error: serde_json::Error) -> Self {
        Self::Parsing {
            issue: error.to_string(),
        }
    }
}

impl From<reqwest::Error> for ToolCallError {
    fn from(error: reqwest::Error) -> Self {
        Self::Api {
            issue: error.to_string(),
        }
    }
}

impl From<llmtoolbox::CallError> for ToolCallError {
    fn from(error: llmtoolbox::CallError) -> Self {
        match error {
            llmtoolbox::CallError::FunctionNotFound { function_name } => {
                Self::FunctionNotFound { function_name }
            }
            llmtoolbox::CallError::Parsing { issue } => Self::Parsing { issue },
        }
    }
}
