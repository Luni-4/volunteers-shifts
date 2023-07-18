use chrono::{DateTime, Datelike, Duration, NaiveDateTime, TimeZone, Utc, Weekday};
use chrono_tz::Europe::Rome;

// Italian days
pub(crate) const ITALIAN_DAYS: [&str; 6] = [
    "Lunedì",
    "Martedì",
    "Mercoledì",
    "Giovedì",
    "Venerdì",
    "Sabato",
];

// Italian months acronyms
const ITALIAN_MONTHS_ACRONYMS: [&str; 12] = [
    "Gen", "Feb", "Mar", "Apr", "Mag", "Giu", "Lug", "Ago", "Set", "Ott", "Nov", "Dic",
];

// Italian months acronyms
const ITALIAN_MONTHS: [&str; 12] = [
    "Gennaio",
    "Febbraio",
    "Marzo",
    "Aprile",
    "Maggio",
    "Giugno",
    "Luglio",
    "Agosto",
    "Settembre",
    "Ottobre",
    "Novembre",
    "Dicembre",
];

// Returns Italian timezone
fn italian_timezone() -> DateTime<chrono_tz::Tz> {
    // Current UTC date and time
    let utc = Utc::now();
    // Current UTC date as NaiveDate
    let current_date = utc.date_naive();
    // Current UTC time as NaiveTime
    let current_time = utc.time();
    // Italian date and time
    Rome.from_utc_datetime(&NaiveDateTime::new(current_date, current_time))
}

// Date structure
pub(crate) struct Date(DateTime<chrono_tz::Tz>);

impl Date {
    // Get current date
    #[inline(always)]
    pub(crate) fn current() -> Self {
        let date = Self(italian_timezone());
        // If it is Sunday, skip to next Monday
        if date.day_as_number() == 6 {
            Self(date.0 + Duration::days(1))
        } else {
            date
        }
    }

    // Get next week date
    #[inline(always)]
    pub(crate) fn next_week(&self) -> Self {
        Self(self.0 + Duration::days(7))
    }

    // Get Monday of a date
    #[inline(always)]
    pub(crate) fn monday(&self) -> Self {
        Self(self.0 - Duration::days(self.day_as_number() as i64))
    }

    // Formats a date
    #[inline(always)]
    pub(crate) fn month_date(&self) -> String {
        format!(
            "{} {}",
            self.0.format("%d"),
            ITALIAN_MONTHS[self.0.month0() as usize]
        )
    }

    // Retrieves week bounds
    #[inline(always)]
    pub(crate) fn week_bounds(&self) -> (String, String) {
        // Monday
        let monday = self.monday().0;
        // Saturday
        let saturday = monday + Duration::days(5);

        (
            Self::format_week_date(monday),
            Self::format_week_date(saturday),
        )
    }

    // Retrieves the number of missing days and dates from current date
    // to Saturday
    #[inline(always)]
    pub(crate) fn now_until_saturday(&self) -> Vec<Self> {
        // Retrieves the number of missing days and dates from current weekday
        // until Saturday, and then formats dates in a specific way
        (0..(Weekday::Sun.num_days_from_monday() - self.day_as_number()))
            .map(|day| {
                let date = self.0 + Duration::days(day as i64);
                Self(date)
            })
            .collect()
    }

    // Gets the day associated to a date
    #[inline(always)]
    pub(crate) fn weekday(&self) -> &'static str {
        ITALIAN_DAYS[self.day_as_number() as usize]
    }

    // Gets day associated to a date
    #[inline(always)]
    pub(crate) fn day(&self) -> u32 {
        self.0.day()
    }

    // Gets month associated to a date
    #[inline(always)]
    pub(crate) fn month(&self) -> u32 {
        self.0.month()
    }

    // Gets year associated to a date
    #[inline(always)]
    pub(crate) fn year(&self) -> i32 {
        self.0.year()
    }

    // Gets day as number days from Monday (excluding Sunday)
    #[inline(always)]
    pub(crate) fn day_as_number(&self) -> u32 {
        self.0.weekday().num_days_from_monday()
    }

    // Returns a week day formatted as day and date
    #[inline(always)]
    pub(crate) fn day_date(&self, day: u32) -> (i32, u32, u32) {
        let date = Self(self.monday().0 + Duration::days(day as i64));
        (date.year(), date.month(), date.day())
    }

    // Formats a date
    #[inline(always)]
    fn format_week_date<T: TimeZone>(date: DateTime<T>) -> String
    where
        T::Offset: core::fmt::Display,
    {
        format!(
            "{} {}.",
            date.format("%d"),
            ITALIAN_MONTHS_ACRONYMS[date.month0() as usize]
        )
    }
}
