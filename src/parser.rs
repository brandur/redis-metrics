use std::str;

#[derive(Debug, PartialEq)]
pub struct Metric {
    name: String,
    value: String,
    metric_type: String,
}

named!(pub statsd<Metric>,
    chain!(
        name: map_res!(is_not!(":"), str::from_utf8) ~
        tag!(":") ~
        value: map_res!(is_not!("|"), str::from_utf8) ~
        tag!("|") ~
        metric_type: map_res!(is_not!("\n"), str::from_utf8) ~
        opt!(
            tag!("")
        ),
        || {Metric{
            name: String::from(name),
            value: String::from(value),
            metric_type: String::from(metric_type),
        }}
    )
);

#[cfg(test)]
mod tests {
    use nom::IResult;
    use super::*;

    #[test]
    fn it_parses_metrics() {
        assert_eq!(statsd(b"gorets:1|c"), IResult::Done(&b""[..], Metric{
            name: String::from("gorets"),
            value: String::from("1"),
            metric_type: String::from("c")
        }));
    }
}
