use std::borrow::Cow;

use reqwest::StatusCode;
use tracing::Instrument;

use crate::components::name::ComponentName;
use crate::prelude::StringOrEnv;

const APPLICATION_JSON: reqwest::header::HeaderValue =
    reqwest::header::HeaderValue::from_static("application/json");
const USER_AGENT: &str = concat!(env!("CARGO_CRATE_NAME"), " ", env!("CARGO_PKG_VERSION"));

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("api token not provided")]
    ApiTokenNotFound,
    #[error("api token format is invalid")]
    ApiTokenInvalidFormat(#[source] reqwest::header::InvalidHeaderValue),
    #[error("unable to build client")]
    UnableToBuildReqwestClient(#[source] reqwest::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("invalid payload")]
    InvalidPayload,
    #[error("request failed")]
    RequestError(#[source] reqwest::Error),
}

struct DatadogClient {
    inner: reqwest::Client,
    url: Cow<'static, str>,
}

impl DatadogClient {
    async fn send_many(
        &self,
        event_logs: impl Iterator<Item = crate::event::log::EventLog>,
    ) -> Result<(), ExecutionError> {
        tracing::debug!("sending logs to datadog logs");
        let events = event_logs.collect::<Vec<_>>();
        match self
            .inner
            .post(self.url.as_ref())
            .json(&events)
            .send()
            .await
        {
            Ok(res) if res.status() == StatusCode::BAD_REQUEST => {
                Err(ExecutionError::InvalidPayload)
            }
            Ok(res) => {
                tracing::debug!("events sent with status code: {:?}", res.status());
                Ok(())
            }
            Err(err) => Err(ExecutionError::RequestError(err)),
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct Config {
    url: Option<String>,
    api_token: StringOrEnv,
}

impl Config {
    fn api_token_header(&self) -> Result<reqwest::header::HeaderValue, BuildError> {
        let token = self
            .api_token
            .as_string()
            .ok_or(BuildError::ApiTokenNotFound)?;
        reqwest::header::HeaderValue::from_str(token.as_str())
            .map_err(BuildError::ApiTokenInvalidFormat)
    }

    pub fn build(self) -> Result<(Sink, crate::prelude::Sender), BuildError> {
        let (sender, receiver) = crate::prelude::create_channel(1000);

        let mut headers = reqwest::header::HeaderMap::with_capacity(3);
        headers.append(reqwest::header::ACCEPT, APPLICATION_JSON);
        headers.append(reqwest::header::CONTENT_TYPE, APPLICATION_JSON);
        headers.append("DD-API-KEY", self.api_token_header()?);

        let inner = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent(USER_AGENT)
            .build()
            .map_err(BuildError::UnableToBuildReqwestClient)?;

        let client = DatadogClient {
            inner,
            url: self.url.map(Cow::Owned).unwrap_or_else(|| {
                Cow::Borrowed("https://http-intake.logs.datadoghq.com/api/v2/logs")
            }),
        };

        Ok((Sink { client, receiver }, sender))
    }
}

pub struct Sink {
    client: DatadogClient,
    receiver: crate::prelude::Receiver,
}

impl Sink {
    async fn execute(mut self) {
        tracing::info!("starting");
        let mut buffer = Vec::with_capacity(20);
        loop {
            let size = self.receiver.recv_many(&mut buffer, 20).await;
            if size == 0 {
                break;
            }
            tracing::debug!("received {size} events");
            if let Err(error) = self
                .client
                .send_many(buffer.drain(..).filter_map(|item| item.into_event_log()))
                .await
            {
                eprintln!("{error:?}");
            }
        }
        tracing::info!("stopping");
    }

    pub async fn run(self, name: &ComponentName) -> tokio::task::JoinHandle<()> {
        let span = tracing::info_span!(
            "component",
            name = name.as_ref(),
            kind = "sink",
            flavor = "datadog_logs"
        );
        tokio::spawn(async move { self.execute().instrument(span).await })
    }
}
