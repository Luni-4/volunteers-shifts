use std::borrow::Cow;

use rocket::http::uri::Origin;
use rocket::http::{Cookie, CookieJar};
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};

use serde::Serialize;

use crate::data::Tasks;
use crate::database::{
    delete_shift, format_db_date, query_is_disabled, query_shifts, query_volunteer_surname_name,
    ShiftQuery,
};
use crate::error::{query_error, InternalError};
use crate::login::{ADMINISTRATION_COOKIE, AUTHENTICATION_COOKIE};
use crate::menu::{Menu, MenuAdministration};
use crate::shifts_manager::shifts_manager_uri;
use crate::{AppState, APP_TITLE, DISABLED_MESSAGE, SHIFTS_MANAGER_ROUTE, SHIFTS_ROUTE};

// Messages
const HEADING_MESSAGE: &str = "Ecco i tuoi turni";
const EMPTY_SHIFTS_MESSAGE: &str = "Nessun turno inserito!";
const INSERT_SHIFTS_TEXT: &str = "Inserisci nuovi turni";
const DELETE_MESSAGE: &str = "Cancella";

// Route to shifts page
macro_rules! shifts_uri {
    ($($t:tt)*) => (rocket::uri!(SHIFTS_ROUTE, $($t)*))
}

pub(crate) use shifts_uri;

// Volunteer shift and the relative route to delete it
#[derive(Serialize)]
struct VolunteerShift {
    // Date
    date: String,
    // Task
    task: &'static str,
    // Hours
    hours: &'static str,
    // Route to delete the shift
    delete_route: Origin<'static>,
}

impl VolunteerShift {
    fn shifts(shifts: Vec<ShiftQuery>, card_id: i16) -> Vec<Self> {
        shifts
            .into_iter()
            .map(|shift| VolunteerShift {
                date: format_db_date(&shift.shift.date),
                task: Tasks::task_from_id(shift.shift.task as usize),
                hours: Tasks::hours_from_id(shift.shift.task as usize),
                delete_route: uri!(remove_shift(card_id, shift.id)),
            })
            .collect()
    }
}

#[delete("/removeshift/<card_id>/<shift_id>")]
pub(crate) async fn remove_shift(
    card_id: i16,
    shift_id: i32,
    state: &State<AppState>,
    uri: &Origin<'_>,
) -> Result<Redirect, InternalError> {
    // Delete a shift using its identifier
    query_error(delete_shift(&state.pool, shift_id), uri).await?;

    // Send an event to refresh inserted shifts
    let _res = state.sender.send(1);

    // Redirect to personal shifts page
    Ok(Redirect::to(shifts_uri!(show_shifts(card_id))))
}

#[inline(always)]
fn render_shifts_template(
    id: i16,
    is_administration: bool,
    heading_message: &str,
    no_shifts_message: &str,
) -> Template {
    Template::render(
        "shifts",
        context! {
           title: APP_TITLE,
           is_administration,
           menu: Menu::render(id),
           menu_administration: MenuAdministration::render(),
           heading_message,
           no_shifts_message: Some(no_shifts_message),
           insert_shifts_link: shifts_manager_uri!(crate::shifts_manager::show_shifts_manager(id)),
           insert_shifts_text: INSERT_SHIFTS_TEXT,
        },
    )
}

#[get("/?<id>")]
pub(crate) async fn show_shifts(
    id: i16,
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    uri: &Origin<'_>,
) -> Result<Template, InternalError> {
    // Check if the administrator is authenticated
    let (is_administration, heading_message) = if jar.get_private(ADMINISTRATION_COOKIE).is_none() {
        // Check if the user is authenticated
        match jar
            .get_private(AUTHENTICATION_COOKIE)
            .as_ref()
            .map(Cookie::value)
        {
            Some(cookie_card_id) if cookie_card_id != id.to_string() => {
                return Err(InternalError::wrong_card_id(uri))
            }
            Some(_) => (false, Cow::Borrowed(HEADING_MESSAGE)),
            None => return Err(InternalError::not_authenticated(uri)),
        }
    } else {
        let surname_name = query_error(query_volunteer_surname_name(&state.pool, id), uri).await?;
        (true, Cow::Owned(format!("({id}) {surname_name}")))
    };

    // Check whether the volunteer is disabled
    let volunteer_is_disabled = query_error(query_is_disabled(&state.pool, id), uri).await?;

    // If volunteer is disabled, show the message
    if volunteer_is_disabled {
        return Ok(render_shifts_template(
            id,
            is_administration,
            &heading_message,
            DISABLED_MESSAGE,
        ));
    }

    // Get shifts for a volunteer
    let shifts = query_error(query_shifts(&state.pool, id), uri).await?;

    // No shifts for the current volunteer
    if shifts.is_empty() {
        return Ok(render_shifts_template(
            id,
            is_administration,
            &heading_message,
            EMPTY_SHIFTS_MESSAGE,
        ));
    }

    let shifts = VolunteerShift::shifts(shifts, id);

    Ok(Template::render(
        "shifts",
        context! {
           title: APP_TITLE,
           is_administration,
           menu: Menu::render(id),
           menu_administration: MenuAdministration::render(),
           heading_message,
           shifts,
           delete_message: DELETE_MESSAGE,
           insert_shifts_link: shifts_manager_uri!(crate::shifts_manager::show_shifts_manager(id)),
           insert_shifts_text: INSERT_SHIFTS_TEXT,
        },
    ))
}

pub(crate) fn routes() -> Vec<rocket::Route> {
    routes![show_shifts]
}
