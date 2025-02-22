use futures::{future, FutureExt};
use vector_config::configurable_component;

use crate::{
    config::{AcknowledgementsConfig, GenerateConfig, Input, SinkConfig, SinkContext},
    sinks::{blackhole::sink::BlackholeSink, Healthcheck, VectorSink},
};

const fn default_print_interval_secs() -> u64 {
    1
}

/// Configuration for the `blackhole` sink.
#[configurable_component(sink)]
#[derive(Clone, Debug, Derivative)]
#[serde(deny_unknown_fields, default)]
#[derivative(Default)]
pub struct BlackholeConfig {
    /// The number of seconds between reporting a summary of activity.
    ///
    /// Set to `0` to disable reporting.
    #[derivative(Default(value = "1"))]
    #[serde(default = "default_print_interval_secs")]
    pub print_interval_secs: u64,

    /// The number of events, per second, that the sink is allowed to consume.
    ///
    /// By default, there is no limit.
    pub rate: Option<usize>,

    #[configurable(derived)]
    #[serde(
        default,
        deserialize_with = "crate::serde::bool_or_struct",
        skip_serializing_if = "crate::serde::skip_serializing_if_default"
    )]
    pub acknowledgements: AcknowledgementsConfig,
}

#[async_trait::async_trait]
#[typetag::serde(name = "blackhole")]
impl SinkConfig for BlackholeConfig {
    async fn build(&self, _cx: SinkContext) -> crate::Result<(VectorSink, Healthcheck)> {
        let sink = BlackholeSink::new(self.clone());
        let healthcheck = future::ok(()).boxed();

        Ok((VectorSink::Stream(Box::new(sink)), healthcheck))
    }

    fn input(&self) -> Input {
        Input::all()
    }

    fn sink_type(&self) -> &'static str {
        "blackhole"
    }

    fn acknowledgements(&self) -> &AcknowledgementsConfig {
        &self.acknowledgements
    }
}

impl GenerateConfig for BlackholeConfig {
    fn generate_config() -> toml::Value {
        toml::Value::try_from(&Self::default()).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use crate::sinks::blackhole::config::BlackholeConfig;

    #[test]
    fn generate_config() {
        crate::test_util::test_generate_config::<BlackholeConfig>();
    }
}
