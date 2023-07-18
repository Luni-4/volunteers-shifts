use chrono::format::strftime::StrftimeItems;
use chrono::format::DelayedFormat;
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

    // Retrieves week date bounds
    pub(crate) fn week_bounds(&self) -> String {
        // Monday
        let monday = self.monday().0;
        // Saturday
        let saturday = monday + Duration::days(5);
        format!(
            "{} - {}",
            Self::format_week_date(monday),
            Self::format_week_date(saturday)
        )
    }

    // Retrieves the number of missing days and dates from current date
    // to Saturday
    pub(crate) fn now_until_saturday(&self) -> Vec<Self> {
        // Retrieves the number of missing days and dates from current weekday
        // until Saturday, and then formats dates in a specific way
        (0..(Weekday::Sun.num_days_from_monday() - self.day_as_number() as u32))
            .map(|day| {
                let date = self.0 + Duration::days(day as i64);
                Self(date)
            })
            .collect()
    }

    // Get the day associated to a date
    pub(crate) fn day(&self) -> &'static str {
        ITALIAN_DAYS[self.day_as_number()]
    }

    // Get day as number days from Monday (excludes Sunday)
    pub(crate) fn day_as_number(&self) -> usize {
        self.0.weekday().num_days_from_monday() as usize
    }

    // Get the date
    pub(crate) fn date(&self) -> String {
        self.0.format("%d/%m/%Y").to_string()
    }

    // Retrieve and format the requested week day as day and date
    pub(crate) fn week_day(&self, day: usize) -> (&'static str, String) {
        let date = Self(self.monday().0 + Duration::days(day as i64));
        (date.day(), date.date())
    }

    // Format a date
    #[inline(always)]
    fn format_week_date<'a, T: TimeZone>(date: DateTime<T>) -> DelayedFormat<StrftimeItems<'a>>
    where
        T::Offset: core::fmt::Display,
    {
        date.format("%d/%m/%Y")
    }
}
