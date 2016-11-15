//! Implements a parser using nom for StatsD metrics including counters,
//! gauges, samples, and sets. See [this document][metric-types] for more
//! details. Some examples of input that this package will parse are:
//!
//!     gorets:1|c
//!     glork:320|ms|@0.1
//!     gaugor:333|g
//!     uniques:765|s
//!
//! See the tests for example, but generally speaking, the `statsd` macro is
//! the only thing that needs to be used from this package.
//!
//! [metric-types]: https://github.com/etsy/statsd/blob/master/docs/metric_types.md

use nom;
use std::str;
use std::str::FromStr;

/// Metric represents a single emitted metric including a name, value, and type
#[derive(Debug, PartialEq)]
pub struct Metric {
    /// The metric's name.
    name: String,

    /// The metric's value.
    value: String,

    /// Type of the metric (e.g. counter, gauge, ...).
    metric_type: MetricType,

    /// Unit is the unit of measurement of a sample (e.g. "ms"). It has a value
    /// for samples, but is `None` for all other metric types.
    unit: Option<String>,

    /// The frequency at which the metric is being sampled, expressed as a
    /// fraction of the per period time (e.g. 0.1 means that the metric is
    /// being sent sampled every 1/10th of the time). Only applies to counters
    /// and samples, and is an optional value even in both those cases.
    sample_rate: Option<f64>,
}

/// All possible types of a metric.
#[derive(Debug, PartialEq)]
pub enum MetricType {
    /// Counter add the value sent with the metric to a bucket as a new
    /// increment.
    Counter,

    /// Gauges are arbitrary values that can be recorded and then added to or
    /// subtracted from with a subsequent signed metric.
    Gauge,

    /// Samples represent timing. For each flush interval the backend will
    /// calculated aggregates like percentiles, mean, standard deviation, etc.
    Sample,

    /// Sets track unique occurrences of values between flushes.
    Set,
}

/// The sample rate within a metric (i.e. the "|@0.1" that a counter or sample
/// may be suffixed with).
named!(sample_rate<f64>,
    chain!(
        tag!("|@") ~
        n: map_res!(map_res!(is_not!("\n"), str::from_utf8), f64::from_str)
        , || n
    )
);

/// Parses a set of metrics that are delimited with a "\n". This is a standard
/// allowed case by StatsD so this should be the only parser from this package
/// that's used.
named!(pub statsd<Vec<Metric> >,
    many1!(
        chain!(
            m: statsd_metric ~
            opt!(complete!(tag!("\n")))
            , || m
        )
    )
);

/// Parses a single StatsD-style metric. The `statsd` metric should be used
/// instead in most cases.
named!(pub statsd_metric<Metric>,
    chain!(
        name: map_res!(is_not!(":"), str::from_utf8) ~
        tag!(":") ~
        value: map_res!(is_not!("|"), str::from_utf8) ~
        tag!("|") ~
        type_or_unit: map_res!(nom::alphanumeric, str::from_utf8) ~
        sample_rate: opt!(complete!(sample_rate))
        ,
        || {Metric{
            name: String::from(name),
            value: String::from(value),
            metric_type: parse_metric_type(type_or_unit),
            unit: parse_unit(type_or_unit),
            sample_rate: sample_rate,
        }}
    )
);

fn parse_metric_type(s: &str) -> MetricType {
    match s {
        "c" => MetricType::Counter,
        "g" => MetricType::Gauge,
        "s" => MetricType::Set,
        _ => MetricType::Sample,
    }
}

fn parse_unit(s: &str) -> Option<String> {
    match s {
        "c" => None,
        "g" => None,
        "s" => None,
        a => Some(String::from(a)),
    }
}

#[cfg(test)]
mod tests {
    use nom::IResult;
    use super::*;

    #[test]
    fn it_parses_counter() {
        assert_eq!(statsd_metric(b"gorets:1|c"), IResult::Done(&b""[..], Metric{
            name: String::from("gorets"),
            value: String::from("1"),
            metric_type: MetricType::Counter,
            unit: None,
            sample_rate: None,
        }));
    }

    #[test]
    fn it_parses_counter_with_sample_rate() {
        assert_eq!(statsd_metric(b"gorets:1|c|@0.1"), IResult::Done(&b""[..], Metric{
            name: String::from("gorets"),
            value: String::from("1"),
            metric_type: MetricType::Counter,
            unit: None,
            sample_rate: Some(0.1),
        }));
    }

    #[test]
    fn it_parses_sample() {
        assert_eq!(statsd_metric(b"glork:320|ms"), IResult::Done(&b""[..], Metric{
            name: String::from("glork"),
            value: String::from("320"),
            metric_type: MetricType::Sample,
            unit: Some(String::from("ms")),
            sample_rate: None,
        }));
    }

    #[test]
    fn it_parses_sample_with_sample_rate() {
        assert_eq!(statsd_metric(b"glork:320|ms|@0.1"), IResult::Done(&b""[..], Metric{
            name: String::from("glork"),
            value: String::from("320"),
            metric_type: MetricType::Sample,
            unit: Some(String::from("ms")),
            sample_rate: Some(0.1),
        }));
    }

    #[test]
    fn it_parses_gauge() {
        assert_eq!(statsd_metric(b"gaugor:333|g"), IResult::Done(&b""[..], Metric{
            name: String::from("gaugor"),
            value: String::from("333"),
            metric_type: MetricType::Gauge,
            unit: None,
            sample_rate: None,
        }));
    }

    #[test]
    fn it_parses_set() {
        assert_eq!(statsd_metric(b"uniques:765|s"), IResult::Done(&b""[..], Metric{
            name: String::from("uniques"),
            value: String::from("765"),
            metric_type: MetricType::Set,
            unit: None,
            sample_rate: None,
        }));
    }

    #[test]
    fn it_parses_single_metric_with_statsd() {
        assert_eq!(statsd(b"gorets:1|c"), IResult::Done(&b""[..], vec![
            Metric{
                name: String::from("gorets"),
                value: String::from("1"),
                metric_type: MetricType::Counter,
                unit: None,
                sample_rate: None,
            }
        ]));
    }

    #[test]
    fn it_parses_multiple_metrics_with_statsd() {
        let data = b"gorets:1|c\nglork:320|ms\ngaugor:333|g\nuniques:765|s";
        assert_eq!(statsd(data), IResult::Done(&b""[..], vec![
            Metric{
                name: String::from("gorets"),
                value: String::from("1"),
                metric_type: MetricType::Counter,
                unit: None,
                sample_rate: None,
            },
            Metric{
                name: String::from("glork"),
                value: String::from("320"),
                metric_type: MetricType::Sample,
                unit: Some(String::from("ms")),
                sample_rate: None,
            },
            Metric{
                name: String::from("gaugor"),
                value: String::from("333"),
                metric_type: MetricType::Gauge,
                unit: None,
                sample_rate: None,
            },
            Metric{
                name: String::from("uniques"),
                value: String::from("765"),
                metric_type: MetricType::Set,
                unit: None,
                sample_rate: None,
            },
        ]))
    }
}
