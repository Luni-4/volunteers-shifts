#[macro_use]
extern crate rocket;

mod cookie;
mod data;
mod database;
mod download_database;
mod error;
mod login;
mod menu;
mod shifts;
mod shifts_manager;
mod time;
mod visualizer;
mod volunteers;

use anyhow::anyhow;

use rocket::config::Config;
use rocket::fs::{relative, FileServer};
use rocket::http::uri::Origin;
use rocket::http::CookieJar;
use rocket::request::FlashMessage;
use rocket::tokio::sync::broadcast::{channel, Sender};
use rocket_dyn_templates::Template;

use shuttle_secrets::SecretStore;

use login::authentication::{check_authentication, show_authentication};
use shifts::remove_shift;
use shifts_manager::add_shift;
use volunteers::update_volunteers;

use database::{create_volunteers_shifts_tables, fill_volunteers_table};

const APP_TITLE: &str = "Turni Volontari";
const DISABLED_MESSAGE: &str = "Tessera Disabilitata";

// Italian routes
const VOLUNTEERS_ROUTE: Origin<'static> = uri!("/volontari");
const SHIFTS_ROUTE: Origin<'static> = uri!("/turni");
const SHIFTS_MANAGER_ROUTE: Origin<'static> = uri!("/gestoreturni");
const VISUALIZE_SHIFTS_ROUTE: Origin<'static> = uri!("/visualizzaturni");
const ADMINISTRATION_ROUTE: Origin<'static> = uri!("/referenti");
const COOKIE_ROUTE: Origin<'static> = uri!("/cookie");
const DOWNLOAD_DATABASE_ROUTE: Origin<'static> = uri!("/download/database");

// Cookies
const POLICY_COOKIE: &str = "cookie-policy";

#[get("/")]
async fn index(flash: Option<FlashMessage<'_>>, jar: &CookieJar<'_>) -> Template {
    show_authentication(flash, jar).await
}

// Access app state among various route calls
pub(crate) struct AppState {
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
                add_shift,
                remove_shift,
                update_volunteers
            ],
        )
        .mount(VOLUNTEERS_ROUTE, volunteers::routes())
        .mount(SHIFTS_ROUTE, shifts::routes())
        .mount(VISUALIZE_SHIFTS_ROUTE, visualizer::routes())
        .mount(SHIFTS_MANAGER_ROUTE, shifts_manager::routes())
        .mount(ADMINISTRATION_ROUTE, login::administration::routes())
        .mount(COOKIE_ROUTE, cookie::routes())
        .mount(DOWNLOAD_DATABASE_ROUTE, download_database::routes())
        .mount("/", FileServer::from(relative!("assets")).rank(4))
        .mount(SHIFTS_ROUTE, FileServer::from(relative!("assets")).rank(5))
        .manage(AppState {
            pool,
            sender: channel::<u8>(8).0,
            volunteers_url,
            administration_password,
            email,
            website,
        })
        .attach(Template::fairing())
        .register("/", error::catchers());

    Ok(rocket.into())
}
