use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use cron::Schedule;
use std::str::FromStr;

pub fn parse_schedule(value: &str) -> Result<Schedule, String> {
    if value.len() > 256 {
        return Err("schedule cannot exceed 256 bytes".into());
    }
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
    let search_start = last_slot
        .filter(|last| *last > window_start)
        .unwrap_or(window_start);
    let Some(slot) = schedule.after(&search_start).next() else {
        return false;
    };
    slot <= now
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
    let window_start = now - chrono::Duration::seconds(65);
    let search_start = last_slot
        .filter(|last| *last > window_start)
        .unwrap_or(window_start)
        .with_timezone(&timezone);
    let Some(slot) = schedule.after(&search_start).next() else {
        return false;
    };
    slot <= local_now
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn accepts_legacy_five_field_cron() {
        assert!(parse_schedule("5 * * * *").is_ok());
        assert!(parse_schedule("bad").is_err());
        assert_eq!(
            parse_schedule(&format!("{} * * * *", "0,".repeat(126))).unwrap_err(),
            "schedule cannot exceed 256 bytes"
        );
    }

    #[test]
    fn detects_due_slot_once() {
        let now = Utc.with_ymd_and_hms(2026, 8, 12, 10, 5, 10).unwrap();
        assert!(is_due("5 * * * *", now, None));
        assert!(!is_due("5 * * * *", now, Some(now)));
    }

    #[test]
    fn detects_the_next_high_frequency_slot_after_last_check() {
        let first = Utc.with_ymd_and_hms(2026, 8, 12, 10, 5, 20).unwrap();
        let before_next = Utc.with_ymd_and_hms(2026, 8, 12, 10, 5, 39).unwrap();
        let next = Utc.with_ymd_and_hms(2026, 8, 12, 10, 5, 40).unwrap();

        assert!(!is_due("0/20 * * * * *", before_next, Some(first)));
        assert!(is_due("0/20 * * * * *", next, Some(first)));
    }

    #[test]
    fn does_not_catch_up_a_stale_slot_after_a_long_pause() {
        let last = Utc.with_ymd_and_hms(2026, 8, 11, 10, 0, 0).unwrap();
        let now = Utc.with_ymd_and_hms(2026, 8, 12, 10, 5, 0).unwrap();

        assert!(!is_due("0 0 10 * * *", now, Some(last)));
        assert!(!is_due_in_timezone("0 0 10 * * *", "UTC", now, Some(last)));
    }

    #[test]
    fn schedule_respects_named_timezone() {
        let now = Utc.with_ymd_and_hms(2026, 8, 12, 2, 5, 10).unwrap();
        assert!(is_due_in_timezone("5 10 * * *", "Asia/Shanghai", now, None));
        assert!(!is_due_in_timezone("5 10 * * *", "UTC", now, None));
    }

    #[test]
    fn accepts_simple_schedule_expressions() {
        for value in [
            "0 30 2 * * *",
            "0 30 2 * * MON",
            "0 30 2 15 * *",
            "0 0 0/6 * * *",
            "0 0/15 * * * *",
            "0/20 * * * * *",
        ] {
            assert!(parse_schedule(value).is_ok(), "{value}");
        }
    }

    #[test]
    fn simple_schedules_fire_on_expected_slots() {
        let monday = Utc.with_ymd_and_hms(2026, 8, 17, 2, 30, 10).unwrap();
        assert!(is_due_in_timezone("0 30 2 * * MON", "UTC", monday, None));
        assert!(!is_due_in_timezone("0 30 2 * * TUE", "UTC", monday, None));

        let monthly = Utc.with_ymd_and_hms(2026, 8, 15, 2, 30, 10).unwrap();
        assert!(is_due_in_timezone("0 30 2 15 * *", "UTC", monthly, None));

        let hours = Utc.with_ymd_and_hms(2026, 8, 17, 12, 0, 10).unwrap();
        let minutes = Utc.with_ymd_and_hms(2026, 8, 17, 12, 15, 10).unwrap();
        let seconds = Utc.with_ymd_and_hms(2026, 8, 17, 12, 15, 20).unwrap();
        assert!(is_due_in_timezone("0 0 0/6 * * *", "UTC", hours, None));
        assert!(is_due_in_timezone("0 0/15 * * * *", "UTC", minutes, None));
        assert!(is_due_in_timezone("0/20 * * * * *", "UTC", seconds, None));
    }
}
