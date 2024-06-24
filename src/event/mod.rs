#[derive(Debug, Clone)]
pub enum Event {
    Log(EventLog),
}

impl From<EventLog> for Event {
    fn from(value: EventLog) -> Self {
        Self::Log(value)
    }
}

#[derive(Debug, Clone)]
pub struct EventLog {
    message: String,
}

impl EventLog {
    pub fn new<M: Into<String>>(message: M) -> Self {
        Self {
            message: message.into(),
        }
    }
}
