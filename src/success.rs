use rocket::http::uri::Origin;
use rocket_dyn_templates::{context, Template};

use serde::Serialize;

use crate::{APP_TITLE, VISUALIZE_SHIFTS_ROUTE};

// Success page information
#[derive(Serialize)]
struct Success {
    // Success message
    congrats_message: &'static str,
    // Information message
    message: &'static str,
    // Route to shifts visualizer
    route: Origin<'static>,
    // Button message to redirect to the shifts visualizer
    visualizer_message: &'static str,
}

impl Success {
    fn text() -> Self {
        Self {
            congrats_message: "Congratulazioni!",
            route: VISUALIZE_SHIFTS_ROUTE,
            message: "I tuoi dati sono stati inseriti correttamente!",
            visualizer_message: "Vai ai turni!",
        }
    }
}

#[post("/")]
pub(crate) async fn show_success() -> Template {
    Template::render(
        "success",
        context! {
            title: APP_TITLE,
            success: Success::text(),
        },
    )
}

pub(crate) fn routes() -> Vec<rocket::Route> {
    routes![show_success,]
}
