use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use cron::Schedule;
use std::str::FromStr;

pub fn parse_schedule(value: &str) -> Result<Schedule, String> {
    let fields: Vec<_> = value.split_whitespace().collect();
    let normalized = match fields.len() {
        5 => format!("0 {value}"),
        6 | 7 => value.to_owned(),
        _ => return Err("schedule must contain 5, 6, or 7 cron fields".into()),
    };
    Schedule::from_str(&normalized).map_err(|error| format!("invalid schedule: {error}"))
}

pub fn is_due(schedule: &str, now: DateTime<Utc>, last_slot: Option<DateTime<Utc>>) -> bool {
    let Ok(schedule) = parse_schedule(schedule) else {
        return false;
    };
    let window_start = now - chrono::Duration::seconds(65);
    let Some(slot) = schedule.after(&window_start).next() else {
        return false;
    };
    slot <= now && last_slot.is_none_or(|last| slot > last)
}

pub fn is_due_in_timezone(
    schedule: &str,
    timezone: &str,
    now: DateTime<Utc>,
    last_slot: Option<DateTime<Utc>>,
) -> bool {
    let Ok(schedule) = parse_schedule(schedule) else {
        return false;
    };
    let Ok(timezone) = timezone.parse::<Tz>() else {
        return false;
    };
    let local_now = now.with_timezone(&timezone);
    let window_start = local_now - chrono::Duration::seconds(65);
    let Some(slot) = schedule.after(&window_start).next() else {
        return false;
    };
    slot <= local_now && last_slot.is_none_or(|last| slot.with_timezone(&Utc) > last)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn accepts_legacy_five_field_cron() {
        assert!(parse_schedule("5 * * * *").is_ok());
        assert!(parse_schedule("bad").is_err());
    }

    #[test]
    fn detects_due_slot_once() {
        let now = Utc.with_ymd_and_hms(2026, 8, 12, 10, 5, 10).unwrap();
        assert!(is_due("5 * * * *", now, None));
        assert!(!is_due("5 * * * *", now, Some(now)));
    }

    #[test]
    fn schedule_respects_named_timezone() {
        let now = Utc.with_ymd_and_hms(2026, 8, 12, 2, 5, 10).unwrap();
        assert!(is_due_in_timezone("5 10 * * *", "Asia/Shanghai", now, None));
        assert!(!is_due_in_timezone("5 10 * * *", "UTC", now, None));
    }
}
