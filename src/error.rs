use rocket::http::uri::Origin;
use rocket::http::Status;
use rocket::Request;

use rocket_dyn_templates::{context, Template};

use crate::{ADMINISTRATION_ROUTE, APP_TITLE};

const MESSAGE_404: &str = "L'indirizzo non esiste";
const UNDEFINED_ERROR_MESSAGE: &str = "Errore non identificato";
const BUTTON_MESSAGE: &str = "Accedi di nuovo";

struct RenderTemplate;

impl RenderTemplate {
    fn text(uri: &Origin<'_>, status: u16, error_message: &str) -> Template {
        Self::render(uri, "/", status, error_message)
    }

    fn administration(uri: &Origin<'_>, status: u16, error_message: &str) -> Template {
        Self::render(
            uri,
            ADMINISTRATION_ROUTE.path().as_str(),
            status,
            error_message,
        )
    }

    fn render(uri: &Origin<'_>, route: &str, status: u16, error_message: &str) -> Template {
        Template::render(
            "error",
            context! {
                title: APP_TITLE,
                route,
                uri,
                status,
                error_message,
                button_message: BUTTON_MESSAGE,
            },
        )
    }
}

#[derive(Responder)]
#[response(status = 500, content_type = "html")]
pub(crate) struct InternalError(Template);

impl InternalError {
    // Render a text containing an internal error
    pub(crate) fn text(uri: &Origin<'_>, error_message: &str) -> Self {
        Self(RenderTemplate::text(uri, 500, error_message))
    }

    // Render a text containing an attempt to access without authentication
    pub(crate) fn not_authenticated(uri: &Origin<'_>) -> Self {
        Self(RenderTemplate::text(
            uri,
            403,
            "Bisogna autenticarsi per vedere questa pagina",
        ))
    }

    // Render a text containing an attempt to access without authentication
    pub(crate) fn not_authenticated_administrator(uri: &Origin<'_>) -> Self {
        Self(RenderTemplate::administration(
            uri,
            403,
            "Bisogna autenticarsi come amministratori per vedere questa pagina",
        ))
    }

    // Check if card inserted is associated to the current user
    pub(crate) fn wrong_card_id(uri: &Origin<'_>) -> Self {
        Self(RenderTemplate::text(
            uri,
            403,
            "Numero di tessera inserito non è il tuo",
        ))
    }
}

pub(crate) async fn query_error<T, K: ToString>(
    function: impl std::future::Future<Output = Result<T, K>>,
    uri: &Origin<'_>,
) -> Result<T, InternalError> {
    function
        .await
        .map_err(|e| InternalError::text(uri, &e.to_string()))
}

// Renders the template for any other kind of catchers
#[catch(default)]
pub(crate) fn default(status: Status, req: &Request<'_>) -> Template {
    RenderTemplate::text(
        req.uri(),
        status.code,
        status.reason().unwrap_or(UNDEFINED_ERROR_MESSAGE),
    )
}

// Renders the template for a not found route
#[catch(404)]
pub(crate) async fn not_found(req: &Request<'_>) -> Template {
    RenderTemplate::text(req.uri(), 404, MESSAGE_404)
}

// Returns all defined catchers
pub(crate) fn catchers() -> Vec<rocket::Catcher> {
    catchers![default, not_found]
}
