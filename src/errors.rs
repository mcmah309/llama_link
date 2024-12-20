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
    FunctionCallError = {
        #[display("The function with name `{function_name}` was not found in the toolbox")]
        FunctionNotFound {
            function_name: String,
        },
    } || CompletionError;

    CompletionStreamError = {
        Deserialization(serde_json::Error),
        SSE(reqwest_eventsource::Error)
    };
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

impl From<serde_json::Error> for FunctionCallError {
    fn from(error: serde_json::Error) -> Self {
        Self::Parsing {
            issue: error.to_string(),
        }
    }
}

impl From<reqwest::Error> for FunctionCallError {
    fn from(error: reqwest::Error) -> Self {
        Self::Api {
            issue: error.to_string(),
        }
    }
}

impl From<llmtoolbox::FunctionCallError> for FunctionCallError {
    fn from(error: llmtoolbox::FunctionCallError) -> Self {
        match error {
            llmtoolbox::FunctionCallError::FunctionNotFound { function_name } => {
                Self::FunctionNotFound { function_name }
            }
            llmtoolbox::FunctionCallError::Parsing { issue } => Self::Parsing { issue },
        }
    }
}
