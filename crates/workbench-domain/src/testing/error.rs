use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum TestingError {
    #[error("invalid action manifest `{manifest_path}`: {detail}")]
    InvalidAction {
        manifest_path: String,
        detail: String,
    },

    #[error("invalid test case: {detail}")]
    InvalidTestCase { detail: String },

    #[error("could not generate remote-test workflow: {detail}")]
    WorkflowGeneration { detail: String },
}
