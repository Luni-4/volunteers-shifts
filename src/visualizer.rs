use rocket::form::Form;
use rocket::http::uri::Origin;
use rocket::http::{Cookie, CookieJar};
use rocket::response::stream::{Event, EventStream};
use rocket::response::Redirect;
use rocket::tokio::select;
use rocket::tokio::sync::broadcast::error::RecvError;
use rocket::{Shutdown, State};
use rocket_dyn_templates::{context, Template};

use serde::Serialize;

use crate::data::{Day, SelectDay, Tasks, WeekData};
use crate::database::query_volunteers_shifts;
use crate::error::{query_error, InternalError};
use crate::login::{ADMINISTRATION_COOKIE, AUTHENTICATION_COOKIE};
use crate::menu::{Menu, MenuAdministration};
use crate::time::Date;
use crate::{AppState, APP_TITLE, VISUALIZE_SHIFTS_ROUTE};

// Cookies key
const WEEK: &str = "week";
const DAY: &str = "day";

// Macro that redirects to visualize shifts
macro_rules! visualize_shifts_uri {
    ($($t:tt)*) => (rocket::uri!(VISUALIZE_SHIFTS_ROUTE, $($t)*))
}

use visualize_shifts_uri as uri;

#[derive(FromForm)]
struct ParamsForm {
    #[field(name = "week")]
    week: String,
    #[field(name = "day")]
    day: String,
}

#[put("/", data = "<params_form>", rank = 2)]
async fn process_visualizer_params(params_form: Form<ParamsForm>, jar: &CookieJar<'_>) -> Redirect {
    // Retrieve form data
    let data = params_form.into_inner();

    // Add data to cookies
    jar.add((WEEK, data.week));
    jar.add((DAY, data.day));

    // Redirect to visualize shifts
    Redirect::to(uri!(visualize_shifts))
}

// Week information
#[derive(Serialize)]
struct Week {
    // Week value
    week_value: bool,
    // Week text
    week_text: String,
}

impl Week {
    fn new(week_value: bool, date: &Date) -> Self {
        Self {
            week_value,
            week_text: WeekData::week_bounds(date),
        }
    }
}

// Form information
#[derive(Serialize)]
struct FormInfo {
    // Route
    route: Origin<'static>,
    // Week text
    week_text: &'static str,
    // Weeks
    weeks: [Week; 2],
    // Days text
    days_text: &'static str,
    // First week days
    first_week_days: Option<Vec<SelectDay>>,
    // Second week days
    second_week_days: Option<Vec<Day>>,
}

impl FormInfo {
    fn render(date: Date, is_first_week: bool, is_first_time: bool) -> Self {
        let first_week = date.monday();
        let second_week = first_week.next_week();
        Self {
            route: uri!(process_visualizer_params),
            week_text: "Scegli settimana",
            weeks: [Week::new(true, &first_week), Week::new(false, &second_week)],
            days_text: "Scegli giorno",
            first_week_days: if is_first_time && is_first_week {
                let day_as_number = date.day_as_number();
                Some(SelectDay::selected_days(&first_week, day_as_number))
            } else if is_first_week {
                Some(SelectDay::days(&first_week))
            } else {
                None
            },
            second_week_days: (!is_first_week).then(|| WeekData::days(&second_week)),
        }
    }
}

// Visualizer information
#[derive(Serialize)]
struct VisualizerInfo {
    // Task
    task_name: &'static str,
    // Hours
    task_hours: &'static str,
    // Volunteers names
    volunteers_names: Vec<String>,
}

#[inline(always)]
fn get_week(week: Option<&str>) -> bool {
    match week {
        Some("false") => false,
        Some("true") | Some(_) | None => true,
    }
}

#[inline(always)]
fn get_day(day: Option<&str>, date: &Date) -> u32 {
    match day {
        Some("0") => 0,
        Some("1") => 1,
        Some("2") => 2,
        Some("3") => 3,
        Some("4") => 4,
        Some("5") => 5,
        Some(_) | None => date.day_as_number(),
    }
}

#[inline(always)]
fn get_card_id(jar: &CookieJar, name: &str) -> Option<i16> {
    jar.get_private(name)
        .as_ref()
        .map(Cookie::value)
        .and_then(|val| val.parse::<i16>().ok())
}

#[get("/", rank = 2)]
pub(crate) async fn visualize_shifts(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    uri: &Origin<'_>,
) -> Result<Template, InternalError> {
    // Check if the administrator is authenticated
    let (is_administration, card_id) =
        if let Some(card_id) = get_card_id(jar, ADMINISTRATION_COOKIE) {
            (true, card_id)
        } else {
            // Check if the user is authenticated
            if let Some(card_id) = get_card_id(jar, AUTHENTICATION_COOKIE) {
                (false, card_id)
            } else {
                return Err(InternalError::not_authenticated(uri));
            }
        };

    // Week cookie
    let week = jar.get(WEEK).map(Cookie::value);

    // Save it is the first time visiting the page through cookie analysis
    let is_first_time = week.is_none();

    // Get week value
    let week = get_week(week);

    // Get current date
    let current_date = Date::current();

    // Get day value
    let day = get_day(jar.get(DAY).map(Cookie::value), &current_date);

    // Retrieve day and date as text
    let date = if week {
        current_date.day_date(day)
    } else {
        current_date.next_week().day_date(day)
    };

    // Retrieve form information
    let form_info = FormInfo::render(current_date, week, is_first_time);

    // Visualizer information
    let mut visualize_info = Vec::new();

    for task in Tasks::render() {
        let volunteers_names = query_error(
            query_volunteers_shifts(&state.pool, date, task.task_value as i16),
            uri,
        )
        .await?;

        visualize_info.push(VisualizerInfo {
            task_name: task.task_name,
            task_hours: task.task_hours,
            volunteers_names,
        });
    }

    Ok(Template::render(
        "visualizer",
        context! {
            title: APP_TITLE,
            is_administration,
            menu: Menu::render(card_id),
            menu_administration: MenuAdministration::render(),
            form_info,
            visualize_info,
        },
    ))
}

#[get("/", format = "text/event-stream", rank = 1)]
async fn visualize_shifts_stream(state: &State<AppState>, mut end: Shutdown) -> EventStream![] {
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
    routes![
        visualize_shifts_stream,
        visualize_shifts,
        process_visualizer_params
    ]
}
