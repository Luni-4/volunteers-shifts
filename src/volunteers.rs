use rocket::http::uri::Origin;
use rocket::http::CookieJar;
use rocket::State;
use rocket_dyn_templates::{context, Template};

use serde::Serialize;

use crate::database::refill_volunteers_table;
use crate::error::{query_error, InternalError};
use crate::{DatabaseState, ADMINISTRATION_COOKIE, APP_TITLE};

// Volunteers table header
#[derive(Serialize)]
struct VolunteersHeader {
    // Card identifier
    card_id: &'static str,
    // Surname
    surname: &'static str,
    // Name
    name: &'static str,
    // Fiscal code
    fiscal_code: &'static str,
    // Disabled volunteer
    disabled: &'static str,
}

impl VolunteersHeader {
    fn text() -> Self {
        Self {
            card_id: "Numero tessera",
            surname: "Cognome",
            name: "Nome",
            fiscal_code: "Codice Fiscale",
            disabled: "Disabilitato",
        }
    }
}

#[get("/")]
pub(crate) async fn show_volunteers(
    database_state: &State<DatabaseState>,
    jar: &CookieJar<'_>,
    uri: &Origin<'_>,
) -> Result<Template, InternalError> {
    // Check if the administrator is authenticated
    if jar.get_private(ADMINISTRATION_COOKIE).is_none() {
        return Err(InternalError::not_authenticated_administrator(uri));
    }

    // Volunteers table header
    let volunteers_header = VolunteersHeader::text();

    // Update and show volunteers table retrieving its data from the file
    let volunteers = query_error(
        refill_volunteers_table(&database_state.pool, &database_state.volunteers_url),
        uri,
    )
    .await?;

    Ok(Template::render(
        "volunteers_table",
        context! {
            title: APP_TITLE,
            volunteers_header,
            volunteers,
        },
    ))
}

pub(crate) fn routes() -> Vec<rocket::Route> {
    routes![show_volunteers]
}
