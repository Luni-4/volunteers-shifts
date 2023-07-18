use std::cmp::Ordering;

use rocket::form::Form;
use rocket::http::uri::Origin;
use rocket::http::{Cookie, CookieJar};
use rocket::request::FlashMessage;
use rocket::response::{Flash, Redirect};
use rocket::State;
use rocket_dyn_templates::{context, Template};

use serde::Serialize;

use crate::data::{
    string_len_as_number, BookedShiftsData, Button, DayShift, ShiftsData, ShiftsTableHeaders,
};
use crate::database::{
    delete_shift, fill_shifts_table, query_current_shifts, query_shifts, query_volunteer_name,
    ShiftQuery,
};
use crate::error::{query_error, InternalError};
use crate::time::{Date, ITALIAN_DAYS};
use crate::{DatabaseState, APP_TITLE, AUTHENTICATION_COOKIE, SHIFTS_MANAGER_ROUTE, SUCCESS_ROUTE};

const HEADING_MESSAGE: &str = "Ciao";
const GREETING_MESSAGE: &str = "Ecco i tuoi turni";
const NO_SHIFT_MESSAGE: &str = "Nessun turno selezionato!";
const REMOVE_SHIFT_ERROR: &str = "Il file di cookie per rimuovere i turni aggiuntivi non esiste";

// Week data
const CURRENT_WEEK_PREFIX: &str = "curr";
const NEXT_WEEK_PREFIX: &str = "next";
const WEEK_MESSAGE: &str = "Settimana";

// Cookie key
const DATA_KEY: &str = "data";

// Macro that redirects to success page
macro_rules! success_uri {
    ($($t:tt)*) => (rocket::uri!(SUCCESS_ROUTE, $crate::success:: $($t)*))
}

macro_rules! shifts_manager_uri {
    ($($t:tt)*) => (rocket::uri!(SHIFTS_MANAGER_ROUTE, $($t)*))
}

use shifts_manager_uri as uri;

// Booked shifts information
#[derive(Serialize)]
struct BookedShifts<'a> {
    // Tells whether a volunteer has already booked some shifts
    any_shift: bool,
    // Greeting message
    greeting_message: &'static str,
    // Table headers for shifts already booked by a volunteer
    table_headers: &'a ShiftsTableHeaders,
    // Every shift associated to a volunteer
    shifts: Vec<BookedShiftsData>,
    // Message when there are no shifts
    no_shift_message: &'static str,
}

impl<'a> BookedShifts<'a> {
    pub fn render(
        any_shift: bool,
        table_headers: &'a ShiftsTableHeaders,
        shifts: Vec<ShiftQuery>,
        card_id: i32,
    ) -> Self {
        Self {
            any_shift,
            greeting_message: GREETING_MESSAGE,
            table_headers,
            shifts: shifts
                .into_iter()
                .map(|shift| BookedShiftsData::new(format!("/deleteshift/{card_id}"), shift))
                .collect(),
            no_shift_message: NO_SHIFT_MESSAGE,
        }
    }
}

#[delete("/deleteshift/<card_num>/<id>")]
pub(crate) async fn delete_volunteer_shift(
    card_num: i32,
    id: i32,
    database_state: &State<DatabaseState>,
    uri: &Origin<'_>,
) -> Result<Redirect, InternalError> {
    // Delete a shift using the id
    query_error(delete_shift(&database_state.pool, id), uri).await?;

    // Redirect to shifts manager visualizer
    Ok(Redirect::to(uri!(show_shift_manager(card_num))))
}

// Week shifts
#[derive(Serialize)]
struct Week<'a> {
    // Week message
    message: String,
    // Error inserting week data
    error: Option<String>,
    // Table headers for week shifts
    table_headers: &'a ShiftsTableHeaders,
    // All week shifts
    shifts: Vec<DayShift>,
}

impl<'a> Week<'a> {
    fn render(
        prefix: &str,
        date: Date,
        table_headers: &'a ShiftsTableHeaders,
        card_id: i32,
        error: Option<String>,
        jar: &CookieJar<'_>,
    ) -> Self {
        // Render all shifts for week
        let shifts = Self::shifts(prefix, card_id, &date, jar);

        Self {
            message: format!("{} {}", WEEK_MESSAGE, date.week_bounds()),
            error,
            table_headers,
            shifts,
        }
    }

    fn shifts(prefix: &str, card_id: i32, date: &Date, jar: &CookieJar<'_>) -> Vec<DayShift> {
        let mut day_shifts = Vec::new();

        for date in date.now_until_saturday() {
            let day_as_number = date.day_as_number();
            if let Some(index) = jar.get(&format!("{}{}", prefix, day_as_number)) {
                let index_len = index.value().len();
                for shift in 0..index_len {
                    day_shifts.push(DayShift::new(
                        prefix,
                        &date,
                        string_len_as_number(shift + 1),
                        card_id,
                        shift == 0,
                        shift + 1 == index_len,
                    ));
                }
            } else {
                day_shifts.push(DayShift::first_shift(
                    prefix,
                    &date,
                    "0".to_string(),
                    card_id,
                ));
            }
        }
        day_shifts
    }

    fn reset_cookies(prefix: &str, jar: &CookieJar<'_>, cookie_len: usize) {
        // Reset cookies for weeks
        (0..cookie_len).for_each(|index| {
            let day = index + (ITALIAN_DAYS.len() - cookie_len);
            let curr_day = format!("{}{day}", prefix);
            if jar.get(&curr_day).is_some() {
                jar.add((curr_day, "0"));
            }
        });
    }
}

#[put("/<prefix>/add/<card_num>/<day>/<index>")]
pub(crate) async fn add_shifts_current_week(
    prefix: String,
    card_num: i32,
    day: &str,
    index: &str,
    jar: &CookieJar<'_>,
) -> Redirect {
    // Save the day with the current week with more shifts as key and the number
    // of shifts (expressed as a sequence of zeros) as value
    jar.add((format!("{prefix}{day}"), format!("{index}0")));
    // Redirect to shifts manager visualizer
    Redirect::to(uri!(show_shift_manager(card_num)))
}

#[delete("/<prefix>/remove/<card_num>/<day>/<index>")]
pub(crate) async fn remove_shifts_current_week(
    prefix: String,
    card_num: i32,
    day: &str,
    index: &str,
    jar: &CookieJar<'_>,
    uri: &Origin<'_>,
) -> Result<Redirect, InternalError> {
    let cookie_key = format!("{prefix}{day}");
    if let Some(cookie_index) = jar.get(&cookie_key).map(Cookie::value) {
        // Get the length of the current index
        let index_len = index.len();
        // Get the length of the saved cookie index
        let cookie_index_len = cookie_index.len();
        // If a removed index is less than cookie index, it means there are
        // more shifts
        match index_len.cmp(&cookie_index_len) {
            Ordering::Less => {
                // Compute the difference among the indexes
                let difference = cookie_index_len - index_len;
                // Get first part of the new index
                let first_part = string_len_as_number(index_len - 1);
                // Get second part of the new index
                let second_part = string_len_as_number(difference);
                // Save the new cookie
                jar.add((cookie_key, format!("{first_part}{second_part}")));
            }
            Ordering::Equal => {
                // If the two indexes are equals, remove the last one
                jar.add((
                    cookie_key,
                    string_len_as_number((index_len - 1).max(1)).to_string(),
                ));
            }
            Ordering::Greater => {
                // When the index is greater than the cookie index (because
                // cookie indexes have been reset for example) do nothing and
                // redirect to the shift manager page
                return Ok(Redirect::to(uri!(show_shift_manager(card_num))));
            }
        }
        Ok(Redirect::to(uri!(show_shift_manager(card_num))))
    } else {
        // Error if a cookie file is not found
        Err(InternalError::text(uri, REMOVE_SHIFT_ERROR))
    }
}

#[get("/?<id>")]
pub(crate) async fn show_shift_manager(
    id: i32,
    flash: Option<FlashMessage<'_>>,
    database_state: &State<DatabaseState>,
    jar: &CookieJar<'_>,
    uri: &Origin<'_>,
) -> Result<Template, InternalError> {
    // Check if the user is authenticated
    match jar
        .get_private(AUTHENTICATION_COOKIE)
        .as_ref()
        .map(Cookie::value)
    {
        Some(card_id) if card_id != id.to_string() => {
            return Err(InternalError::wrong_card_id(uri))
        }
        Some(_) => (),
        None => return Err(InternalError::not_authenticated(uri)),
    }

    // Get error message
    let (current_error, next_error) =
        if let Some((kind, error)) = flash.map(FlashMessage::into_inner) {
            match kind.as_str() {
                CURRENT_WEEK_PREFIX => (Some(error), None),
                NEXT_WEEK_PREFIX => (None, Some(error)),
                _ => (None, None),
            }
        } else {
            (None, None)
        };

    // Clear sessionStorage
    let clear_session_storage = jar
        .get(DATA_KEY)
        .map(Cookie::value)
        .map_or(false, |value| value == "1");

    // Set cookie to allow new data insertion
    jar.add((DATA_KEY, "0"));

    // Get name associated to volunteer
    let volunteer_name = query_error(query_volunteer_name(&database_state.pool, id), uri).await?;

    // Get message
    let heading_message = format!("{HEADING_MESSAGE} {}!", volunteer_name);

    // Get shifts data for card identification
    let shifts = query_error(query_shifts(&database_state.pool, id), uri).await?;

    // Table headers
    let table_headers = ShiftsTableHeaders::text();

    // Get all shifts already booked by a volunteer
    let booked_shifts = BookedShifts::render(!shifts.is_empty(), &table_headers, shifts, id);

    // Create current week
    let current_week = Week::render(
        CURRENT_WEEK_PREFIX,
        Date::current(),
        &table_headers,
        id,
        current_error,
        jar,
    );

    // Create next week
    let next_week = Week::render(
        NEXT_WEEK_PREFIX,
        Date::current().next_week().monday(),
        &table_headers,
        id,
        next_error,
        jar,
    );

    // Button text
    let button = Button::shifts_buttons();

    Ok(Template::render(
        "insert_shifts",
        context! {
            title: APP_TITLE,
            route: SHIFTS_MANAGER_ROUTE,
            heading_message,
            booked_shifts,
            current_week,
            next_week,
            id,
            button,
            clear_session_storage,
        },
    ))
}

#[post("/", data = "<shifts_data_form>")]
pub(crate) async fn check_shifts_data<'r>(
    shifts_data_form: Form<ShiftsData<'r>>,
    state: &State<DatabaseState>,
    jar: &CookieJar<'_>,
    uri: &Origin<'_>,
) -> Result<Flash<Redirect>, InternalError> {
    // If the same data is inserted again without refreshing the page,
    // redirect to success page
    if jar
        .get(DATA_KEY)
        .map(Cookie::value)
        .map_or(false, |data| data == "1")
    {
        return Ok(Flash::success(
            Redirect::temporary(success_uri!(show_success)),
            "Successful shifts insertion.",
        ));
    }

    // Retrieve form data
    let data = shifts_data_form.into_inner();

    // Returns an error if no days for the current week have been selected
    if data.current.days.is_empty() && data.next.days.is_empty() {
        return Ok(Flash::new(
            Redirect::to(uri!(show_shift_manager(data.card_id))),
            CURRENT_WEEK_PREFIX,
            "Bisogna selezionare almeno un giorno.",
        ));
    }

    // Check if there are wrong exit hours for the current week
    if let Some(error_message) = data.current.check_hours() {
        return Ok(Flash::new(
            Redirect::to(uri!(show_shift_manager(data.card_id))),
            CURRENT_WEEK_PREFIX,
            error_message,
        ));
    }

    // Check if there are wrong exit hours for the next week
    if let Some(error_message) = data.next.check_hours() {
        return Ok(Flash::new(
            Redirect::to(uri!(show_shift_manager(data.card_id))),
            NEXT_WEEK_PREFIX,
            error_message,
        ));
    }

    // Retrieve every volunteer shifts to avoid producing duplicates
    let all_shifts = query_error(query_current_shifts(&state.pool, data.card_id), uri).await?;

    // Create new shifts
    let shifts = data.create_shifts(all_shifts);

    // Insert all shifts
    query_error(fill_shifts_table(&state.pool, shifts), uri).await?;

    // Reset cookies to the initial state for the weeks
    Week::reset_cookies(CURRENT_WEEK_PREFIX, jar, data.current.total_days());
    Week::reset_cookies(NEXT_WEEK_PREFIX, jar, data.next.total_days());

    // Set cookie to notify that data has been inserted correctly
    jar.add((DATA_KEY, "1"));

    // Send an event to refresh inserted shifts
    let _res = state.sender.send(1);

    // If everything is correct, redirect to success page
    Ok(Flash::success(
        Redirect::temporary(success_uri!(show_success)),
        "Successful shifts insertion.",
    ))
}

pub(crate) fn routes() -> Vec<rocket::Route> {
    routes![show_shift_manager, check_shifts_data]
}
