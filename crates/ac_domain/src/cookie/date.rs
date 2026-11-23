//! The date of an `Expires` attribute, read as leniently as browsers do
//! (RFC 6265 section 5.1.1): `Sun, 06 Nov 1994 08:49:37 GMT`, also
//! `Sunday, 06-Nov-94 08:49:37 GMT` and other orders.

use super::UnixSeconds;

const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];

/// Seconds since 1970 for a cookie date; `None` if it is none.
#[must_use]
pub fn parse_cookie_date(text: &str) -> Option<UnixSeconds> {
    let mut time = None;
    let mut day = None;
    let mut month = None;
    let mut year = None;
    for token in text.split(is_delimiter).filter(|token| !token.is_empty()) {
        if time.is_none()
            && let Some(found) = time_of(token)
        {
            time = Some(found);
        } else if day.is_none()
            && let Some(found) = leading_number(token, 1, 2)
        {
            day = Some(found);
        } else if month.is_none()
            && let Some(found) = month_of(token)
        {
            month = Some(found);
        } else if year.is_none()
            && let Some(found) = leading_number(token, 2, 4)
        {
            year = Some(found);
        }
    }
    let (hour, minute, second) = time?;
    let (day, month) = (day?, month?);
    let year = match year? {
        year @ 70..=99 => year + 1900,
        year @ 0..=69 => year + 2000,
        year => year,
    };
    if !(1..=31).contains(&day) || year < 1601 || hour > 23 || minute > 59 || second > 59 {
        return None;
    }
    let days = days_from_civil(year, month, day);
    Some(days * 86_400 + hour * 3_600 + minute * 60 + second)
}

/// RFC 6265: tab, space to `/`, `;` to `@`, `[` to `` ` ``, `{` to `~`.
fn is_delimiter(c: char) -> bool {
    matches!(c, '\t' | ' '..='/' | ';'..='@' | '['..='`' | '{'..='~')
}

/// `hh:mm:ss` (one or two digits each).
fn time_of(token: &str) -> Option<(i64, i64, i64)> {
    let mut fields = token.split(':');
    let hour = leading_number(fields.next()?, 1, 2)?;
    let minute = leading_number(fields.next()?, 1, 2)?;
    let second = leading_number(fields.next()?, 1, 2)?;
    Some((hour, minute, second))
}

/// A number of `min`–`max` digits at the start, followed by no digit.
fn leading_number(token: &str, min: usize, max: usize) -> Option<i64> {
    let digits = token.bytes().take_while(u8::is_ascii_digit).count();
    if !(min..=max).contains(&digits) {
        return None;
    }
    token[..digits].parse().ok()
}

fn month_of(token: &str) -> Option<i64> {
    let start = token.get(..3)?.to_ascii_lowercase();
    MONTHS
        .iter()
        .position(|month| *month == start)
        .and_then(|index| i64::try_from(index + 1).ok())
}

/// Days since 1970-01-01 (Howard Hinnant's `days_from_civil`).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_index = (month + 9) % 12;
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}
