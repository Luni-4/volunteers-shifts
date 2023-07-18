use rocket::http::uri::Origin;
use rocket::http::CookieJar;
use rocket::response::Redirect;
use rocket::State;

use rocket_dyn_templates::{context, Template};

use serde::Serialize;

use crate::{DatabaseState, APP_TITLE, COOKIE_ROUTE, POLICY_COOKIE};

#[derive(Serialize, Default)]
pub(crate) struct CookieMessage {
    // Check whether it is the first time for cookie popup
    is_first: bool,
    // Cookie title
    title: Option<&'static str>,
    // Route
    route: Option<Origin<'static>>,
    // Link text
    link: Option<&'static str>,
    // Accept button
    accept: Option<&'static str>,
    // Hide cookie dialog route
    hide_dialog_route: Option<String>,
    // Decline button
    decline: Option<&'static str>,
}

impl CookieMessage {
    pub(crate) fn render(jar: &CookieJar<'_>, uri: Option<&str>) -> Self {
        if jar.get(POLICY_COOKIE).is_none() {
            Self {
                is_first: true,
                title: Some("Questo sito usa i cookie!"),
                route: Some(COOKIE_ROUTE),
                link: Some("Ma cosa sono i cookie?"),
                hide_dialog_route: Some(format!("{COOKIE_ROUTE}{}", uri.unwrap_or_default())),
                accept: Some("Accetto"),
                decline: Some("Declino"),
            }
        } else {
            Self::default()
        }
    }
}

#[put("/")]
async fn hide_index_cookie_dialog(jar: &CookieJar<'_>) -> Redirect {
    jar.add((POLICY_COOKIE, "1"));
    Redirect::to(uri!(crate::authentication::show_authentication))
}

#[put("/<redirect>")]
async fn hide_pages_cookie_dialog(redirect: &str, jar: &CookieJar<'_>) -> Redirect {
    jar.add((POLICY_COOKIE, "1"));
    Redirect::to(format!("/{redirect}"))
}

#[get("/")]
async fn show_cookie_policy(database_state: &State<DatabaseState>) -> Template {
    Template::render(
        "cookie_policy",
        context! {
            title: APP_TITLE,
            website_link: &database_state.website,
            email: &database_state.email,
        },
    )
}

pub(crate) fn routes() -> Vec<rocket::Route> {
    routes![
        show_cookie_policy,
        hide_index_cookie_dialog,
        hide_pages_cookie_dialog
    ]
}
