pub(crate) mod administration;
pub(crate) mod authentication;

use rocket::http::{Cookie, CookieJar};

// Cookies
pub(crate) const AUTHENTICATION_COOKIE: &str = "authenticated";
pub(crate) const ADMINISTRATION_COOKIE: &str = "administration";
pub(crate) const CARD_COOKIE: &str = "card_id";
pub(crate) const SURNAME_COOKIE: &str = "surname";

#[inline(always)]
pub(crate) fn get_cookie_value(jar: &CookieJar<'_>, name: &str) -> Option<String> {
    jar.get_private(name)
        .as_ref()
        .map(Cookie::value)
        .map(|value| value.to_string())
}

#[inline(always)]
pub(crate) fn get_cookie_value_str(jar: &CookieJar, key: &str) -> &'static str {
    jar.get_private(key)
        .as_ref()
        .and_then(Cookie::value_raw)
        .unwrap_or_default()
}
