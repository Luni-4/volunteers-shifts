use rocket::http::uri::Origin;
use rocket::http::CookieJar;
use rocket::serde::json::Json;
use rocket::State;

use crate::database::{query_all_shifts, Shift};
use crate::error::{query_error, InternalError};

use crate::login::ADMINISTRATION_COOKIE;
use crate::AppState;

#[get("/")]
async fn download_database(
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    uri: &Origin<'_>,
) -> Result<Json<Vec<Shift>>, InternalError> {
    // Check if the administrator is authenticated
    if jar.get_private(ADMINISTRATION_COOKIE).is_none() {
        return Err(InternalError::not_authenticated_administrator(uri));
    }

    // Retrieve all shifts
    let all_shifts = query_error(query_all_shifts(&state.pool), uri).await?;

    Ok(Json(all_shifts))
}

pub(crate) fn routes() -> Vec<rocket::Route> {
    routes![download_database]
}
