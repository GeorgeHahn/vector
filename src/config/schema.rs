use serde::{Deserialize, Serialize};
use vector_core::config::LogNamespace;

pub(crate) use crate::schema::Definition;

#[derive(Debug, Deserialize, Serialize, PartialEq, Copy, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct Options {
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    #[serde(default = "default_validation")]
    pub validation: bool,

    pub log_namespace: Option<bool>,
}

impl Options {
    /// Gets the value of the globally configured log namespace, or the default if it wasn't set.
    pub fn log_namespace(self) -> LogNamespace {
        self.log_namespace
            .map_or(LogNamespace::Legacy, |use_vector_namespace| {
                use_vector_namespace.into()
            })
    }

    /// Merges two schema options together.
    pub fn append(&mut self, with: Self, errors: &mut Vec<String>) {
        if self.log_namespace.is_some()
            && with.log_namespace.is_some()
            && self.log_namespace != with.log_namespace
        {
            errors.push(
                format!("conflicting values for 'log_namespace' found. Both {:?} and {:?} used in the same component",
                        self.log_namespace(), with.log_namespace())
            );
        }
        if let Some(log_namespace) = with.log_namespace {
            self.log_namespace = Some(log_namespace);
        }

        // If either config enables these flags, it is enabled.
        self.enabled |= with.enabled;
        self.validation |= with.validation;
    }
}

impl Default for Options {
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            validation: default_validation(),
            log_namespace: None,
        }
    }
}

const fn default_enabled() -> bool {
    false
}

const fn default_validation() -> bool {
    false
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_append() {
        for (test, mut a, b, expected) in [
            (
                "enable log namespacing",
                Options {
                    enabled: false,
                    validation: false,
                    log_namespace: None,
                },
                Options {
                    enabled: false,
                    validation: false,
                    log_namespace: Some(true),
                },
                Some(Options {
                    enabled: false,
                    validation: false,
                    log_namespace: Some(true),
                }),
            ),
            (
                "log namespace conflict",
                Options {
                    enabled: false,
                    validation: false,
                    log_namespace: Some(false),
                },
                Options {
                    enabled: false,
                    validation: false,
                    log_namespace: Some(true),
                },
                None,
            ),
            (
                "enable schemas",
                Options {
                    enabled: false,
                    validation: false,
                    log_namespace: None,
                },
                Options {
                    enabled: true,
                    validation: false,
                    log_namespace: None,
                },
                Some(Options {
                    enabled: true,
                    validation: false,
                    log_namespace: None,
                }),
            ),
            (
                "enable sink requirements",
                Options {
                    enabled: false,
                    validation: false,
                    log_namespace: None,
                },
                Options {
                    enabled: false,
                    validation: true,
                    log_namespace: None,
                },
                Some(Options {
                    enabled: false,
                    validation: true,
                    log_namespace: None,
                }),
            ),
        ] {
            let mut errors = vec![];
            a.append(b, &mut errors);
            if errors.is_empty() {
                assert_eq!(Some(a), expected, "result mismatch: {}", test);
            } else {
                assert_eq!(
                    errors.is_empty(),
                    expected.is_some(),
                    "error mismatch: {}",
                    test
                );
            }
        }
    }
}
