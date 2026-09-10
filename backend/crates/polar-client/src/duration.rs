//! ISO 8601 -kestojen jäsennys (`PT2H44M45S`, `PT1H2M3.5S`, `P1DT2H`).
//!
//! Polar antaa kestot tässä muodossa; kantaan tallennetaan sekunteja.

/// Palauttaa keston sekunteina pyöristettynä alaspäin, tai `None` jos
/// merkkijono ei ole kelvollinen kesto.
pub fn parse_iso8601_seconds(input: &str) -> Option<i64> {
    let s = input.trim();
    let rest = s.strip_prefix('P')?;
    let (date_part, time_part) = match rest.split_once('T') {
        Some((d, t)) => (d, Some(t)),
        None => (rest, None),
    };

    let mut total = 0.0_f64;
    total += parse_components(date_part, &[('D', 86_400.0), ('W', 604_800.0)])?;
    if let Some(t) = time_part {
        if t.is_empty() {
            return None;
        }
        total += parse_components(t, &[('H', 3_600.0), ('M', 60.0), ('S', 1.0)])?;
    }
    if total.is_finite() && total >= 0.0 {
        Some(total.floor() as i64)
    } else {
        None
    }
}

fn parse_components(part: &str, units: &[(char, f64)]) -> Option<f64> {
    let mut total = 0.0;
    let mut number = String::new();
    for ch in part.chars() {
        if ch.is_ascii_digit() || ch == '.' || ch == ',' {
            number.push(if ch == ',' { '.' } else { ch });
            continue;
        }
        let factor = units.iter().find(|(u, _)| *u == ch)?.1;
        if number.is_empty() {
            return None;
        }
        total += number.parse::<f64>().ok()? * factor;
        number.clear();
    }
    if !number.is_empty() {
        return None; // luku ilman yksikköä
    }
    Some(total)
}

#[cfg(test)]
mod tests {
    use super::parse_iso8601_seconds as p;

    #[test]
    fn parses_common_polar_durations() {
        assert_eq!(p("PT2H44M45S"), Some(2 * 3600 + 44 * 60 + 45));
        assert_eq!(p("PT2H44M"), Some(2 * 3600 + 44 * 60));
        assert_eq!(p("PT3H11M"), Some(3 * 3600 + 11 * 60));
        assert_eq!(p("PT18H23M30S"), Some(18 * 3600 + 23 * 60 + 30));
        assert_eq!(p("PT8H"), Some(8 * 3600));
        assert_eq!(p("PT45.5S"), Some(45));
        assert_eq!(p("PT0S"), Some(0));
        assert_eq!(p("P1DT2H"), Some(86_400 + 7_200));
        assert_eq!(p("PT1H2M3,5S"), Some(3_723));
    }

    #[test]
    fn rejects_garbage() {
        assert_eq!(p(""), None);
        assert_eq!(p("2h"), None);
        assert_eq!(p("PT"), None);
        assert_eq!(p("PT5"), None);
        assert_eq!(p("PTXS"), None);
    }
}
