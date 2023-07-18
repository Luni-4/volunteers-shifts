#![allow(clippy::declare_interior_mutable_const)]
#![allow(clippy::borrow_interior_mutable_const)]

#[macro_use]
extern crate rocket;

mod administration;
mod authentication;
mod cookie;
mod data;
mod database;
mod error;
mod shifts;
mod shifts_manager;
mod success;
mod time;
mod visualizer;
mod volunteers;

use anyhow::anyhow;

use rocket::config::Config;
use rocket::fairing::AdHoc;
use rocket::http::uri::Origin;
use rocket::http::{CookieJar, Method};
use rocket::request::FlashMessage;
use rocket::tokio::sync::broadcast::{channel, Sender};
use rocket_dyn_templates::Template;

use shuttle_secrets::SecretStore;

use authentication::{check_authentication, show_authentication};
use shifts::remove_volunteer_shift;
use shifts_manager::{add_shifts_current_week, delete_volunteer_shift, remove_shifts_current_week};
use visualizer::{change_day_left, change_day_right, change_week};

use data::string_len_as_number;
use database::{create_volunteers_shifts_tables, fill_volunteers_table};
use time::Date;

const APP_TITLE: &str = "Turni Volontari";

// Italian routes
const VOLUNTEER_ROUTE: Origin<'static> = uri!("/volontari");
const SHIFTS_ROUTE: Origin<'static> = uri!("/turni");
const SHIFTS_MANAGER_ROUTE: Origin<'static> = uri!("/gestoreturni");
const VISUALIZE_SHIFTS_ROUTE: Origin<'static> = uri!("/visualizzaturni");
const ADMINISTRATION_ROUTE: Origin<'static> = uri!("/amministrazione");
const SUCCESS_ROUTE: Origin<'static> = uri!("/successo");
const COOKIE_ROUTE: Origin<'static> = uri!("/cookie");

// Cookies
const AUTHENTICATION_COOKIE: &str = "authenticated";
const CARD_COOKIE: &str = "card_id";
const SURNAME_COOKIE: &str = "surname";
const ADMINISTRATION_COOKIE: &str = "administration";
const POLICY_COOKIE: &str = "cookie-policy";

#[get("/")]
async fn index(flash: Option<FlashMessage<'_>>, jar: &CookieJar<'_>) -> Template {
    show_authentication(flash, jar).await
}

// Save database state among various route calls
pub(crate) struct DatabaseState {
    pub(crate) pool: sqlx::PgPool,
    pub(crate) sender: Sender<u8>,
    pub(crate) volunteers_url: String,
    pub(crate) administration_password: String,
    pub(crate) email: String,
    pub(crate) website: String,
}

#[shuttle_runtime::main]
async fn rocket(
    #[shuttle_shared_db::Postgres] pool: sqlx::PgPool,
    #[shuttle_secrets::Secrets] secret_store: SecretStore,
) -> shuttle_rocket::ShuttleRocket {
    // Get volunteers file URL
    let volunteers_url = if let Some(url) = secret_store.get("VOLUNTEERS_FILE_URL") {
        url
    } else {
        return Err(anyhow!("Volunteers file URL not found").into());
    };

    // Get administration password
    let administration_password =
        if let Some(password) = secret_store.get("ADMINISTRATION_PASSWORD") {
            password
        } else {
            return Err(anyhow!("Administration password not found").into());
        };

    // Get email for cookie policy
    let email = if let Some(email) = secret_store.get("EMAIL") {
        email
    } else {
        return Err(anyhow!("Email for cookie policy not found").into());
    };

    // Get website for cookie policy
    let website = if let Some(website) = secret_store.get("WEBSITE") {
        website
    } else {
        return Err(anyhow!("Website for cookie policy not found").into());
    };

    // Get rocket secret key for private cookies
    let rocket_secret_key = if let Some(secret_key) = secret_store.get("ROCKET_SECRET_KEY") {
        secret_key
    } else {
        return Err(anyhow!("Rocket secret key not found").into());
    };

    // Configure secret key for rocket
    let figment = Config::figment().merge(("secret_key", rocket_secret_key));

    // Create volunteers and shifts tables
    create_volunteers_shifts_tables(&pool).await?;

    // Fill volunteers table
    fill_volunteers_table(&pool, &volunteers_url).await?;

    let rocket = rocket::custom(figment)
        .mount(
            "/",
            routes![
                index,
                check_authentication,
                add_shifts_current_week,
                remove_shifts_current_week,
                remove_volunteer_shift,
                delete_volunteer_shift,
                change_week,
                change_day_left,
                change_day_right,
            ],
        )
        .mount(VOLUNTEER_ROUTE, volunteers::routes())
        .mount(SHIFTS_ROUTE, shifts::routes())
        .mount(VISUALIZE_SHIFTS_ROUTE, visualizer::routes())
        .mount(SHIFTS_MANAGER_ROUTE, shifts_manager::routes())
        .mount(ADMINISTRATION_ROUTE, administration::routes())
        .mount(SUCCESS_ROUTE, success::routes())
        .mount(COOKIE_ROUTE, cookie::routes())
        .manage(DatabaseState {
            pool,
            sender: channel::<u8>(8).0,
            volunteers_url,
            administration_password,
            email,
            website,
        })
        .attach(AdHoc::on_request("Reset visualizer cookies", |req, _| {
            Box::pin(async move {
                if req.method() == Method::Get
                    && req.uri().path() == VISUALIZE_SHIFTS_ROUTE.path()
                    && !req.headers().contains("referer")
                {
                    req.cookies().add(("week", "0"));
                    req.cookies().add((
                        "day",
                        string_len_as_number(Date::current().day_as_number() + 1),
                    ));
                }
            })
        }))
        .attach(Template::fairing())
        .register("/", error::catchers());

    Ok(rocket.into())
}
