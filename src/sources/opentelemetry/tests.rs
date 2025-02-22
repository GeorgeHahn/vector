use crate::{
    config::{SourceConfig, SourceContext},
    event::{into_event_stream, Event, EventStatus, LogEvent, Value},
    opentelemetry::{
        Common::{any_value, AnyValue, KeyValue},
        LogService::{logs_service_client::LogsServiceClient, ExportLogsServiceRequest},
        Logs::{LogRecord, ResourceLogs, ScopeLogs},
        Resource as OtelResource,
    },
    sources::opentelemetry::{GrpcConfig, HttpConfig, OpentelemetryConfig, LOGS},
    test_util::{
        self,
        components::{assert_source_compliance, SOURCE_TAGS},
        next_addr,
    },
    SourceSender,
};
use chrono::{TimeZone, Utc};
use futures::Stream;
use futures_util::StreamExt;
use std::collections::BTreeMap;
use tonic::Request;

#[test]
fn generate_config() {
    crate::test_util::test_generate_config::<OpentelemetryConfig>();
}

#[tokio::test]
async fn receive_grpc_logs() {
    assert_source_compliance(&SOURCE_TAGS, async {
        let grpc_addr = next_addr();
        let http_addr = next_addr();

        let source = OpentelemetryConfig {
            grpc: GrpcConfig {
                address: grpc_addr,
                tls: Default::default(),
            },
            http: HttpConfig {
                address: http_addr,
                tls: Default::default(),
            },
            acknowledgements: Default::default(),
        };
        let (sender, logs_output, _) = new_source(EventStatus::Delivered);
        let server = source
            .build(SourceContext::new_test(sender, None))
            .await
            .unwrap();
        tokio::spawn(server);
        test_util::wait_for_tcp(grpc_addr).await;

        // send request via grpc client
        let mut client = LogsServiceClient::connect(format!("http://{}", grpc_addr))
            .await
            .unwrap();
        let req = Request::new(ExportLogsServiceRequest {
            resource_logs: vec![ResourceLogs {
                resource: Some(OtelResource {
                    attributes: vec![KeyValue {
                        key: "res_key".into(),
                        value: Some(AnyValue {
                            value: Some(any_value::Value::StringValue("res_val".into())),
                        }),
                    }],
                    dropped_attributes_count: 0,
                }),
                scope_logs: vec![ScopeLogs {
                    scope: None,
                    log_records: vec![LogRecord {
                        time_unix_nano: 1,
                        observed_time_unix_nano: 2,
                        severity_number: 9,
                        severity_text: "info".into(),
                        body: Some(AnyValue {
                            value: Some(any_value::Value::StringValue("log body".into())),
                        }),
                        attributes: vec![KeyValue {
                            key: "attr_key".into(),
                            value: Some(AnyValue {
                                value: Some(any_value::Value::StringValue("attr_val".into())),
                            }),
                        }],
                        dropped_attributes_count: 3,
                        flags: 4,
                        // opentelemetry sdk will hex::decode the given trace_id and span_id
                        trace_id: str_into_hex_bytes("4ac52aadf321c2e531db005df08792f5"),
                        span_id: str_into_hex_bytes("0b9e4bda2a55530d"),
                    }],
                    schema_url: "v1".into(),
                }],
                schema_url: "v1".into(),
            }],
        });
        let _ = client.export(req).await;
        let mut output = test_util::collect_ready(logs_output).await;
        // we just send one, so only one output
        assert_eq!(output.len(), 1);
        let actual_event = output.pop().unwrap();
        let expect_vec = vec_into_btmap(vec![
            (
                "attributes",
                Value::Object(vec_into_btmap(vec![("attr_key", "attr_val".into())])),
            ),
            (
                "resources",
                Value::Object(vec_into_btmap(vec![("res_key", "res_val".into())])),
            ),
            ("message", "log body".into()),
            ("trace_id", "4ac52aadf321c2e531db005df08792f5".into()),
            ("span_id", "0b9e4bda2a55530d".into()),
            ("severity_number", 9.into()),
            ("severity_text", "info".into()),
            ("flags", 4.into()),
            ("dropped_attributes_count", 3.into()),
            ("timestamp", Utc.timestamp_nanos(1).into()),
            ("observed_timestamp", Utc.timestamp_nanos(2).into()),
        ]);
        let expect_event = Event::from(LogEvent::from(expect_vec));
        assert_eq!(actual_event, expect_event);
    })
    .await;
}

fn new_source(
    status: EventStatus,
) -> (
    SourceSender,
    impl Stream<Item = Event>,
    impl Stream<Item = Event>,
) {
    let (mut sender, recv) = SourceSender::new_test_finalize(status);
    let logs_output = sender
        .add_outputs(status, LOGS.to_string())
        .flat_map(into_event_stream);
    (sender, logs_output, recv)
}

fn str_into_hex_bytes(s: &str) -> Vec<u8> {
    // unwrap is okay in test
    hex::decode(s).unwrap()
}

fn vec_into_btmap(arr: Vec<(&'static str, Value)>) -> BTreeMap<String, Value> {
    BTreeMap::from_iter(
        arr.into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect::<Vec<(_, _)>>(),
    )
}
