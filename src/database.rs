use std::collections::HashSet;

use anyhow::{anyhow, Result};

use serde::{Deserialize, Serialize};

use sqlx::postgres::PgQueryResult;
use sqlx::types::chrono::NaiveDate;
use sqlx::{Executor, FromRow, PgPool};

use crate::time::Date;

const INSERT_VOLUNTEERS: &str =
    "INSERT INTO volunteers(card_id, surname, name, phone_number, disabled)
     VALUES ($1, $2, $3, $4, $5)";
const UPDATE_VOLUNTEER: &str = "UPDATE volunteers
     SET surname = $2,
         name = $3,
         phone_number = $4,
         disabled = $5
     WHERE card_id = $1";
const INSERT_SHIFTS: &str = "INSERT INTO shifts(date, task, card_id)
     VALUES ($1, $2, $3)";
const GET_ALL_VOLUNTEERS: &str =
    "SELECT card_id, surname, name, phone_number, disabled FROM volunteers ORDER BY card_id ASC";
const GET_ALL_SHIFTS: &str =
    "SELECT date, task, card_id FROM shifts WHERE date >= $1 ORDER BY date ASC";
const GET_VOLUNTEER_SHIFTS: &str =
    "SELECT id, date, task, card_id FROM shifts WHERE card_id = $1 AND date >= $2 ORDER BY date ASC";
const GET_VOLUNTEER_CURRENT_SHIFTS: &str =
    "SELECT date, task, card_id FROM shifts WHERE card_id = $1 AND date >= $2 ORDER BY date ASC";
const GET_VOLUNTEER_NAME: &str = "SELECT name FROM volunteers WHERE card_id = $1";
const GET_VOLUNTEER_NAME_SURNAME: &str = "SELECT name, surname FROM volunteers WHERE card_id = $1";
const GET_VOLUNTEERS_NAMES: &str =
    "SELECT name, surname FROM volunteers JOIN shifts ON volunteers.card_id = shifts.card_id
     WHERE date = $1 AND task = $2";
const CHECK_VOLUNTEER_FROM_CARD_ID: &str = "SELECT COUNT(*) FROM volunteers WHERE card_id = $1";
const CHECK_VOLUNTEER_FROM_SURNAME: &str = "SELECT COUNT(*) FROM volunteers WHERE surname = $1";
const CHECK_VOLUNTEER_CARD_ID_TO_SURNAME: &str =
    "SELECT COUNT(*) FROM volunteers WHERE card_id = $1 AND surname = $2";
const CHECK_VOLUNTEER_IS_DISABLED: &str =
    "SELECT COUNT(*) FROM volunteers WHERE card_id = $1 AND disabled = TRUE";
const DELETE_SHIFT: &str = "DELETE FROM shifts WHERE id = $1";
const DELETE_OLD_SHIFTS: &str = "DELETE FROM shifts WHERE date < CURRENT_DATE - interval '3 weeks'";

// FIXME: Leave this hack until a new sqlx version is released
#[inline(always)]
fn italian_day(day: &str) -> &'static str {
    match day {
        "Monday" => "Lunedì",
        "Tuesday" => "Martedì",
        "Wednesday" => "Mercoledì",
        "Thursday" => "Giovedì",
        "Friday" => "Venerdì",
        "Saturday" => "Sabato",
        "Sunday" => "Domenica",
        _ => unreachable!(),
    }
}

#[inline(always)]
pub(crate) fn insert_db_date(date: (i32, u32, u32)) -> Option<NaiveDate> {
    NaiveDate::from_ymd_opt(date.0, date.1, date.2)
}

#[inline(always)]
pub(crate) fn format_db_date(date: &NaiveDate) -> String {
    format!(
        "{} {}",
        italian_day(date.format("%A").to_string().as_str()),
        date.format("%d/%m/%Y"),
    )
}

#[inline(always)]
fn insert_db_date_error(date: (i32, u32, u32)) -> Result<NaiveDate> {
    insert_db_date(date).ok_or(anyhow!("Error creating database date"))
}

#[inline(always)]
fn database_current_date() -> Result<NaiveDate> {
    let date = Date::current();
    insert_db_date_error((date.year(), date.month(), date.day()))
}

/// Volunteer definition
#[derive(Debug, FromRow, Serialize, Deserialize)]
pub(crate) struct Volunteer {
    /// Card identification (database primary key)
    pub(crate) card_id: i16,
    /// Surname
    pub(crate) surname: String,
    /// Name
    pub(crate) name: String,
    /// Check whether the volunteer is disabled
    pub(crate) disabled: bool,
}

/// Volunteer manager definition
#[derive(Debug, FromRow, Serialize, Deserialize)]
pub(crate) struct VolunteerManager {
    /// Visible volunteer information
    #[sqlx(flatten)]
    pub(crate) volunteer: Volunteer,
    /// Phone number
    pub(crate) phone_number: String,
}

/// Shift definition
#[derive(PartialEq, Eq, Hash, Clone, FromRow, Serialize, Deserialize)]
pub(crate) struct Shift {
    /// Chosen date
    pub(crate) date: NaiveDate,
    /// Task
    pub(crate) task: i16,
    /// Card identification
    pub(crate) card_id: i16,
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

// Get shifts associated to a volunteer and retrieved through card identification
pub(crate) async fn query_shifts(pool: &PgPool, card_id: i16) -> Result<Vec<ShiftQuery>> {
    let date = database_current_date()?;

    Ok(sqlx::query_as(GET_VOLUNTEER_SHIFTS)
        .bind(card_id)
        .bind(date)
        .fetch_all(pool)
        .await?)
}

// Get all shifts
pub(crate) async fn query_all_shifts(pool: &PgPool) -> Result<Vec<Shift>> {
    let date = database_current_date()?;
    Ok(sqlx::query_as(GET_ALL_SHIFTS)
        .bind(date)
        .fetch_all(pool)
        .await?)
}

// Get shifts associated to a volunteer (without id) and retrieved through card identification
pub(crate) async fn query_current_shifts(pool: &PgPool, card_id: i16) -> Result<HashSet<Shift>> {
    let date = database_current_date()?;
    let shifts = sqlx::query_as(GET_VOLUNTEER_CURRENT_SHIFTS)
        .bind(card_id)
        .bind(date)
        .fetch_all(pool)
        .await?;
    Ok(HashSet::from_iter(shifts.iter().cloned()))
}

// Check whether a volunteer exists from card identification
pub(crate) async fn query_check_card_id(pool: &PgPool, card_id: i16) -> Result<bool> {
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
    card_id: i16,
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
pub(crate) async fn query_is_disabled(pool: &PgPool, card_id: i16) -> Result<bool> {
    #[derive(FromRow, Serialize, Deserialize)]
    struct Temp(i64);
    let single_row: Temp = sqlx::query_as(CHECK_VOLUNTEER_IS_DISABLED)
        .bind(card_id)
        .fetch_one(pool)
        .await?;
    Ok(single_row.0 == 1)
}

// Get volunteer name and surname
pub(crate) async fn query_volunteer_surname_name(pool: &PgPool, id: i16) -> Result<String> {
    #[derive(FromRow, Serialize, Deserialize)]
    struct Temp {
        name: String,
        surname: String,
    }
    let volunteer: Temp = sqlx::query_as(GET_VOLUNTEER_NAME_SURNAME)
        .bind(id)
        .fetch_one(pool)
        .await?;
    // Join volunteer name and surname into a single string
    Ok(format!("{} {}", volunteer.surname, volunteer.name))
}

// Get volunteers names associated to specific shifts
pub(crate) async fn query_volunteers_shifts(
    pool: &PgPool,
    date: (i32, u32, u32),
    task: i16,
) -> Result<Vec<String>> {
    #[derive(FromRow, Serialize, Deserialize)]
    struct Temp {
        name: String,
        surname: String,
    }
    let date = insert_db_date_error(date)?;
    let volunteers: Vec<Temp> = sqlx::query_as(GET_VOLUNTEERS_NAMES)
        .bind(date)
        .bind(task)
        .fetch_all(pool)
        .await?;
    // Join volunteer's name and surname into a single string
    Ok(volunteers
        .iter()
        .map(|volunteer| format!("{} {}", volunteer.name, volunteer.surname))
        .collect())
}

// Get volunteer name from card identification
pub(crate) async fn query_volunteer_name(pool: &PgPool, card_id: i16) -> Result<String> {
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

// Get all volunteers data
pub(crate) async fn query_volunteers(pool: &PgPool) -> Result<Vec<VolunteerManager>> {
    Ok(sqlx::query_as(GET_ALL_VOLUNTEERS).fetch_all(pool).await?)
}

// Fill volunteers table
pub(crate) async fn fill_volunteers_table(pool: &PgPool, volunteers_url: &str) -> Result<()> {
    // Download volunteers from Google Sheet file and return them
    let volunteers = download_file(volunteers_url).await?;

    // Insert data inside volunteers table
    for volunteer in &volunteers {
        insert_update_volunteer(pool, INSERT_VOLUNTEERS, volunteer).await?;
    }

    Ok(())
}

// Refill volunteers table
pub(crate) async fn refill_volunteers_table(pool: &PgPool, volunteers_url: &str) -> Result<()> {
    // Download volunteers from Google Sheet file and return them
    let volunteers = download_file(volunteers_url).await?;

    // Update or insert new data inside volunteers table
    for volunteer in &volunteers {
        // If a volunteer is already present in the table, update data,
        // otherwise insert new data
        if query_check_card_id(pool, volunteer.volunteer.card_id).await? {
            insert_update_volunteer(pool, UPDATE_VOLUNTEER, volunteer).await?;
        } else {
            insert_update_volunteer(pool, INSERT_VOLUNTEERS, volunteer).await?;
        }
    }

    Ok(())
}

// Fill shifts table
//
// Do not check whether a shift is already present through card id and date
// because a cookie to avoid inserting the same data again
pub(crate) async fn fill_shifts_table(pool: &PgPool, shifts: HashSet<Shift>) -> Result<()> {
    // Insert data inside shifts table
    for shift in shifts {
        sqlx::query(INSERT_SHIFTS)
            .bind(shift.date)
            .bind(shift.task)
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
                phone_number: row[7].to_string(),
                volunteer: Volunteer {
                    card_id: row[1].parse::<i16>()?,
                    name: row[3].to_string(),
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
        .bind(&volunteer.phone_number)
        .bind(volunteer.volunteer.disabled)
        .execute(pool)
        .await?;
    Ok(())
}
