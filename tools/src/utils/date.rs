use chrono::NaiveDate;

pub fn parse_date(input: &str) -> Option<NaiveDate> {
    let cleaned = input
        .replace(',', "")
        .split_whitespace()
        .map(|w| {
            if w.starts_with(char::is_numeric) {
                w.trim_end_matches(char::is_alphabetic)
            } else {
                w
            }
        })
        .collect::<Vec<_>>()
        .join(" ");

    const FORMATS: &[&str] = &[
        "%d %B %Y", "%B %d %Y", "%m-%d-%Y",
        "%Y-%m-%d",
        // ---
        "%d %b %Y", "%b %d %Y", "%d-%m-%Y",
        "%d/%m/%Y", "%m/%d/%Y", "%Y/%m/%d",
    ];

    FORMATS
        .iter()
        .find_map(|format| NaiveDate::parse_from_str(&cleaned, format).ok())
}
