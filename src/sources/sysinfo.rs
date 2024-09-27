use std::collections::VecDeque;

use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use tracing::Instrument;

use crate::components::collector::Collector;
use crate::components::output::ComponentWithOutputs;
use crate::event::metric::{EventMetric, EventMetricValue};
use crate::event::Event;

const NAMESPACE: &str = "host.system";

#[derive(Debug, thiserror::Error)]
pub struct BuildError;

impl std::fmt::Display for BuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unable to build component")
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct CpuConfig {
    #[serde(default = "crate::helper::default_true")]
    pub usage: bool,
    #[serde(default = "crate::helper::default_true")]
    pub frequency: bool,
}

impl Default for CpuConfig {
    fn default() -> Self {
        Self {
            usage: true,
            frequency: true,
        }
    }
}

impl CpuConfig {
    fn refresh_kind(&self) -> CpuRefreshKind {
        let res = CpuRefreshKind::new();
        let res = match self.usage {
            true => res.with_cpu_usage(),
            false => res.without_cpu_usage(),
        };
        match self.frequency {
            true => res.with_frequency(),
            false => res.without_frequency(),
        }
    }
}

#[derive(Clone, Debug, serde::Deserialize)]
pub struct MemoryConfig {
    #[serde(default = "crate::helper::default_true")]
    pub ram: bool,
    #[serde(default = "crate::helper::default_true")]
    pub swap: bool,
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            ram: true,
            swap: true,
        }
    }
}

impl MemoryConfig {
    fn refresh_kind(&self) -> MemoryRefreshKind {
        let res = MemoryRefreshKind::new();
        let res = match self.ram {
            true => res.with_ram(),
            false => res.without_ram(),
        };
        match self.swap {
            true => res.with_swap(),
            false => res.without_swap(),
        }
    }
}

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct Config {
    /// Interval between emitting events, in ms
    pub interval: Option<u64>,
    #[serde(default)]
    pub cpu: CpuConfig,
    #[serde(default)]
    pub memory: MemoryConfig,
}

impl ComponentWithOutputs for Config {}

impl Config {
    fn refresh_kind(&self) -> RefreshKind {
        RefreshKind::new()
            .with_cpu(self.cpu.refresh_kind())
            .with_memory(self.memory.refresh_kind())
    }

    pub fn build(self) -> Result<Source, BuildError> {
        let specifics = self.refresh_kind();
        Ok(Source {
            state: Stale {
                duration: tokio::time::Duration::from_millis(self.interval.unwrap_or(1000)),
            },
            system: System::new_with_specifics(specifics),
            specifics,
            hostname: System::host_name(),
            config: self,
        })
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum StartingError {}

pub(crate) struct Stale {
    duration: tokio::time::Duration,
}

pub(crate) struct Running {
    timer: tokio::time::Interval,
}

pub struct Source<S = Stale> {
    state: S,
    config: Config,
    system: sysinfo::System,
    specifics: sysinfo::RefreshKind,
    hostname: Option<String>,
}

impl<S> Source<S> {
    pub const fn flavor(&self) -> &'static str {
        "sysinfo"
    }
}

impl Source<Stale> {
    async fn prepare(self) -> Result<Source<Running>, StartingError> {
        Ok(Source {
            state: Running {
                timer: tokio::time::interval(self.state.duration),
            },
            config: self.config,
            system: self.system,
            specifics: self.specifics,
            hostname: self.hostname,
        })
    }

    pub async fn run(
        self,
        span: tracing::Span,
        collector: Collector,
    ) -> Result<tokio::task::JoinHandle<()>, StartingError> {
        let prepared = self.prepare().await?;
        Ok(tokio::spawn(async move {
            prepared.execute(collector).instrument(span).await
        }))
    }
}

impl Source<Running> {
    fn reload(&mut self) {
        tracing::debug!("reloading system");
        self.system.refresh_specifics(self.specifics);
    }

    fn global_cpu_usage(&self, instant: u64, buffer: &mut VecDeque<EventMetric>) {
        let value = self.system.global_cpu_usage();
        let event = EventMetric::new(
            instant,
            NAMESPACE,
            "global-cpu-usage",
            EventMetricValue::Gauge(value as f64),
        );
        buffer.push_back(event);
    }

    fn cpu_usage(&self, instant: u64, buffer: &mut VecDeque<EventMetric>) {
        for cpu in self.system.cpus() {
            let value = cpu.cpu_usage();
            let event = EventMetric::new(
                instant,
                NAMESPACE,
                "cpu-usage",
                EventMetricValue::Gauge(value as f64),
            )
            .with_tag("name", cpu.name().to_owned());
            buffer.push_back(event);
        }
    }

    fn cpu_frequency(&self, instant: u64, buffer: &mut VecDeque<EventMetric>) {
        for cpu in self.system.cpus() {
            let value = cpu.frequency();
            let event = EventMetric::new(
                instant,
                NAMESPACE,
                "cpu-frequency",
                EventMetricValue::Gauge(value as f64),
            )
            .with_tag("name", cpu.name().to_owned());
            buffer.push_back(event);
        }
    }

    fn free_swap(&self, instant: u64, buffer: &mut VecDeque<EventMetric>) {
        let value = self.system.free_swap();
        let event = EventMetric::new(
            instant,
            NAMESPACE,
            "free-swap",
            EventMetricValue::Gauge(value as f64),
        );
        buffer.push_back(event);
    }

    fn used_swap(&self, instant: u64, buffer: &mut VecDeque<EventMetric>) {
        let value = self.system.used_swap();
        let event = EventMetric::new(
            instant,
            NAMESPACE,
            "used-swap",
            EventMetricValue::Gauge(value as f64),
        );
        buffer.push_back(event);
    }

    fn total_swap(&self, instant: u64, buffer: &mut VecDeque<EventMetric>) {
        let value = self.system.total_swap();
        let event = EventMetric::new(
            instant,
            NAMESPACE,
            "total-swap",
            EventMetricValue::Gauge(value as f64),
        );
        buffer.push_back(event);
    }

    fn available_memory(&self, instant: u64, buffer: &mut VecDeque<EventMetric>) {
        let value = self.system.available_memory();
        let event = EventMetric::new(
            instant,
            NAMESPACE,
            "available-memory",
            EventMetricValue::Gauge(value as f64),
        );
        buffer.push_back(event);
    }

    fn free_memory(&self, instant: u64, buffer: &mut VecDeque<EventMetric>) {
        let value = self.system.free_memory();
        let event = EventMetric::new(
            instant,
            NAMESPACE,
            "free-memory",
            EventMetricValue::Gauge(value as f64),
        );
        buffer.push_back(event);
    }

    fn used_memory(&self, instant: u64, buffer: &mut VecDeque<EventMetric>) {
        let value = self.system.used_memory();
        let event = EventMetric::new(
            instant,
            NAMESPACE,
            "used-memory",
            EventMetricValue::Gauge(value as f64),
        );
        buffer.push_back(event);
    }

    fn total_memory(&self, instant: u64, buffer: &mut VecDeque<EventMetric>) {
        let value = self.system.total_memory();
        let event = EventMetric::new(
            instant,
            NAMESPACE,
            "total-memory",
            EventMetricValue::Gauge(value as f64),
        );
        buffer.push_back(event);
    }

    fn iterate(&mut self, buffer: &mut VecDeque<EventMetric>) {
        self.reload();
        let instant = crate::helper::now();
        if self.config.cpu.usage {
            self.global_cpu_usage(instant, buffer);
            self.cpu_usage(instant, buffer);
        }
        if self.config.cpu.frequency {
            self.cpu_frequency(instant, buffer);
        }
        if self.config.memory.swap {
            self.free_swap(instant, buffer);
            self.used_swap(instant, buffer);
            self.total_swap(instant, buffer);
        }
        if self.config.memory.ram {
            self.available_memory(instant, buffer);
            self.free_memory(instant, buffer);
            self.used_memory(instant, buffer);
            self.total_memory(instant, buffer);
        }
    }

    fn augment_metric(&self, mut metric: EventMetric) -> Event {
        if let Some(ref inner) = self.hostname {
            metric.add_tag("hostname", inner.to_owned());
        }
        metric.into()
    }

    async fn execute(mut self, collector: Collector) {
        tracing::info!("starting");
        let mut buffer = VecDeque::new();
        'root: loop {
            let _ = self.state.timer.tick().await;
            self.iterate(&mut buffer);
            while let Some(metric) = buffer.pop_front() {
                let event = self.augment_metric(metric);
                if let Err(error) = collector.send_default(event).await {
                    tracing::error!("unable to send generated log: {error:?}");
                    break 'root;
                }
            }
        }
        tracing::info!("stopping");
    }
}
