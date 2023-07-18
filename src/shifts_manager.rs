use rocket::form::Form;
use rocket::http::uri::Origin;
use rocket::http::{Cookie, CookieJar};
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};

use serde::Serialize;

use crate::data::{Button, ShiftLabels, ShiftsData, Tasks, WeekData, FAKE_DAY_VALUE};
use crate::database::{
    fill_shifts_table, query_current_shifts, query_volunteer_name, query_volunteer_surname_name,
};
use crate::error::{query_error, InternalError};
use crate::login::{ADMINISTRATION_COOKIE, AUTHENTICATION_COOKIE};
use crate::menu::{Menu, MenuAdministration};
use crate::shifts::shifts_uri;
use crate::time::Date;
use crate::{AppState, APP_TITLE, SHIFTS_MANAGER_ROUTE, SHIFTS_ROUTE};

const HEADING_MESSAGE: &str = "Inserisci uno o più turni";

// Cookie keys
const SHIFT_NUMBERS: &str = "shift";
const DATA_KEY: &str = "data";

// Route to the shifts manager page
macro_rules! shifts_manager_uri {
    ($($t:tt)*) => (rocket::uri!(SHIFTS_MANAGER_ROUTE, $($t)*))
}

pub(crate) use shifts_manager_uri;

// Route
#[derive(Serialize)]
struct Routes {
    // Add button identifier
    add_button_id: String,
    // Add route
    add_route: Origin<'static>,
    // Check whether the shift is the last one
    is_last_shift: bool,
}

impl Routes {
    #[inline]
    fn render(card_id: i16, shift: u16, is_last_shift: bool) -> Self {
        Self {
            add_button_id: Self::add_button_id(shift),
            add_route: uri!(add_shift(card_id, shift)),
            is_last_shift,
        }
    }

    #[inline(always)]
    fn add_button_id(shift_number: u16) -> String {
        format!("Shift{shift_number}AddButton")
    }
}

// Shifts data
#[derive(Serialize)]
struct Shifts {
    // Routes for shift
    routes: Routes,
    // Shift title
    shift_title: String,
    // Shift identifier
    shift_id: String,
    // Fake value
    fake_value: u32,
    // Dates explanation
    explain_dates: &'static str,
    // First week date
    first_week_date: WeekData,
    // Second week date
    second_week_date: WeekData,
    // Tasks explanation
    explain_tasks: &'static str,
    // Tasks
    tasks: &'static [Tasks],
    // Button text
    button: Button,
}

impl Shifts {
    fn render(card_id: i16, jar: &CookieJar<'_>) -> Vec<Self> {
        let current_date = Date::current();
        if let Some(shifts_number) = jar
            .get_private(SHIFT_NUMBERS)
            .as_ref()
            .map(Cookie::value)
            .and_then(|value| value.parse::<u16>().ok())
        {
            (1..shifts_number + 1)
                .map(|shift| {
                    Self::fill_shift(card_id, shift, shift == shifts_number, &current_date)
                })
                .collect()
        } else {
            vec![Self::fill_shift(card_id, 1, true, &current_date)]
        }
    }

    #[inline]
    fn fill_shift(card_id: i16, shift: u16, is_last_shift: bool, date: &Date) -> Self {
        Self {
            routes: Routes::render(card_id, shift, is_last_shift),
            shift_title: Self::shift_title(shift),
            shift_id: Self::shift_id(shift),
            fake_value: FAKE_DAY_VALUE,
            explain_dates: "Inserisci data...",
            first_week_date: WeekData::week_info(date),
            second_week_date: WeekData::week_info(&date.next_week().monday()),
            explain_tasks: "Inserisci mansione...",
            tasks: Tasks::render(),
            button: Button::add_shift(),
        }
    }

    #[inline(always)]
    fn shift_title(shift_number: u16) -> String {
        format!("Turno {shift_number}")
    }

    #[inline(always)]
    fn shift_id(shift_number: u16) -> String {
        format!("shift{shift_number}")
    }

    #[inline(always)]
    fn reset_cookie(jar: &CookieJar<'_>) {
        // Reset shift cookies
        jar.add_private((SHIFT_NUMBERS, "1"));
    }
}

#[put("/add/<card_id>/<shift_id>")]
pub(crate) async fn add_shift(card_id: i16, shift_id: u16, jar: &CookieJar<'_>) -> Redirect {
    // Increment current shift number and save it as cookie
    jar.add_private((SHIFT_NUMBERS, (shift_id + 1).to_string()));
    // Redirect to shifts manager page
    Redirect::to(shifts_manager_uri!(show_shifts_manager(card_id)))
}

#[get("/?<id>")]
pub(crate) async fn show_shifts_manager(
    id: i16,
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    uri: &Origin<'_>,
) -> Result<Template, InternalError> {
    // Check if the administrator is authenticated
    let is_administration = if jar.get_private(ADMINISTRATION_COOKIE).is_none() {
        // Check if the user is authenticated
        match jar
            .get_private(AUTHENTICATION_COOKIE)
            .as_ref()
            .map(Cookie::value)
        {
            Some(cookie_card_id) if cookie_card_id != id.to_string() => {
                return Err(InternalError::wrong_card_id(uri))
            }
            Some(_) => false,
            None => return Err(InternalError::not_authenticated(uri)),
        }
    } else {
        true
    };

    // Clear sessionStorage
    let clear_session_storage = jar
        .get(DATA_KEY)
        .map(Cookie::value)
        .map_or(false, |value| value == "1");

    // Set cookie to allow new data insertion
    jar.add((DATA_KEY, "0"));

    // Shift Labels
    let shift_labels = ShiftLabels::render();

    // Create shifts
    let shifts = Shifts::render(id, jar);

    // Button text
    let button = Button::shifts_buttons();

    Ok(Template::render(
        "shifts_manager",
        context! {
            title: APP_TITLE,
            is_administration,
            menu: Menu::render(id),
            menu_administration: MenuAdministration::render(),
            route: SHIFTS_MANAGER_ROUTE,
            heading_message: if is_administration {
                let surname_name = query_error(query_volunteer_surname_name(&state.pool, id), uri).await?;
                format!("({id}) {surname_name}")
            } else {
                let volunteer_name = query_error(query_volunteer_name(&state.pool, id), uri).await?;
                format!("Ciao {}!", volunteer_name)
            },
            guide_message: HEADING_MESSAGE,
            shift_labels,
            shifts,
            id,
            button,
            clear_session_storage,
        },
    ))
}

#[post("/", data = "<shifts_form>")]
async fn check_shifts_data(
    shifts_form: Form<ShiftsData>,
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    uri: &Origin<'_>,
) -> Result<Redirect, InternalError> {
    // Retrieve form data
    let data = shifts_form.into_inner();

    // If the same data is inserted again without refreshing the page,
    // redirect to success page
    if jar
        .get(DATA_KEY)
        .map(Cookie::value)
        .map_or(false, |data| data == "1")
    {
        return Ok(Redirect::to(shifts_uri!(crate::shifts::show_shifts(
            data.card_id
        ))));
    }

    // Retrieve every volunteer shifts to avoid producing duplicates
    let all_shifts = query_error(query_current_shifts(&state.pool, data.card_id), uri).await?;

    // Create new shifts
    let shifts = data.create_shifts(all_shifts);

    // Insert all shifts
    query_error(fill_shifts_table(&state.pool, shifts), uri).await?;

    // Set cookie to notify that data has been inserted correctly
    jar.add((DATA_KEY, "1"));

    // Send an event to refresh inserted shifts
    let _res = state.sender.send(1);

    // Reset cookies
    Shifts::reset_cookie(jar);

    // If everything is correct, redirect to personal shifts page
    Ok(Redirect::to(shifts_uri!(crate::shifts::show_shifts(
        data.card_id
    ))))
}

pub(crate) fn routes() -> Vec<rocket::Route> {
    routes![show_shifts_manager, check_shifts_data]
}
