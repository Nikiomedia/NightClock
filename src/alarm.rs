//! Weckzeit-Logik.

use crate::settings::Alarm;
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone};

fn is_one_shot(a: &Alarm) -> bool {
    !a.days.iter().any(|d| *d)
}

fn matches_day(a: &Alarm, date: NaiveDate) -> bool {
    is_one_shot(a) || a.days[date.weekday().num_days_from_monday() as usize]
}

fn at(date: NaiveDate, h: u32, m: u32) -> Option<DateTime<Local>> {
    let naive = date.and_hms_opt(h, m, 0)?;
    Local.from_local_datetime(&naive).earliest()
}

/// Nächster Weckzeitpunkt aller aktiven Wecker.
pub fn next_alarm(alarms: &[Alarm], now: DateTime<Local>) -> Option<(DateTime<Local>, usize)> {
    let mut best: Option<(DateTime<Local>, usize)> = None;
    for (i, a) in alarms.iter().enumerate() {
        if !a.enabled {
            continue;
        }
        for d in 0..8 {
            let date = now.date_naive() + Duration::days(d);
            if !matches_day(a, date) {
                continue;
            }
            if let Some(t) = at(date, a.hour, a.minute) {
                if t > now {
                    if best.is_none_or(|(b, _)| t < b) {
                        best = Some((t, i));
                    }
                    break;
                }
            }
        }
    }
    best
}

/// Ist zwischen `last` und `now` ein Wecker fällig geworden?
/// (Robust gegen ausgelassene Frames; nach mehr als 30 min wird nicht mehr nachgeholt.)
pub fn due_alarm(alarms: &[Alarm], last: DateTime<Local>, now: DateTime<Local>) -> Option<usize> {
    if now <= last {
        return None;
    }
    for (i, a) in alarms.iter().enumerate() {
        if !a.enabled {
            continue;
        }
        let mut dates = vec![last.date_naive()];
        if now.date_naive() != last.date_naive() {
            dates.push(now.date_naive());
        }
        for date in dates {
            if !matches_day(a, date) {
                continue;
            }
            if let Some(t) = at(date, a.hour, a.minute) {
                if t > last && t <= now && now - t < Duration::minutes(30) {
                    return Some(i);
                }
            }
        }
    }
    None
}

pub fn alarm_is_one_shot(a: &Alarm) -> bool {
    is_one_shot(a)
}

/// "heute", "morgen" oder Wochentag.
pub fn relative_day(at: DateTime<Local>, now: DateTime<Local>) -> String {
    use crate::i18n::t;
    let diff = (at.date_naive() - now.date_naive()).num_days();
    match diff {
        0 => t("heute", "today").into(),
        1 => t("morgen", "tomorrow").into(),
        _ => crate::i18n::weekday(at.weekday().num_days_from_monday() as usize).to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t(y: i32, mo: u32, d: u32, h: u32, mi: u32, s: u32) -> DateTime<Local> {
        Local.with_ymd_and_hms(y, mo, d, h, mi, s).unwrap()
    }

    fn alarm(h: u32, m: u32, days: [bool; 7]) -> Alarm {
        Alarm { enabled: true, hour: h, minute: m, days }
    }

    #[test]
    fn rings_when_crossing_alarm_time() {
        // Do, 12.11.2026
        let a = vec![alarm(7, 0, [true; 7])];
        assert_eq!(due_alarm(&a, t(2026, 11, 12, 6, 59, 59), t(2026, 11, 12, 7, 0, 0)), Some(0));
        assert_eq!(due_alarm(&a, t(2026, 11, 12, 7, 0, 0), t(2026, 11, 12, 7, 0, 1)), None);
        assert_eq!(due_alarm(&a, t(2026, 11, 12, 6, 0, 0), t(2026, 11, 12, 6, 59, 0)), None);
    }

    #[test]
    fn respects_weekdays() {
        // Werktags-Wecker klingelt nicht am Samstag (14.11.2026)
        let a = vec![alarm(7, 0, [true, true, true, true, true, false, false])];
        assert_eq!(due_alarm(&a, t(2026, 11, 14, 6, 59, 59), t(2026, 11, 14, 7, 0, 0)), None);
        assert_eq!(due_alarm(&a, t(2026, 11, 13, 6, 59, 59), t(2026, 11, 13, 7, 0, 0)), Some(0));
    }

    #[test]
    fn midnight_alarm_and_next() {
        let a = vec![alarm(0, 0, [true; 7])];
        assert_eq!(due_alarm(&a, t(2026, 11, 12, 23, 59, 59), t(2026, 11, 13, 0, 0, 0)), Some(0));
        let w = vec![alarm(7, 0, [true, true, true, true, true, false, false])];
        // Fr 22 Uhr -> nächster Wecker Mo 07:00
        let (n, _) = next_alarm(&w, t(2026, 11, 13, 22, 0, 0)).unwrap();
        assert_eq!(n, t(2026, 11, 16, 7, 0, 0));
    }

    #[test]
    fn disabled_alarm_never_rings() {
        let mut a = vec![alarm(7, 0, [true; 7])];
        a[0].enabled = false;
        assert_eq!(due_alarm(&a, t(2026, 11, 12, 6, 59, 59), t(2026, 11, 12, 7, 0, 0)), None);
        assert!(next_alarm(&a, t(2026, 11, 12, 6, 0, 0)).is_none());
    }
}
