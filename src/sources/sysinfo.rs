use sysinfo::{CpuRefreshKind, MemoryRefreshKind, RefreshKind, System};
use tokio::sync::mpsc::error::TrySendError;
use tracing::Instrument;

use crate::components::name::ComponentName;
use crate::event::metric::EventMetric;
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
        let res = match self.frequency {
            true => res.with_frequency(),
            false => res.without_frequency(),
        };
        res
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
        let res = match self.swap {
            true => res.with_swap(),
            false => res.without_swap(),
        };
        res
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

impl Config {
    fn refresh_kind(&self) -> RefreshKind {
        RefreshKind::new()
            .with_cpu(self.cpu.refresh_kind())
            .with_memory(self.memory.refresh_kind())
    }

    pub fn build(self, sender: crate::prelude::Sender) -> Result<Source, BuildError> {
        Ok(Source {
            duration: tokio::time::Duration::from_millis(self.interval.unwrap_or(1000)),
            sender,
            system: System::new_with_specifics(self.refresh_kind()),
            hostname: System::host_name(),
            config: self,
        })
    }
}

pub struct Source {
    config: Config,
    duration: tokio::time::Duration,
    sender: crate::prelude::Sender,
    system: sysinfo::System,
    hostname: Option<String>,
}

impl Source {
    fn reload(&mut self) {
        tracing::debug!("reloading system");
        self.system.refresh_memory();
    }

    fn global_cpu_usage(&self) -> Result<(), TrySendError<Event>> {
        let value = self.system.global_cpu_usage();
        let event = EventMetric::new(NAMESPACE, "global-cpu-usage", value as f64);
        self.send_metric(event)
    }

    fn cpu_usage(&self) -> Result<(), TrySendError<Event>> {
        for cpu in self.system.cpus() {
            let value = cpu.cpu_usage();
            let event = EventMetric::new(NAMESPACE, "cpu-usage", value as f64)
                .with_tag("name", cpu.name().to_owned());
            self.send_metric(event)?;
        }
        Ok(())
    }

    fn cpu_frequency(&self) -> Result<(), TrySendError<Event>> {
        for cpu in self.system.cpus() {
            let value = cpu.frequency();
            let event = EventMetric::new(NAMESPACE, "cpu-frequency", value as f64)
                .with_tag("name", cpu.name().to_owned());
            self.send_metric(event)?;
        }
        Ok(())
    }

    fn free_swap(&self) -> Result<(), TrySendError<Event>> {
        let value = self.system.free_swap();
        let event = EventMetric::new(NAMESPACE, "free-swap", value as f64);
        self.send_metric(event)
    }

    fn used_swap(&self) -> Result<(), TrySendError<Event>> {
        let value = self.system.used_swap();
        let event = EventMetric::new(NAMESPACE, "used-swap", value as f64);
        self.send_metric(event)
    }

    fn total_swap(&self) -> Result<(), TrySendError<Event>> {
        let value = self.system.total_swap();
        let event = EventMetric::new(NAMESPACE, "total-swap", value as f64);
        self.send_metric(event)
    }

    fn available_memory(&self) -> Result<(), TrySendError<Event>> {
        let value = self.system.available_memory();
        let event = EventMetric::new(NAMESPACE, "available-memory", value as f64);
        self.send_metric(event)
    }

    fn free_memory(&self) -> Result<(), TrySendError<Event>> {
        let value = self.system.free_memory();
        let event = EventMetric::new(NAMESPACE, "free-memory", value as f64);
        self.send_metric(event)
    }

    fn used_memory(&self) -> Result<(), TrySendError<Event>> {
        let value = self.system.used_memory();
        let metric = EventMetric::new(NAMESPACE, "used-memory", value as f64);
        self.send_metric(metric)
    }

    fn total_memory(&self) -> Result<(), TrySendError<Event>> {
        let value = self.system.total_memory();
        let metric = EventMetric::new(NAMESPACE, "total-memory", value as f64);
        self.send_metric(metric)
    }

    fn send_metric(&self, mut metric: EventMetric) -> Result<(), TrySendError<Event>> {
        if let Some(ref inner) = self.hostname {
            metric.add_tag("hostname", inner.to_owned());
        }
        self.sender.try_send(metric.into())
    }

    fn iterate(&mut self) -> Result<(), TrySendError<Event>> {
        self.reload();
        if self.config.cpu.usage {
            self.global_cpu_usage()?;
            self.cpu_usage()?;
        }
        if self.config.cpu.frequency {
            self.cpu_frequency()?;
        }
        if self.config.memory.swap {
            self.free_swap()?;
            self.used_swap()?;
            self.total_swap()?;
        }
        if self.config.memory.ram {
            self.available_memory()?;
            self.free_memory()?;
            self.used_memory()?;
            self.total_memory()?;
        }
        Ok(())
    }

    async fn execute(mut self) {
        tracing::info!("starting");
        let mut timer = tokio::time::interval(self.duration);
        loop {
            let _ = timer.tick().await;
            if let Err(err) = self.iterate() {
                tracing::error!("unable to send generated event: {err:?}");
                break;
            }
        }
        tracing::info!("stopping");
    }

    pub async fn run(self, name: &ComponentName) -> tokio::task::JoinHandle<()> {
        let span = tracing::info_span!(
            "component",
            name = name.as_ref(),
            kind = "source",
            flavor = "sysinfo"
        );
        tokio::spawn(async move { self.execute().instrument(span).await })
    }
}
