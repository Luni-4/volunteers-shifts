use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

use serde::Serialize;

use crate::database::{Shift, ShiftQuery};
use crate::time::{Date, ITALIAN_DAYS};

#[inline(always)]
pub(crate) fn string_len_as_number(len: usize) -> String {
    "0".repeat(len)
}

// Tables headers for shifts
#[derive(Serialize)]
pub(crate) struct ShiftsTableHeaders {
    // Description of the handle which selects a shift for a day
    select: &'static str,
    // Date
    date: &'static str,
    // Task
    task: &'static str,
    // Entrance hour
    entrance_hour: &'static str,
    // Exit hour
    exit_hour: &'static str,
    // Add/Remove shift command
    shift: &'static str,
    // Delete shift command
    delete_shift: &'static str,
}

// Button text
#[derive(Serialize)]
pub(crate) struct Button {
    // Send button text
    send_text: &'static str,
}

impl Button {
    pub(crate) fn authentication_button() -> Self {
        Self {
            send_text: "Accedi",
        }
    }
    pub(crate) fn shifts_buttons() -> Self {
        Self {
            send_text: "Invia turni",
        }
    }
}

impl ShiftsTableHeaders {
    pub(crate) fn text() -> Self {
        Self {
            select: "Seleziona giorno",
            date: "Data",
            task: "Mansione",
            entrance_hour: "Ora di entrata",
            exit_hour: "Ora di uscita",
            shift: "Aggiungi/Rimuovi",
            delete_shift: "Cancella",
        }
    }
}

// Shifts route and data
#[derive(Serialize)]
pub(crate) struct BookedShiftsData {
    route: String,
    data: ShiftQuery,
}

impl BookedShiftsData {
    pub(crate) fn new(route: String, data: ShiftQuery) -> Self {
        Self { route, data }
    }
}

// Identifier and names for the form fields
#[derive(Serialize)]
struct FormIds {
    // Checkbox id and name
    checkbox_id: String,
    // Checkbox value
    checkbox_value: String,
    // Tasks
    tasks: String,
    // Entrance hour
    entrance_hour: String,
    // Exit hour
    exit_hour: String,
}

impl FormIds {
    fn new(prefix: &str, day: &Day, index: String) -> Self {
        Self {
            checkbox_id: FormData::checkbox(prefix, day.day_as_number),
            checkbox_value: day.date.to_string(),
            tasks: FormData::tasks(prefix, day.day, &index),
            entrance_hour: FormData::entrance_hour(prefix, day.day, &index),
            exit_hour: FormData::exit_hour(prefix, day.day, &index),
        }
    }
}

// Shift day and date
#[derive(Serialize)]
struct Day {
    day_as_number: usize,
    day: &'static str,
    date: String,
}

impl Day {
    fn new(date: &Date) -> Self {
        Self {
            day_as_number: date.day_as_number(),
            day: date.day(),
            date: date.date(),
        }
    }
}

// Tasks
#[derive(Serialize)]
pub(crate) struct Tasks;

impl Tasks {
    const TASKS: &'static [&'static str] = &["Cucina", "Sala", "Camere", "Giardino"];

    pub(crate) fn tasks_as_str() -> &'static [&'static str] {
        Self::TASKS
    }
}

// Shift hours
#[derive(Serialize)]
pub(crate) struct Hours {
    value: usize,
    show: &'static str,
}

impl Hours {
    const HOURS: &'static [&'static str] = &[
        "08:00", "09:00", "10:00", "11:00", "12:00", "13:00", "14:00", "15:00", "16:00", "17:00",
        "18:00",
    ];

    pub(crate) fn new(value: usize) -> Self {
        Self {
            value,
            show: Self::HOURS[value],
        }
    }

    // List of entrance hours
    fn entrance_hours() -> [Hours; 10] {
        [
            Hours::new(0),
            Hours::new(1),
            Hours::new(2),
            Hours::new(3),
            Hours::new(4),
            Hours::new(5),
            Hours::new(6),
            Hours::new(7),
            Hours::new(8),
            Hours::new(9),
        ]
    }

    // List of exit hours
    fn exit_hours() -> [Hours; 10] {
        [
            Hours::new(1),
            Hours::new(2),
            Hours::new(3),
            Hours::new(4),
            Hours::new(5),
            Hours::new(6),
            Hours::new(7),
            Hours::new(8),
            Hours::new(9),
            Hours::new(10),
        ]
    }

    pub(crate) fn split(
        entrance_hour: usize,
        exit_hour: usize,
    ) -> Box<dyn Iterator<Item = (String, String)>> {
        Box::new((entrance_hour..(exit_hour - entrance_hour)).map(|hour| {
            (
                Self::HOURS[hour].to_string(),
                Self::HOURS[hour + 1].to_string(),
            )
        }))
    }

    pub(crate) fn intervals(
    ) -> Box<dyn Iterator<Item = (&'static str, &'static str)> + Send + 'static> {
        Box::new(Self::HOURS.windows(2).map(|hours| (hours[0], hours[1])))
    }

    pub(crate) fn identical_hours(entrance_hour: usize, exit_hour: usize) -> String {
        format!(
            "L'ora di entrata \"{}\" è uguale all'ora di uscita \"{}\"",
            Self::HOURS[entrance_hour],
            Self::HOURS[exit_hour]
        )
    }

    pub(crate) fn entrance_is_greater(entrance_hour: usize, exit_hour: usize) -> String {
        format!(
            "L'ora di entrata \"{}\" è maggiore dell'ora di uscita \"{}\"",
            Self::HOURS[entrance_hour],
            Self::HOURS[exit_hour]
        )
    }
}

// A day shift
#[derive(Serialize)]
pub(crate) struct DayShift {
    // Route to add shifts
    add_route: String,
    // Route to remove shifts
    remove_route: String,
    // Button id to add shifts
    add_button_id: String,
    // Button id to remove shifts
    remove_button_id: String,
    // Day information associated to the shift
    date: Option<Day>,
    // Whether the shift is the last one of a day
    last_shift: bool,
    // Format identifier
    form_id: FormIds,
    // Tasks
    tasks: &'static [&'static str],
    // Entrance hours
    entrance_hours: [Hours; 10],
    // Exit hours
    exit_hours: [Hours; 10],
}

impl DayShift {
    // Creates a new day shift
    pub(crate) fn new(
        prefix: &str,
        date: &Date,
        index: String,
        card_id: i32,
        first_shift: bool,
        last_shift: bool,
    ) -> Self {
        let day = Day::new(date);
        Self {
            add_route: format!("{prefix}/add/{card_id}/{}/{index}", day.day_as_number),
            remove_route: format!("{prefix}/remove/{card_id}/{}/{index}", day.day_as_number),
            add_button_id: format!("{prefix}ShiftAdd{}", day.day),
            remove_button_id: format!("{prefix}ShiftRemove{}", day.day),
            last_shift,
            form_id: FormIds::new(prefix, &day, index),
            tasks: Tasks::tasks_as_str(),
            entrance_hours: Hours::entrance_hours(),
            exit_hours: Hours::exit_hours(),
            date: if first_shift { Some(day) } else { None },
        }
    }

    // Creates first day shift
    pub(crate) fn first_shift(prefix: &str, date: &Date, index: String, card_id: i32) -> Self {
        Self::new(prefix, date, index, card_id, true, true)
    }
}

#[derive(FromForm)]
pub(crate) struct FormData<'r> {
    #[field(name = "checkboxes")]
    pub(crate) days: HashMap<usize, &'r str>,
    #[field(name = "tasks")]
    pub(crate) tasks: Vec<Vec<&'r str>>,
    #[field(name = "entranceHours")]
    pub(crate) entrance_hours: Vec<Vec<usize>>,
    #[field(name = "exitHours")]
    pub(crate) exit_hours: Vec<Vec<usize>>,
}

impl<'r> FormData<'r> {
    // Total number of days (selected and not selected ones)
    pub(crate) fn total_days(&self) -> usize {
        self.entrance_hours.len()
    }

    // Check whether any entrance hour is greater or equal than exit hour for selected days
    pub(crate) fn check_hours(&self) -> Option<String> {
        for day in self.days.keys() {
            let index = day - (ITALIAN_DAYS.len() - self.total_days());

            for (entrance_hour, exit_hour) in self.entrance_hours[index]
                .iter()
                .zip(self.exit_hours[index].iter())
            {
                match entrance_hour.cmp(exit_hour) {
                    Ordering::Greater => {
                        return Some(Hours::entrance_is_greater(*entrance_hour, *exit_hour))
                    }
                    Ordering::Equal => {
                        return Some(Hours::identical_hours(*entrance_hour, *exit_hour))
                    }
                    Ordering::Less => (),
                }
            }
        }
        None
    }

    // Create shifts for a week
    fn create_shifts(
        &self,
        shifts: &mut HashSet<Shift>,
        current_shifts: &HashSet<Shift>,
        card_id: i32,
    ) {
        self.days.iter().for_each(|(day, date)| {
            let day_index = day - (ITALIAN_DAYS.len() - self.total_days());
            for (idx, task) in self.tasks[day_index].iter().enumerate() {
                // Split hours in different shifts
                for (entrance_hour, exit_hour) in Hours::split(
                    self.entrance_hours[day_index][idx],
                    self.exit_hours[day_index][idx],
                ) {
                    let shift = Shift {
                        date: date.to_string(),
                        day: ITALIAN_DAYS[*day].to_string(),
                        task: task.to_string(),
                        entrance_hour,
                        exit_hour,
                        card_id,
                    };
                    if !current_shifts.contains(&shift) {
                        shifts.insert(shift);
                    }
                }
            }
        });
    }

    fn checkbox(prefix: &str, day_as_number: usize) -> String {
        format!("{prefix}.checkboxes[{day_as_number}]")
    }

    fn tasks(prefix: &str, day: &str, id: &str) -> String {
        format!("{prefix}.tasks[{day}][{id}]")
    }

    fn entrance_hour(prefix: &str, day: &str, id: &str) -> String {
        format!("{prefix}.entranceHours[{day}][{id}]")
    }

    fn exit_hour(prefix: &str, day: &str, id: &str) -> String {
        format!("{prefix}.exitHours[{day}][{id}]")
    }
}

#[derive(FromForm)]
pub(crate) struct ShiftsData<'r> {
    pub(crate) card_id: i32,
    #[field(name = "curr")]
    pub(crate) current: FormData<'r>,
    #[field(name = "next")]
    pub(crate) next: FormData<'r>,
}

impl<'r> ShiftsData<'r> {
    // Create all shifts to be inserted into the database
    pub(crate) fn create_shifts(&self, current_shifts: HashSet<Shift>) -> HashSet<Shift> {
        let mut shifts = HashSet::new();
        self.current
            .create_shifts(&mut shifts, &current_shifts, self.card_id);
        self.next
            .create_shifts(&mut shifts, &current_shifts, self.card_id);
        shifts
    }
}
