use std::collections::HashSet;

use anyhow::Result;

use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};

use sqlx::postgres::PgQueryResult;
use sqlx::types::time;
use sqlx::{Executor, FromRow, PgPool};

const INSERT_VOLUNTEER: &str =
    "INSERT INTO volunteers(card_id, surname, name, fiscal_code, disabled)
     VALUES ($1, $2, $3, $4, $5)
     RETURNING card_id, surname, name, fiscal_code, disabled";
const UPDATE_VOLUNTEER: &str = "UPDATE volunteers
     SET surname = $2,
         name = $3,
         fiscal_code = $4,
         disabled = $5
     WHERE card_id = $1";
const INSERT_SHIFTS: &str = "INSERT INTO shifts(date, day, task, entrance_hour, exit_hour, card_id)
     VALUES ($1, $2, $3, $4, $5, $6)";
const GET_VOLUNTEER_SHIFTS: &str =
    "SELECT id, date, day, task, entrance_hour, exit_hour, card_id FROM shifts WHERE card_id = $1 ORDER BY date ASC, entrance_hour ASC";
const GET_VOLUNTEER_CURRENT_SHIFTS: &str =
    "SELECT date, day, task, entrance_hour, exit_hour, card_id FROM shifts WHERE card_id = $1 ORDER BY date ASC, entrance_hour ASC";
const GET_VOLUNTEER_INFO: &str =
    "SELECT card_id, name, surname, disabled FROM volunteers ORDER BY card_id";
const GET_VOLUNTEER_NAME: &str = "SELECT name FROM volunteers WHERE card_id = $1";
const GET_VOLUNTEERS_NAMES: &str =
    "SELECT name, surname FROM volunteers JOIN shifts ON volunteers.card_id = shifts.card_id
     WHERE date = $1 AND day = $2 AND task = $3 AND entrance_hour = $4 AND exit_hour = $5";
const CHECK_VOLUNTEER_FROM_CARD_ID: &str = "SELECT COUNT(*) FROM volunteers WHERE card_id = $1";
const CHECK_VOLUNTEER_FROM_SURNAME: &str = "SELECT COUNT(*) FROM volunteers WHERE surname = $1";
const CHECK_VOLUNTEER_CARD_ID_TO_SURNAME: &str =
    "SELECT COUNT(*) FROM volunteers WHERE card_id = $1 AND surname = $2";
const CHECK_VOLUNTEER_IS_DISABLED: &str =
    "SELECT COUNT(*) FROM volunteers WHERE card_id = $1 AND disabled = TRUE";
const DELETE_SHIFT: &str = "DELETE FROM shifts WHERE id = $1";
const DELETE_OLD_SHIFTS: &str = "DELETE FROM shifts WHERE row_deadline < now()";

/// Volunteer definition
#[derive(FromRow, Serialize, Deserialize)]
pub(crate) struct Volunteer {
    /// Card identification (database primary key)
    pub(crate) card_id: i32,
    /// Surname
    pub(crate) surname: String,
    /// Name
    pub(crate) name: String,
    /// Check whether the volunteer is disabled
    pub(crate) disabled: bool,
}

/// Volunteer manager definition
#[derive(FromRow, Serialize, Deserialize)]
pub(crate) struct VolunteerManager {
    /// Visible volunteer information
    #[sqlx(flatten)]
    pub(crate) volunteer: Volunteer,
    /// Fiscal code
    pub(crate) fiscal_code: String,
}

/// Shift definition
#[derive(PartialEq, Eq, Hash, Clone, FromRow, Serialize, Deserialize)]
pub(crate) struct Shift {
    /// Chosen date
    pub(crate) date: String,
    /// Chosen day
    pub(crate) day: String,
    /// Task
    pub(crate) task: String,
    /// Entrance hour
    pub(crate) entrance_hour: String,
    /// Exit hour
    pub(crate) exit_hour: String,
    /// Card identification
    pub(crate) card_id: i32,
}

/// Shift query information
#[derive(FromRow, Serialize, Deserialize)]
pub(crate) struct ShiftQuery {
    /// Id to discriminate the rows (database primary key)
    pub(crate) id: i32,
    /// Visible shift information
    #[sqlx(flatten)]
    pub(crate) shift: Shift,
}

/// Shift manager definition
#[derive(FromRow)]
pub(crate) struct ShiftManager {
    /// Visible shift information
    #[sqlx(flatten)]
    pub(crate) shift: Shift,
    /// Row deadline
    pub(crate) row_deadline: time::PrimitiveDateTime,
}

impl Serialize for ShiftManager {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut st = serializer.serialize_struct("shift_manager", 2)?;
        st.serialize_field("shift", &self.shift)?;
        st.serialize_field("row_deadline", &self.row_deadline.to_string())?;
        st.end()
    }
}

// Get shifts associated to a volunteer and retrieved through card identification
pub(crate) async fn query_shifts(pool: &PgPool, card_id: i32) -> Result<Vec<ShiftQuery>> {
    Ok(sqlx::query_as(GET_VOLUNTEER_SHIFTS)
        .bind(card_id)
        .fetch_all(pool)
        .await?)
}

// Get shifts associated to a volunteer (without id) and retrieved through card identification
pub(crate) async fn query_current_shifts(pool: &PgPool, card_id: i32) -> Result<HashSet<Shift>> {
    let shifts = sqlx::query_as(GET_VOLUNTEER_CURRENT_SHIFTS)
        .bind(card_id)
        .fetch_all(pool)
        .await?;
    Ok(HashSet::from_iter(shifts.iter().cloned()))
}

// Check whether a volunteer exists from card identification
pub(crate) async fn query_check_card_id(pool: &PgPool, card_id: i32) -> Result<bool> {
    #[derive(FromRow, Serialize, Deserialize)]
    struct Temp(i64);
    let single_row: Temp = sqlx::query_as(CHECK_VOLUNTEER_FROM_CARD_ID)
        .bind(card_id)
        .fetch_one(pool)
        .await?;
    Ok(single_row.0 == 1)
}

// Check whether a volunteer exists from surname
pub(crate) async fn query_check_surname(pool: &PgPool, surname: &str) -> Result<bool> {
    #[derive(FromRow, Serialize, Deserialize)]
    struct Temp(i64);
    let single_row: Temp = sqlx::query_as(CHECK_VOLUNTEER_FROM_SURNAME)
        .bind(surname)
        .fetch_one(pool)
        .await?;
    Ok(single_row.0 > 0)
}

// Check whether a volunteer exists from surname
pub(crate) async fn query_card_id_to_surname(
    pool: &PgPool,
    card_id: i32,
    surname: &str,
) -> Result<bool> {
    #[derive(FromRow, Serialize, Deserialize)]
    struct Temp(i64);
    let single_row: Temp = sqlx::query_as(CHECK_VOLUNTEER_CARD_ID_TO_SURNAME)
        .bind(card_id)
        .bind(surname)
        .fetch_one(pool)
        .await?;
    Ok(single_row.0 == 1)
}

// Check whether a volunteer is disabled
pub(crate) async fn query_is_disabled(pool: &PgPool, card_id: i32) -> Result<bool> {
    #[derive(FromRow, Serialize, Deserialize)]
    struct Temp(i64);
    let single_row: Temp = sqlx::query_as(CHECK_VOLUNTEER_IS_DISABLED)
        .bind(card_id)
        .fetch_one(pool)
        .await?;
    Ok(single_row.0 == 1)
}

// Get volunteers names associated to specific shifts
pub(crate) async fn query_volunteers_shifts(
    pool: &PgPool,
    date: &str,
    day: &str,
    task: &str,
    entrance_hour: &str,
    exit_hour: &str,
) -> Result<Vec<String>> {
    #[derive(FromRow, Serialize, Deserialize)]
    struct Temp {
        name: String,
        surname: String,
    }
    let volunteers: Vec<Temp> = sqlx::query_as(GET_VOLUNTEERS_NAMES)
        .bind(date)
        .bind(day)
        .bind(task)
        .bind(entrance_hour)
        .bind(exit_hour)
        .fetch_all(pool)
        .await?;
    // Join volunteer's name and surname into a single string
    Ok(volunteers
        .iter()
        .map(|volunteer| format!("{} {}", volunteer.name, volunteer.surname))
        .collect())
}

// Get volunteers card identification, name, and surname
pub(crate) async fn query_volunteer_info(pool: &PgPool) -> Result<Vec<Volunteer>> {
    Ok(sqlx::query_as(GET_VOLUNTEER_INFO).fetch_all(pool).await?)
}

// Get volunteer name from card identification
pub(crate) async fn query_volunteer_name(pool: &PgPool, card_id: i32) -> Result<String> {
    #[derive(FromRow, Serialize, Deserialize)]
    struct Temp {
        name: String,
    }
    let volunteer: Temp = sqlx::query_as(GET_VOLUNTEER_NAME)
        .bind(card_id)
        .fetch_one(pool)
        .await?;
    Ok(volunteer.name)
}

// Fill volunteers table
pub(crate) async fn fill_volunteers_table(pool: &PgPool, volunteers_url: &str) -> Result<()> {
    // Download volunteers from Google Sheet file and return them
    let volunteers = download_file(volunteers_url).await?;

    // Insert data inside volunteers table
    for volunteer in &volunteers {
        insert_update_volunteer(pool, INSERT_VOLUNTEER, volunteer).await?;
    }

    Ok(())
}

// Refill volunteers table
pub(crate) async fn refill_volunteers_table(
    pool: &PgPool,
    volunteers_url: &str,
) -> Result<Vec<VolunteerManager>> {
    // Download volunteers from Google Sheet file and return them
    let mut volunteers = download_file(volunteers_url).await?;

    // Update or insert new data inside volunteers table
    for volunteer in &volunteers {
        // If a volunteer is already present in the table, update data,
        // otherwise insert new data
        if query_check_card_id(pool, volunteer.volunteer.card_id).await? {
            insert_update_volunteer(pool, UPDATE_VOLUNTEER, volunteer).await?;
        } else {
            insert_update_volunteer(pool, INSERT_VOLUNTEER, volunteer).await?;
        }
    }

    // Sort volunteers by card identification
    volunteers.sort_by(|a, b| a.volunteer.card_id.cmp(&b.volunteer.card_id));

    Ok(volunteers)
}

// Fill shifts table
//
// Do not check whether a shift is already present through card id and date
// because a cookie to avoid inserting the same data again
pub(crate) async fn fill_shifts_table(pool: &PgPool, shifts: HashSet<Shift>) -> Result<()> {
    // Insert data inside shifts table
    for shift in shifts {
        sqlx::query(INSERT_SHIFTS)
            .bind(&shift.date)
            .bind(&shift.day)
            .bind(&shift.task)
            .bind(&shift.entrance_hour)
            .bind(&shift.exit_hour)
            .bind(shift.card_id)
            .execute(pool)
            .await?;
    }
    Ok(())
}

// Delete a shift using the id
pub(crate) async fn delete_shift(pool: &PgPool, id: i32) -> Result<()> {
    // Delete a shift
    sqlx::query(DELETE_SHIFT).bind(id).execute(pool).await?;
    Ok(())
}

// Delete shifts with a date older than the current date
// https://www.postgresqltutorial.com/postgresql-tutorial/postgresql-delete/
pub(crate) async fn delete_old_shifts(pool: &PgPool) -> Result<()> {
    // Delete all shifts
    sqlx::query(DELETE_OLD_SHIFTS).execute(pool).await?;
    Ok(())
}

// Create volunteers and shifts tables at the start of application
pub(crate) async fn create_volunteers_shifts_tables(pool: &PgPool) -> Result<PgQueryResult> {
    Ok(pool.execute(include_str!("../sql/schema.sql")).await?)
}

// Download volunteer csv file
async fn download_file(volunteers_url: &str) -> Result<Vec<VolunteerManager>> {
    // Download volunteers file from Google Sheet
    let response = reqwest::get(volunteers_url).await?;

    // Get file content as text
    let body = response.text().await?;

    let mut volunteers = Vec::new();
    // Iterate on values contained in the csv file
    //
    // Skip the two header rows
    for row in csv::ReaderBuilder::new()
        .from_reader(body.as_bytes())
        .records()
        .skip(2)
    {
        if let Ok(row) = row {
            volunteers.push(VolunteerManager {
                fiscal_code: row[5].to_string(),
                volunteer: Volunteer {
                    card_id: row[1].parse::<i32>()?,
                    name: row[4].to_string(),
                    surname: row[2].to_string(),
                    disabled: !row[0].is_empty(),
                },
            })
        } else {
            continue;
        }
    }
    Ok(volunteers)
}

// Run insert and update volunteers queries
async fn insert_update_volunteer(
    pool: &PgPool,
    query: &str,
    volunteer: &VolunteerManager,
) -> Result<()> {
    sqlx::query(query)
        .bind(volunteer.volunteer.card_id)
        .bind(&volunteer.volunteer.surname)
        .bind(&volunteer.volunteer.name)
        .bind(&volunteer.fiscal_code)
        .bind(volunteer.volunteer.disabled)
        .execute(pool)
        .await?;
    Ok(())
}
