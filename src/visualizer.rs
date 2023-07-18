use rocket::http::uri::Origin;
use rocket::http::CookieJar;
use rocket::response::stream::{Event, EventStream};
use rocket::response::Redirect;
use rocket::tokio::select;
use rocket::tokio::sync::broadcast::error::RecvError;
use rocket::{Shutdown, State};
use rocket_dyn_templates::{context, Template};

use serde::Serialize;

use crate::cookie::CookieMessage;
use crate::data::{string_len_as_number, Hours, Tasks};
use crate::database::query_volunteers_shifts;
use crate::error::{query_error, InternalError};
use crate::time::Date;
use crate::{DatabaseState, APP_TITLE, VISUALIZE_SHIFTS_ROUTE};

// Macro that redirects to visualize shifts
macro_rules! visualize_shifts_uri {
    ($($t:tt)*) => (rocket::uri!(VISUALIZE_SHIFTS_ROUTE, $($t)*))
}

use visualize_shifts_uri as uri;

#[inline(always)]
fn add_cookie_redirect(key: &'static str, val: &str, jar: &CookieJar<'_>) -> Redirect {
    // Set the week to be shown
    jar.add((key, val.to_string()));
    // Redirect to shifts visualizer
    Redirect::to(uri!(visualize_shifts))
}

// Buttons data to change a week
#[derive(Serialize)]
struct Buttons {
    current_week_route: &'static str,
    is_curr: bool,
    current_week: &'static str,
    next_week_route: &'static str,
    is_next: bool,
    next_week: &'static str,
}

impl Buttons {
    fn render(is_curr: bool) -> Self {
        Self {
            current_week_route: "currentweek/change/0",
            is_curr,
            current_week: "Vedi settimana corrente",
            next_week_route: "currentweek/change/1",
            is_next: !is_curr,
            next_week: "Vedi settimana successiva",
        }
    }
}

// Change day date data
#[derive(Serialize)]
struct ChangeDate {
    change_day_route_left: String,
    change_day_route_right: String,
}

impl ChangeDate {
    fn render(is_curr: bool, jar: &CookieJar<'_>) -> (&'static str, String, Self) {
        // Retrieve current date
        let date = Date::current();
        // Retrieve day (Monday (0) - Saturday (5))
        let day = if let Some(day) = jar.get_pending("day") {
            day.value().len() - 1
        } else {
            date.day_as_number()
        };
        // Get date from day formatted as string
        let (day_str, date) = if is_curr {
            date.week_day(day)
        } else {
            date.next_week().week_day(day)
        };
        (
            day_str,
            date,
            Self {
                change_day_route_left: format!(
                    "/changeday/left/{}",
                    string_len_as_number(day.max(1))
                ),
                change_day_route_right: format!(
                    "changeday/right/{}",
                    string_len_as_number((day + 2).min(6))
                ),
            },
        )
    }
}

// Volunteers shifts
#[derive(Serialize)]
struct VolunteerShifts {
    tasks: &'static [&'static str],
    interval: String,
    volunteers_names: Vec<String>,
}

#[put("/changeday/left/<day>")]
pub(crate) async fn change_day_left(day: &str, jar: &CookieJar<'_>) -> Redirect {
    // Save the previous day and redirect to shifts visualizer
    add_cookie_redirect("day", day, jar)
}

#[put("/changeday/right/<day>")]
pub(crate) async fn change_day_right(day: &str, jar: &CookieJar<'_>) -> Redirect {
    // Save the next day and redirect to shifts visualizer
    add_cookie_redirect("day", day, jar)
}

#[put("/currentweek/change/<week>")]
pub(crate) async fn change_week(week: &str, jar: &CookieJar<'_>) -> Redirect {
    // Change day to Monday when next week is selected
    if week == "1" {
        // Set the day to be shown
        jar.add(("day", "0"));
    } else {
        jar.add((
            "day",
            string_len_as_number(Date::current().day_as_number() + 1),
        ));
    }
    // Set the week to be shown and redirect to shifts visualizer
    add_cookie_redirect("week", week, jar)
}

#[get("/", rank = 2)]
async fn visualize_shifts(
    state: &State<DatabaseState>,
    jar: &CookieJar<'_>,
    uri: &Origin<'_>,
) -> Result<Template, InternalError> {
    // Choose the week to be shown.
    // If the cookie file does not exist or the selected week is
    // the current one, show current week data, otherwise next week.
    let is_curr = jar
        .get_pending("week")
        .map_or(true, |week| week.value() == "0");

    // Buttons data
    let buttons_data = Buttons::render(is_curr);

    // Date and change day date
    let (day, date, change_day_date) = ChangeDate::render(is_curr, jar);

    // Hours intervals to save volunteers names
    let mut hours_intervals = Vec::new();

    // Iterate over hours intervals
    for (start_hour, end_hour) in Hours::intervals() {
        // Save names
        let mut names_surnames = Vec::new();
        // Iterate over tasks
        for task in Tasks::tasks_as_str() {
            let query_names_surnames = query_error(
                query_volunteers_shifts(&state.pool, &date, day, task, start_hour, end_hour),
                uri,
            )
            .await?;
            names_surnames.push(if query_names_surnames.is_empty() {
                "<br>".to_string()
            } else {
                query_names_surnames.join("<br>")
            });
        }
        hours_intervals.push(VolunteerShifts {
            tasks: Tasks::tasks_as_str(),
            interval: format!("{}<br>{}", start_hour, end_hour),
            volunteers_names: names_surnames,
        });
    }

    Ok(Template::render(
        "main_table",
        context! {
            title: APP_TITLE,
            buttons_data,
            change_day_date,
            day,
            date,
            hours_intervals,
            cookie: CookieMessage::render(jar, Some(VISUALIZE_SHIFTS_ROUTE.path().as_str())),
        },
    ))
}

#[get("/", format = "text/event-stream", rank = 1)]
async fn visualize_shifts_stream(
    state: &State<DatabaseState>,
    mut end: Shutdown,
) -> EventStream![] {
    let mut rx = state.sender.subscribe();
    EventStream! {
        loop {
            let _ = select! {
                msg = rx.recv() => match msg {
                    Ok(msg) => msg,
                    Err(RecvError::Closed) => break,
                    Err(RecvError::Lagged(_)) => continue,
                },
                _ = &mut end => break,
            };

            yield Event::data("");
        }
    }
}

pub(crate) fn routes() -> Vec<rocket::Route> {
    routes![visualize_shifts_stream, visualize_shifts]
}
