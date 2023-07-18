use rocket::http::uri::Origin;

use serde::Serialize;

use crate::{
    ADMINISTRATION_ROUTE, SHIFTS_MANAGER_ROUTE, SHIFTS_ROUTE, VISUALIZE_SHIFTS_ROUTE,
    VOLUNTEERS_ROUTE,
};

#[derive(Serialize)]
pub(crate) struct Menu {
    // Image path
    image_path: &'static str,
    // Route to index
    index_route: Origin<'static>,
    // Route to insert shifts
    insert_shifts_route: Origin<'static>,
    // Link text for route to insert shifts
    insert_shifts_text: &'static str,
    // Route to view personal shifts
    personal_shifts_route: Origin<'static>,
    // Link text for view personal shifts
    personal_shifts_text: &'static str,
    // Route to visualize shifts
    visualize_shifts_route: Origin<'static>,
    // Link text for visualize shifts
    visualize_shifts_text: &'static str,
}

impl Menu {
    pub(crate) fn render(card_id: i16) -> Self {
        Self {
            image_path: "img/logo.png",
            index_route: uri!("/"),
            insert_shifts_route: uri!(
                SHIFTS_MANAGER_ROUTE,
                crate::shifts_manager::show_shifts_manager(card_id)
            ),
            insert_shifts_text: "Inserisci turni",
            personal_shifts_route: uri!(SHIFTS_ROUTE, crate::shifts::show_shifts(card_id)),
            personal_shifts_text: "I tuoi turni",
            visualize_shifts_route: uri!(
                VISUALIZE_SHIFTS_ROUTE,
                crate::visualizer::visualize_shifts
            ),
            visualize_shifts_text: "Vedi turni",
        }
    }
}

#[derive(Serialize)]
pub(crate) struct MenuAdministration {
    // Image path
    image_path: &'static str,
    // Route to index
    index_route: Origin<'static>,
    // Route to volunteers page
    volunteers_route: Origin<'static>,
    // Link text for route to volunteers
    volunteers_text: &'static str,
    // Route to visualize shifts
    visualize_shifts_route: Origin<'static>,
    // Link text for visualize shifts
    visualize_shifts_text: &'static str,
}

impl MenuAdministration {
    pub(crate) fn render() -> Self {
        Self {
            image_path: "img/logo.png",
            index_route: ADMINISTRATION_ROUTE,
            volunteers_route: VOLUNTEERS_ROUTE,
            volunteers_text: "Volontari",
            visualize_shifts_route: uri!(
                VISUALIZE_SHIFTS_ROUTE,
                crate::visualizer::visualize_shifts
            ),
            visualize_shifts_text: "Vedi turni",
        }
    }
}
