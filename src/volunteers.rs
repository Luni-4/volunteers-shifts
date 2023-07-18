use rocket::http::uri::Origin;
use rocket::http::CookieJar;
use rocket::response::Redirect;
use rocket::State;
use rocket_dyn_templates::{context, Template};

use crate::database::{delete_old_shifts, query_volunteers, refill_volunteers_table};
use crate::error::{query_error, InternalError};
use crate::login::ADMINISTRATION_COOKIE;
use crate::menu::MenuAdministration;
use crate::{
    AppState, APP_TITLE, DISABLED_MESSAGE, SHIFTS_MANAGER_ROUTE, SHIFTS_ROUTE, VOLUNTEERS_ROUTE,
};

const SHIFTS_MANAGER_MESSAGE: &str = "Inserisci turni";
const SHIFTS_MESSAGE: &str = "Cancella turni";
const UPDATE_VOLUNTEER_MESSAGE: &str = "Aggiorna volontari";

// Route to volunteers page
macro_rules! volunteers_uri {
    ($($t:tt)*) => (rocket::uri!(VOLUNTEERS_ROUTE, $($t)*))
}

#[put("/updatevolunteers")]
pub(crate) async fn update_volunteers(
    state: &State<AppState>,
    uri: &Origin<'_>,
) -> Result<Redirect, InternalError> {
    // Update volunteers retrieving their data from the csv file
    query_error(
        refill_volunteers_table(&state.pool, &state.volunteers_url),
        uri,
    )
    .await?;

    // Redirect to volunteers page
    Ok(Redirect::to(volunteers_uri!(show_volunteers)))
}

#[get("/")]
pub(crate) async fn show_volunteers(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    uri: &Origin<'_>,
) -> Result<Template, InternalError> {
    // Check if the administrator is authenticated
    if jar.get_private(ADMINISTRATION_COOKIE).is_none() {
        return Err(InternalError::not_authenticated_administrator(uri));
    }

    // Get all volunteers saved in database
    let volunteers = query_error(query_volunteers(&state.pool), uri).await?;

    // Remove all volunteers shifts in the database which are older than current date
    query_error(delete_old_shifts(&state.pool), uri).await?;

    Ok(Template::render(
        "volunteers",
        context! {
            title: APP_TITLE,
            menu_administration: MenuAdministration::render(),
            volunteers,
            disabled_message: DISABLED_MESSAGE,
            shifts_manager_route: SHIFTS_MANAGER_ROUTE,
            shifts_manager_message: SHIFTS_MANAGER_MESSAGE,
            shifts_route: SHIFTS_ROUTE,
            shifts_message: SHIFTS_MESSAGE,
            volunteer_route: uri!(update_volunteers),
            update_volunteer_message: UPDATE_VOLUNTEER_MESSAGE,
        },
    ))
}

pub(crate) fn routes() -> Vec<rocket::Route> {
    routes![show_volunteers]
}
