pub type Sender = tokio::sync::mpsc::Sender<crate::event::Event>;
pub type Receiver = tokio::sync::mpsc::Receiver<crate::event::Event>;

#[inline]
pub fn create_channel(size: usize) -> (Sender, Receiver) {
    tokio::sync::mpsc::channel(size)
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(untagged)]
pub enum StringOrEnv {
    String(String),
    EnvironmentVariable {
        key: String,
        default_value: Option<String>,
    },
}

impl StringOrEnv {
    pub fn as_string(&self) -> Option<String> {
        match self {
            Self::String(inner) => Some(inner.clone()),
            Self::EnvironmentVariable { key, default_value } => std::env::var(key)
                .ok()
                .or(default_value.as_ref().map(String::from)),
        }
    }
}
