use std::borrow::Cow;

use rocket::form::Form;
use rocket::http::uri::Origin;
use rocket::http::{Cookie, CookieJar};
use rocket::request::FlashMessage;
use rocket::response::{Flash, Redirect};
use rocket::State;
use rocket_dyn_templates::{context, Template};

use serde::Serialize;

use crate::cookie::CookieMessage;
use crate::data::Button;
use crate::database::{
    query_card_id_to_surname, query_check_card_id, query_check_surname, query_is_disabled,
};
use crate::error::{query_error, InternalError};
use crate::{
    DatabaseState, APP_TITLE, AUTHENTICATION_COOKIE, CARD_COOKIE, SHIFTS_MANAGER_ROUTE,
    SURNAME_COOKIE,
};

macro_rules! shifts_manager_uri {
    ($($t:tt)*) => (rocket::uri!(SHIFTS_MANAGER_ROUTE, $crate::shifts_manager:: $($t)*))
}

// Input type number errors
#[derive(Serialize)]
struct InputTypeNumberErrors {
    // Missing value
    missing: &'static str,
    // Invalid value
    invalid: &'static str,
    // Underflow
    underflow: &'static str,
    // Overflow
    overflow: &'static str,
}

impl InputTypeNumberErrors {
    fn text() -> Self {
        Self {
            missing: "Inserire un numero di tessera.",
            invalid: "Inserire un numero di tessera come nell'esempio.",
            underflow: "Il numero di tessera deve essere positivo.",
            overflow: "Il numero di tessera deve essere inferiore o uguale a 100.",
        }
    }
}

// Authentication information
#[derive(Serialize)]
struct AuthenticationInfo {
    // Generic description
    description: &'static str,
    // Card identifier text
    card_id_text: &'static str,
    // Card identifier placeholder
    card_id_placeholder: &'static str,
    // Card identifier message when the field is empty
    card_id_empty_message: &'static str,
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
            card_id_text: "Numero tessera",
            card_id_placeholder: "es. 001",
            card_id_empty_message: "Inserire un numero di tessera.",
            card_id_value: Self::field_value(jar, CARD_COOKIE),
            card_id_error: InputTypeNumberErrors::text(),
            surname_text: "Cognome",
            surname_placeholder: "es. Rossi",
            surname_value: Self::field_value(jar, SURNAME_COOKIE),
            surname_empty_message: "Inserire un cognome.",
        }
    }

    #[inline(always)]
    fn field_value(jar: &CookieJar, key: &str) -> &'static str {
        jar.get_private(key)
            .as_ref()
            .and_then(Cookie::value_raw)
            .unwrap_or_default()
    }
}

// List of error messages
#[derive(Default, Serialize)]
struct ErrorMessage {
    card_id: Option<String>,
    surname: Option<String>,
}

impl ErrorMessage {
    fn card_id_text(&mut self, card_id: Option<&str>) {
        self.card_id =
            card_id.map(|card_id| format!("Il numero di tessera \"{card_id}\" non esiste"));
    }

    fn card_id_disabled_text(&mut self, card_id: Option<&str>) {
        self.card_id =
            card_id.map(|card_id| format!("Il numero di tessera \"{card_id}\" è disabilitato"));
    }

    fn surname_text(&mut self, surname: Option<&str>) {
        self.surname = surname.map(|surname| format!("Il cognome \"{surname}\" non esiste"));
    }

    fn card_id_wrong_surname_text(&mut self, card_id: Option<&str>, surname: Option<&str>) {
        self.surname = card_id.zip(surname).map(|(card_id, surname)| {
            format!("Il cognome \"{surname}\" non è associato al numero di tessera \"{card_id}\"")
        });
    }
}

#[inline(always)]
fn get_cookie<'a>(jar: &'a CookieJar<'a>, name: &'a str) -> Option<&'a str> {
    jar.get_private(name)
        .as_ref()
        .map(Cookie::value_raw)
        .unwrap_or_default()
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
            "card_id" => error_messages.card_id_text(get_cookie(jar, CARD_COOKIE)),
            "card_id-disabled" => {
                error_messages.card_id_disabled_text(get_cookie(jar, CARD_COOKIE))
            }
            "surname" => error_messages.surname_text(get_cookie(jar, SURNAME_COOKIE)),
            "card_id-surname" => {
                error_messages.card_id_text(get_cookie(jar, CARD_COOKIE));
                error_messages.surname_text(get_cookie(jar, SURNAME_COOKIE))
            }
            "card_id-wrong-surname" => error_messages.card_id_wrong_surname_text(
                get_cookie(jar, CARD_COOKIE),
                get_cookie(jar, SURNAME_COOKIE),
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
    card_id: i32,
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
    database_state: &State<DatabaseState>,
    jar: &CookieJar<'_>,
    uri: &Origin<'_>,
) -> Result<Flash<Redirect>, InternalError> {
    // Retrieve form data
    let authentication = authentication_form.into_inner();

    // Check whether the card identifier is present in the volunteers table
    let is_card_id = query_error(
        query_check_card_id(&database_state.pool, authentication.card_id),
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
    let volunteer_is_disabled = query_error(
        query_is_disabled(&database_state.pool, authentication.card_id),
        uri,
    )
    .await?;

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
    let is_surname = query_error(query_check_surname(&database_state.pool, &surname), uri).await?;

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
        query_card_id_to_surname(&database_state.pool, authentication.card_id, &surname),
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

    // If everything is correct, redirect to shift manager
    Ok(Flash::success(
        Redirect::to(shifts_manager_uri!(show_shift_manager(
            authentication.card_id
        ))),
        "Successful authentication.",
    ))
}
