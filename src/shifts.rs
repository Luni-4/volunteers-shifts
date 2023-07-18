use rocket::http::uri::Origin;
use rocket::http::CookieJar;
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};

use serde::Serialize;

use crate::data::{BookedShiftsData, ShiftsTableHeaders};
use crate::database::{
    delete_old_shifts, delete_shift, query_shifts, query_volunteer_info, ShiftQuery,
};
use crate::error::{query_error, InternalError};
use crate::{DatabaseState, ADMINISTRATION_COOKIE, APP_TITLE};

const DISABLED_MESSAGE: &str = "Il volontario è disabilitato!";

// Shifts table headers
#[derive(Serialize)]
struct ShiftsHeaders {
    id: &'static str,
    table_headers: ShiftsTableHeaders,
}

impl ShiftsHeaders {
    fn text() -> Self {
        Self {
            id: "Id Turno",
            table_headers: ShiftsTableHeaders::text(),
        }
    }
}

// Visible volunteer information and the relative shifts
#[derive(Serialize)]
struct VolunteerShifts {
    // Volunteer message
    volunteer_message: String,
    // Volunteer disabled
    volunteer_disabled: Option<&'static str>,
    // Shifts headers
    shifts_headers: Option<ShiftsHeaders>,
    // Shifts
    shifts: Option<Vec<BookedShiftsData>>,
}

impl VolunteerShifts {
    fn new(volunteer_message: String, shifts: Vec<ShiftQuery>) -> Self {
        Self {
            volunteer_message,
            volunteer_disabled: None,
            shifts_headers: Some(ShiftsHeaders::text()),
            shifts: Some(
                shifts
                    .into_iter()
                    .map(|shift| BookedShiftsData::new("/removeshift/".to_string(), shift))
                    .collect(),
            ),
        }
    }

    fn disabled(volunteer_message: String) -> Self {
        Self {
            volunteer_message,
            volunteer_disabled: Some(DISABLED_MESSAGE),
            shifts_headers: None,
            shifts: None,
        }
    }
}

#[delete("/removeshift/<id>")]
pub(crate) async fn remove_volunteer_shift(
    id: i32,
    database_state: &State<DatabaseState>,
    uri: &Origin<'_>,
) -> Result<Redirect, InternalError> {
    // Delete a shift using the id
    query_error(delete_shift(&database_state.pool, id), uri).await?;

    // Redirect to shifts manager visualizer
    Ok(Redirect::to(uri!(show_shifts)))
}

#[get("/")]
pub(crate) async fn show_shifts(
    database_state: &State<DatabaseState>,
    jar: &CookieJar<'_>,
    uri: &Origin<'_>,
) -> Result<Template, InternalError> {
    // Check if the administrator is authenticated
    if jar.get_private(ADMINISTRATION_COOKIE).is_none() {
        return Err(InternalError::not_authenticated_administrator(uri));
    }

    // Remove shifts older than current date
    query_error(delete_old_shifts(&database_state.pool), uri).await?;

    // Get volunteers card identifier, name, and surname
    let volunteers = query_error(query_volunteer_info(&database_state.pool), uri).await?;

    // Vector for volunteer shifts data
    let mut volunteers_shifts = Vec::new();

    // Get shifts for each volunteer
    for volunteer in volunteers.into_iter() {
        // Create the message
        let message = format!(
            "({}) {} {}",
            volunteer.card_id, volunteer.name, volunteer.surname
        );

        // Check if the volunteer is disabled
        volunteers_shifts.push(if !volunteer.disabled {
            // Get shifts for a volunteer
            let shifts =
                query_error(query_shifts(&database_state.pool, volunteer.card_id), uri).await?;

            VolunteerShifts::new(message, shifts)
        } else {
            VolunteerShifts::disabled(message)
        });
    }

    Ok(Template::render(
        "shifts_table",
        context! {
           title: APP_TITLE,
           volunteers_shifts,
        },
    ))
}

pub(crate) fn routes() -> Vec<rocket::Route> {
    routes![show_shifts]
}
