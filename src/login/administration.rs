use rocket::form::Form;
use rocket::http::uri::Origin;
use rocket::http::CookieJar;
use rocket::request::FlashMessage;
use rocket::response::{Flash, Redirect};
use rocket::State;
use rocket_dyn_templates::{context, Template};

use serde::Serialize;

use crate::cookie::CookieMessage;
use crate::data::{Button, InputTypeNumberErrors};
use crate::database::{query_check_card_id, query_is_disabled};
use crate::error::{query_error, InternalError};
use crate::{AppState, ADMINISTRATION_ROUTE, APP_TITLE, VOLUNTEERS_ROUTE};

use super::{get_cookie_value, get_cookie_value_str, ADMINISTRATION_COOKIE, AUTHENTICATION_COOKIE};

// Macro which redirects to volunteers route
macro_rules! volunteers_uri {
    ($($t:tt)*) => (rocket::uri!(VOLUNTEERS_ROUTE, $($t)*))
}

// Macro which redirects to administration routes
macro_rules! administration_uri {
    ($($t:tt)*) => (rocket::uri!(ADMINISTRATION_ROUTE, $($t)*))
}

use administration_uri as uri;

// Administration information
#[derive(Serialize)]
struct AdministrationInfo {
    // Generic description
    description: &'static str,
    // Card identifier text
    card_id_text: &'static str,
    // Card identifier placeholder
    card_id_placeholder: &'static str,
    // Card identifier value
    card_id_value: &'static str,
    // Card identifier error messages
    card_id_error: InputTypeNumberErrors,
    // Password text
    password_text: &'static str,
    // Erroneous password
    password_error_message: &'static str,
}

impl AdministrationInfo {
    fn render(jar: &CookieJar) -> Self {
        Self {
            description: "Autenticati come referente",
            card_id_text: "Numero tessera (senza sigle iniziali)",
            card_id_placeholder: "es. 001",
            card_id_value: get_cookie_value_str(jar, ADMINISTRATION_COOKIE),
            card_id_error: InputTypeNumberErrors::text(),
            password_text: "Password",
            password_error_message: "Inserire la password (almeno 8 caratteri)",
        }
    }
}

// List of error messages
#[derive(Default, Serialize)]
struct ErrorMessage {
    card_id: Option<String>,
    password: Option<&'static str>,
}

impl ErrorMessage {
    fn card_id_text(&mut self, card_id: Option<String>) {
        self.card_id =
            card_id.map(|card_id| format!("Il numero di tessera \"{card_id}\" non esiste"));
    }

    fn card_id_disabled_text(&mut self, card_id: Option<String>) {
        self.card_id =
            card_id.map(|card_id| format!("Il numero di tessera \"{card_id}\" è disabilitato"));
    }

    fn wrong_password_text(&mut self) {
        self.password = Some("La password inserita non è corretta");
    }
}

#[get("/")]
async fn show_administration(flash: Option<FlashMessage<'_>>, jar: &CookieJar<'_>) -> Template {
    // Get flash message
    let flash = flash.as_ref().map(FlashMessage::message);

    // Create an empty error (cleaning the error at each call)
    let mut error_messages = ErrorMessage::default();

    // Fill error message
    if let Some(error) = flash {
        match error {
            "card_id-non-existent" => {
                error_messages.card_id_text(get_cookie_value(jar, ADMINISTRATION_COOKIE))
            }
            "card_id-disabled" => {
                error_messages.card_id_disabled_text(get_cookie_value(jar, ADMINISTRATION_COOKIE))
            }
            "wrong-password" => error_messages.wrong_password_text(),
            _ => (),
        }
    }

    // Administration text
    let administration = AdministrationInfo::render(jar);

    // Administration button
    let button = Button::authentication_button();

    Template::render(
        "administration",
        context! {
            title: APP_TITLE,
            route: ADMINISTRATION_ROUTE,
            administration,
            error_messages,
            button,
            cookie: CookieMessage::render(jar, Some(ADMINISTRATION_ROUTE.path().as_str())),
        },
    )
}

#[derive(FromForm)]
struct Administration<'r> {
    card_id: i16,
    password: &'r str,
}

#[post("/", data = "<administration_form>")]
async fn check_administration<'r>(
    administration_form: Form<Administration<'r>>,
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    uri: &Origin<'_>,
) -> Result<Flash<Redirect>, InternalError> {
    // Retrieve form data
    let administration = administration_form.into_inner();

    // Check whether the card identifier is present in the volunteers table
    let is_card_id = query_error(
        query_check_card_id(&state.pool, administration.card_id),
        uri,
    )
    .await?;

    // Save card identification cookie
    jar.add_private((ADMINISTRATION_COOKIE, administration.card_id.to_string()));

    // If there is no card identifier, redirects to administration page
    if !is_card_id {
        return Ok(Flash::error(
            Redirect::to(uri!(show_administration)),
            "card_id-non-existent",
        ));
    }

    // Check whether the volunteer is disabled
    let volunteer_is_disabled =
        query_error(query_is_disabled(&state.pool, administration.card_id), uri).await?;

    // If volunteer is disabled, redirects to administration page
    if volunteer_is_disabled {
        return Ok(Flash::error(
            Redirect::to(uri!(show_administration)),
            "card_id-disabled",
        ));
    }

    // Compare passwords
    if administration.password == state.administration_password {
        // Remove authentication cookie
        jar.remove_private(AUTHENTICATION_COOKIE);
        // If everything is correct, redirect to administration page
        Ok(Flash::success(
            Redirect::to(volunteers_uri!(crate::volunteers::show_volunteers)),
            "Successful authentication.",
        ))
    } else {
        // If there is an error, redirect to administration authentication
        Ok(Flash::error(
            Redirect::to(uri!(show_administration)),
            "wrong-password",
        ))
    }
}

pub(crate) fn routes() -> Vec<rocket::Route> {
    routes![show_administration, check_administration]
}
