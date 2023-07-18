use std::borrow::Cow;

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
use crate::database::{
    query_card_id_to_surname, query_check_card_id, query_check_surname, query_is_disabled,
};
use crate::error::{query_error, InternalError};
use crate::{AppState, APP_TITLE, SHIFTS_MANAGER_ROUTE};

use super::{ADMINISTRATION_COOKIE, AUTHENTICATION_COOKIE, CARD_COOKIE, SURNAME_COOKIE};

macro_rules! shifts_manager_uri {
    ($($t:tt)*) => (rocket::uri!(SHIFTS_MANAGER_ROUTE, $crate::shifts_manager:: $($t)*))
}

use super::{get_cookie_value, get_cookie_value_str};

// Authentication information
#[derive(Serialize)]
struct AuthenticationInfo {
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
    // Surname text
    surname_text: &'static str,
    // Surname placeholder
    surname_placeholder: &'static str,
    // Surname value
    surname_value: &'static str,
    // Surname message when the field is empty
    surname_empty_message: &'static str,
}

impl AuthenticationInfo {
    fn render(jar: &CookieJar) -> Self {
        Self {
            description: "Inserisci i tuoi dati",
            card_id_text: "Numero tessera (senza sigle iniziali)",
            card_id_placeholder: "es. 001",
            card_id_value: get_cookie_value_str(jar, CARD_COOKIE),
            card_id_error: InputTypeNumberErrors::text(),
            surname_text: "Cognome",
            surname_placeholder: "es. Rossi",
            surname_value: get_cookie_value_str(jar, SURNAME_COOKIE),
            surname_empty_message: "Inserire un cognome.",
        }
    }
}

// List of error messages
#[derive(Default, Serialize)]
struct ErrorMessage {
    card_id: Option<String>,
    surname: Option<String>,
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

    fn surname_text(&mut self, surname: Option<String>) {
        self.surname = surname.map(|surname| format!("Il cognome \"{surname}\" non esiste"));
    }

    fn card_id_wrong_surname_text(&mut self, card_id: Option<String>, surname: Option<String>) {
        self.surname = card_id.zip(surname).map(|(card_id, surname)| {
            format!("Il cognome \"{surname}\" non è associato al numero di tessera \"{card_id}\"")
        });
    }
}

#[get("/")]
pub(crate) async fn show_authentication(
    flash: Option<FlashMessage<'_>>,
    jar: &CookieJar<'_>,
) -> Template {
    // Get flash message
    let flash = flash.as_ref().map(FlashMessage::message);

    // Create an empty error (cleaning the error at each call)
    let mut error_messages = ErrorMessage::default();

    // Fill error message
    if let Some(error) = flash {
        match error {
            "card_id" => error_messages.card_id_text(get_cookie_value(jar, CARD_COOKIE)),
            "card_id-disabled" => {
                error_messages.card_id_disabled_text(get_cookie_value(jar, CARD_COOKIE))
            }
            "surname" => error_messages.surname_text(get_cookie_value(jar, SURNAME_COOKIE)),
            "card_id-surname" => {
                error_messages.card_id_text(get_cookie_value(jar, CARD_COOKIE));
                error_messages.surname_text(get_cookie_value(jar, SURNAME_COOKIE))
            }
            "card_id-wrong-surname" => error_messages.card_id_wrong_surname_text(
                get_cookie_value(jar, CARD_COOKIE),
                get_cookie_value(jar, SURNAME_COOKIE),
            ),
            _ => (),
        }
    }

    // Authentication text
    let auth_info = AuthenticationInfo::render(jar);

    // Authentication button
    let button = Button::authentication_button();

    Template::render(
        "authentication",
        context! {
            title: APP_TITLE,
            auth_info,
            error_messages,
            button,
            cookie: CookieMessage::render(jar, None),
        },
    )
}

#[derive(FromForm)]
pub(crate) struct Authentication<'r> {
    card_id: i16,
    surname: &'r str,
}

// Capitalize the first letter in a surname.
fn capitalize(s: &str) -> Cow<'_, str> {
    let mut c = s.chars();
    match c.next() {
        None => Cow::Borrowed(s),
        Some(f) => Cow::Owned(f.to_uppercase().collect::<String>() + c.as_str()),
    }
}

#[post("/", data = "<authentication_form>")]
pub(crate) async fn check_authentication<'r>(
    authentication_form: Form<Authentication<'r>>,
    state: &State<AppState>,
    jar: &CookieJar<'_>,
    uri: &Origin<'_>,
) -> Result<Flash<Redirect>, InternalError> {
    // Retrieve form data
    let authentication = authentication_form.into_inner();

    // Check whether the card identifier is present in the volunteers table
    let is_card_id = query_error(
        query_check_card_id(&state.pool, authentication.card_id),
        uri,
    )
    .await?;

    // Save card identification cookie
    jar.add_private((CARD_COOKIE, authentication.card_id.to_string()));

    // If there is no card identifier, redirects to authentication page
    if !is_card_id {
        return Ok(Flash::error(
            Redirect::to(uri!(show_authentication)),
            CARD_COOKIE,
        ));
    }

    // Check whether the volunteer is disabled
    let volunteer_is_disabled =
        query_error(query_is_disabled(&state.pool, authentication.card_id), uri).await?;

    // If volunteer is disabled, redirects to authentication page
    if volunteer_is_disabled {
        return Ok(Flash::error(
            Redirect::to(uri!(show_authentication)),
            "card_id_disabled",
        ));
    }

    // Capitalize surname
    let surname = capitalize(authentication.surname.trim());

    // Check whether the surname is present in the volunteers table
    let is_surname = query_error(query_check_surname(&state.pool, &surname), uri).await?;

    // Save surname cookie
    jar.add_private((SURNAME_COOKIE, authentication.surname.to_string()));

    // If the surname is wrong, redirect to authentication page
    if !is_surname {
        return Ok(Flash::error(
            Redirect::to(uri!(show_authentication)),
            SURNAME_COOKIE,
        ));
    }

    // If both card identifier and surname values are wrong, redirect to
    // authentication page
    if !is_card_id && !is_surname {
        // Redirect to authentication page
        return Ok(Flash::error(
            Redirect::to(uri!(show_authentication)),
            "card_id-surname",
        ));
    }

    // Check whether the surname is associated to the card identification
    let is_query_surname = query_error(
        query_card_id_to_surname(&state.pool, authentication.card_id, &surname),
        uri,
    )
    .await?;

    // If the surname is not associated to the right card identification,
    // redirects to authentication page
    if !is_query_surname {
        return Ok(Flash::error(
            Redirect::to(uri!(show_authentication)),
            "card_id-wrong-surname",
        ));
    }

    // Clean up card identification and surname cookies
    jar.remove_private(CARD_COOKIE);
    jar.remove_private(SURNAME_COOKIE);

    // Set cookie to certificate authentication
    jar.add_private((AUTHENTICATION_COOKIE, authentication.card_id.to_string()));

    // Remove administration cookie
    jar.remove_private(ADMINISTRATION_COOKIE);

    // If everything is correct, redirect to shift manager
    Ok(Flash::success(
        Redirect::to(shifts_manager_uri!(show_shifts_manager(
            authentication.card_id
        ))),
        "Successful authentication.",
    ))
}
