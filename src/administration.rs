use rocket::form::Form;
use rocket::http::uri::Origin;
use rocket::http::CookieJar;
use rocket::request::FlashMessage;
use rocket::response::{Flash, Redirect};
use rocket::State;
use rocket_dyn_templates::{context, Template};

use serde::Serialize;

use crate::cookie::CookieMessage;
use crate::data::Button;
use crate::error::InternalError;
use crate::{
    DatabaseState, ADMINISTRATION_COOKIE, ADMINISTRATION_ROUTE, APP_TITLE, SHIFTS_ROUTE,
    VOLUNTEER_ROUTE,
};

const PASSWORD_ERROR: &str = "La password inserita non è corretta";

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
    // Password text
    password_text: &'static str,
    // Password message when the field is empty
    password_empty_message: &'static str,
    // Too short password
    password_short_message: &'static str,
}

impl AdministrationInfo {
    fn text() -> Self {
        Self {
            description: "Autenticati come amministratore",
            password_text: "Password",
            password_empty_message: "Inserire la password",
            password_short_message: "La password è troppo corta (almeno 8 caratteri)",
        }
    }
}

#[get("/")]
pub(crate) async fn show_administration(
    flash: Option<FlashMessage<'_>>,
    jar: &CookieJar<'_>,
) -> Template {
    // Get flash parameters
    let error_message = flash
        .as_ref()
        .map(FlashMessage::message)
        .map(|_| PASSWORD_ERROR);

    // Administration text
    let administration = AdministrationInfo::text();

    // Administration button
    let button = Button::authentication_button();

    Template::render(
        "administration",
        context! {
            title: APP_TITLE,
            route: ADMINISTRATION_ROUTE,
            administration,
            error_message,
            button,
            cookie: CookieMessage::render(jar, Some(ADMINISTRATION_ROUTE.path().as_str())),
        },
    )
}

#[get("/menu")]
pub(crate) async fn show_administration_page(
    jar: &CookieJar<'_>,
    uri: &Origin<'_>,
) -> Result<Template, InternalError> {
    // Check if the administrator is authenticated
    if jar.get_private(ADMINISTRATION_COOKIE).is_none() {
        return Err(InternalError::not_authenticated_administrator(uri));
    }

    Ok(Template::render(
        "administration_page",
        context! {
            title: APP_TITLE,
            volunteer_route: VOLUNTEER_ROUTE,
            volunteer_message: "Volontari",
            shifts_route: SHIFTS_ROUTE,
            shifts_message: "Turni",
        },
    ))
}

#[derive(FromForm)]
pub(crate) struct Administration<'r> {
    password: &'r str,
}

#[post("/", data = "<administration_form>")]
pub(crate) async fn check_administration<'r>(
    administration_form: Form<Administration<'r>>,
    database_state: &State<DatabaseState>,
    jar: &CookieJar<'_>,
) -> Flash<Redirect> {
    // Retrieve form data
    let administration = administration_form.into_inner();

    // Compare passwords
    if administration.password == database_state.administration_password {
        // Save administration cookie
        jar.add_private((ADMINISTRATION_COOKIE, "1"));

        // If everything is correct, redirect to administration page
        Flash::success(
            Redirect::to(uri!(show_administration_page)),
            "Successful authentication.",
        )
    } else {
        // If there is an error, redirect to administration authentication
        Flash::error(
            Redirect::to(uri!(show_administration)),
            "Password not correct",
        )
    }
}

pub(crate) fn routes() -> Vec<rocket::Route> {
    routes![
        show_administration,
        check_administration,
        show_administration_page
    ]
}
