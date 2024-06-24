use reqwest::StatusCode;

struct DatadogClient {
    inner: reqwest::Client,
    url: String,
}

impl DatadogClient {
    async fn send_many(&self, event_logs: impl Iterator<Item = crate::event::EventLog>) {
        let events = event_logs.collect::<Vec<_>>();
        match self.inner.post(&self.url).json(&events).send().await {
            Ok(res) if res.status() == StatusCode::BAD_REQUEST => {
                eprintln!("response: {:?}", res.text().await);
            }
            Ok(res) => println!("events sent with status code: {:?}", res.status()),
            Err(err) => eprintln!("something went wrong while sending to datadog: {err:?}"),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Config {
    base_url: Option<String>,
    api_token: Option<String>,
}

impl Config {
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    pub fn with_api_token(mut self, api_token: impl Into<String>) -> Self {
        self.api_token = Some(api_token.into());
        self
    }

    fn api_token_header(&self) -> reqwest::header::HeaderValue {
        match self.api_token {
            Some(ref value) => reqwest::header::HeaderValue::from_str(value)
                .expect("unable to turn api token into header"),
            _ => panic!("api token not found"),
        }
    }

    pub fn build(self) -> (Sink, crate::prelude::Sender) {
        let (sender, receiver) = crate::prelude::create_channel(1000);

        let mut headers = reqwest::header::HeaderMap::with_capacity(3);
        headers.append(
            reqwest::header::ACCEPT,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        headers.append(
            reqwest::header::CONTENT_TYPE,
            reqwest::header::HeaderValue::from_static("application/json"),
        );
        headers.append("DD-API-KEY", self.api_token_header());

        let inner = reqwest::Client::builder()
            .default_headers(headers)
            .user_agent("tiny-vector")
            .build()
            .expect("unable to create datadog client");

        let client = DatadogClient {
            inner,
            url: self.base_url.unwrap_or_else(|| {
                String::from("https://http-intake.logs.datadoghq.com/api/v2/logs")
            }),
        };

        (Sink { client, receiver }, sender)
    }
}

pub struct Sink {
    client: DatadogClient,
    receiver: crate::prelude::Receiver,
}

impl Sink {
    async fn execute(mut self) {
        let mut buffer = Vec::with_capacity(20);
        loop {
            let size = self.receiver.recv_many(&mut buffer, 20).await;
            if size == 0 {
                break;
            }
            println!("received {size} events");
            self.client
                .send_many(buffer.drain(..).filter_map(|item| item.into_event_log()))
                .await;
        }
    }

    pub fn run(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move { self.execute().await })
    }
}
