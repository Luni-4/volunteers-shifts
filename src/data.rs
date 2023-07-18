use std::collections::HashSet;

use serde::Serialize;

use crate::database::{insert_db_date, Shift};
use crate::time::Date;

// Fake values for explaining options in selects
pub(crate) const FAKE_DAY_VALUE: u32 = 100;
pub(crate) const FAKE_TASKS_VALUE: i16 = 100;

// Input type number errors
#[derive(Serialize)]
pub(crate) struct InputTypeNumberErrors {
    // Missing value
    missing: &'static str,
    // Invalid value
    invalid: &'static str,
    // Underflow
    underflow: &'static str,
    // Overflow
    overflow: &'static str,
}

impl InputTypeNumberErrors {
    pub(crate) fn text() -> Self {
        Self {
            missing: "Inserire un numero di tessera senza spazi iniziali e finali.",
            invalid: "Inserire un numero di tessera come nell'esempio.",
            underflow: "Il numero di tessera deve essere positivo.",
            overflow: "Il numero di tessera deve essere inferiore o uguale a 100.",
        }
    }
}

// Button text
#[derive(Serialize)]
pub(crate) struct Button {
    // Button text
    text: &'static str,
}

impl Button {
    pub(crate) fn authentication_button() -> Self {
        Self { text: "Accedi" }
    }
    pub(crate) fn shifts_buttons() -> Self {
        Self {
            text: "Salva turni",
        }
    }

    pub(crate) fn add_shift() -> Self {
        Self {
            text: "Aggiungi turno",
        }
    }
}

// Tasks
#[derive(Serialize)]
pub(crate) struct Tasks {
    pub(crate) task_value: usize,
    pub(crate) task_name: &'static str,
    pub(crate) task_hours: &'static str,
}

impl Tasks {
    const TASKS: &'static [Self] = &[
        Tasks::new(0, "Aiuto Cucina", "10:00-14:00"),
        Tasks::new(1, "Accoglienza", "11:30-13:30"),
        Tasks::new(2, "Servizio tavoli", "11:00-14:00"),
        Tasks::new(3, "Pomeriggio", "14:00-16:00"),
        Tasks::new(4, "Accoglienza notturna", "19:00-21:00"),
    ];

    const fn new(task_value: usize, task_name: &'static str, task_hours: &'static str) -> Self {
        Self {
            task_value,
            task_name,
            task_hours,
        }
    }

    pub(crate) const fn render() -> &'static [Self] {
        Self::TASKS
    }

    pub(crate) fn task_from_id(id: usize) -> &'static str {
        Self::TASKS[id].task_name
    }

    pub(crate) fn hours_from_id(id: usize) -> &'static str {
        Self::TASKS[id].task_hours
    }
}

// Day expressed as number and text
#[derive(Serialize)]
pub(crate) struct Day {
    // Day as number
    day_value: u32,
    // Day as text
    day_text: String,
}

impl Day {
    fn new(date: Date) -> Self {
        Self {
            day_value: date.day_as_number(),
            day_text: format!("{} {}", date.weekday(), date.month_date()),
        }
    }
}

// Iterate over days
#[inline]
fn iterate_over_days<F, T>(date: &Date, fill_struct: F) -> Vec<T>
where
    F: Fn(Date) -> T,
{
    date.now_until_saturday()
        .into_iter()
        .map(fill_struct)
        .collect()
}

// Weeks data
#[derive(Serialize)]
pub(crate) struct WeekData {
    // Week bounds
    week_bounds: String,
    // Week days
    week_days: Vec<Day>,
}

impl WeekData {
    pub(crate) fn week_info(date: &Date) -> Self {
        Self {
            week_bounds: Self::week_bounds(date),
            week_days: Self::days(date),
        }
    }

    pub(crate) fn week_bounds(date: &Date) -> String {
        let (monday, saturday) = date.week_bounds();
        format!("{monday} - {saturday}")
    }

    pub(crate) fn days(date: &Date) -> Vec<Day> {
        iterate_over_days(date, Day::new)
    }
}

// Day information
#[derive(Serialize)]
pub(crate) struct SelectDay {
    // Whether the day is selected
    is_selected: Option<&'static str>,
    // Day as number and text
    day: Day,
}

impl SelectDay {
    pub(crate) fn selected_days(date: &Date, current_day_as_number: u32) -> Vec<Self> {
        iterate_over_days(date, |date| {
            let day = Day::new(date);
            Self {
                is_selected: (day.day_value == current_day_as_number).then_some("selected"),
                day,
            }
        })
    }

    pub(crate) fn days(date: &Date) -> Vec<Self> {
        iterate_over_days(date, |date| Self {
            is_selected: None,
            day: Day::new(date),
        })
    }
}

// Shift labels
#[derive(Serialize)]
pub(crate) struct ShiftLabels {
    // Week name label
    week_name: &'static str,
    // Week date label
    week_date_label: &'static str,
    // Task label
    task_label: &'static str,
}

impl ShiftLabels {
    pub(crate) fn render() -> Self {
        Self {
            week_name: "Scegli settimana",
            week_date_label: "Scegli giorno",
            task_label: "Scegli mansione",
        }
    }
}

#[derive(FromForm)]
pub(crate) struct ShiftsData {
    pub(crate) card_id: i16,
    #[field(name = "week")]
    pub(crate) weeks: Vec<bool>,
    #[field(name = "dates")]
    pub(crate) days: Vec<u32>,
    #[field(name = "tasks")]
    pub(crate) tasks: Vec<i16>,
}

impl ShiftsData {
    // Create all shifts to be inserted into the database
    pub(crate) fn create_shifts(&self, already_saved_shifts: HashSet<Shift>) -> HashSet<Shift> {
        let mut shifts = HashSet::new();
        for (week, (day, task)) in self
            .weeks
            .iter()
            .zip(self.days.chunks(2).zip(self.tasks.iter()))
        {
            // Skip fake value used to help a volunteer in discriminate
            // among already compiled shifts and new ones. 100 is
            // a symbolic value.
            if *task == FAKE_TASKS_VALUE {
                continue;
            }
            let day = if *week { day[0] } else { day[1] };
            if day == FAKE_DAY_VALUE {
                continue;
            }
            let current_date = Date::current();
            let date = if *week {
                current_date.day_date(day)
            } else {
                current_date.next_week().day_date(day)
            };
            // If there is an error in inserting the date, skip the shift
            let shift = if let Some(date) = insert_db_date(date) {
                Shift {
                    date,
                    task: *task,
                    card_id: self.card_id,
                }
            } else {
                continue;
            };
            if !already_saved_shifts.contains(&shift) {
                shifts.insert(shift);
            }
        }
        shifts
    }
}
