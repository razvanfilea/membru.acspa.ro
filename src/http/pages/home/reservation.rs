use crate::http::AppState;
use crate::model::role::UserRole;
use crate::model::user::UserUi;
use chrono::{Datelike, NaiveDate, Weekday};
use sqlx::{query, query_as, SqlitePool};

pub async fn is_free_day(pool: &SqlitePool, date: &NaiveDate) -> bool {
    let exists_in_table = async {
        query!(
            "select exists(select true from free_days where date = $1) as 'exists!'",
            date
        )
        .fetch_one(pool)
        .await
        .expect("Database error")
        .exists
            != 0
    };

    date.weekday() == Weekday::Sat || date.weekday() == Weekday::Sun || exists_in_table.await
}

pub async fn create_reservation(
    state: &AppState,
    user: UserUi,
    selected_date: NaiveDate,
    selected_hour: u8,
) -> Result<String, String> {
    let pool = &state.pool;
    // let mut transaction = pool.begin().await.expect("Database error");
    let role = query_as!(
        UserRole,
        "select * from user_roles where name = $1",
        user.role
    )
    .fetch_one(pool)
    .await
    .expect("Database error");

    // Check if it already exists
    let already_exists = query!(
        "select exists(select true from reservations where location = $1 and date = $2 and hour = $3 and user_id = $4 and cancelled = false) as 'exists!'",
        state.location.id,
        selected_date,
        selected_hour,
        user.id
    )
    .fetch_one(pool)
    .await
    .expect("Database error")
    .exists;

    if already_exists == 1 {
        return Err("Ai făcut deja o astfel de rezervare.".to_string());
    }

    let restriction = query!(
        "select message from reservations_restrictions where location = $1 and date = $2 and hour = $3",
        state.location.id,
        selected_date,
        selected_hour
    )
    .fetch_optional(pool)
    .await
    .expect("Database error");

    // Check if there is a restriction
    if let Some(restriction) = restriction {
        return Err(restriction.message.unwrap_or_else(|| {
            "Rezervările sunt restricționate la această oră și dată".to_string()
        }));
    }

    let fixed_reservations_count = query!(r#"select (
    (select count(*) from reservations where location = $1 and date = $2 and hour = $3 and cancelled = false) +
    (select count(*) from guests where location = $1 and date = $2 and hour = $3 and special = true)) as 'count!'"#,
        state.location.id, selected_date, selected_hour)
        .fetch_one(pool)
        .await
        .expect("Database error")
        .count as i64;

    if fixed_reservations_count > state.location.slot_capacity {
        return Err("S-a atins numărul maxim de rezervări pentru intervalul orar".to_string());
    }

    let user_reservations_count = query!(
        r#"select count(*) as 'count!' from reservations 
            where user_id = $1 and cancelled = false and
            strftime('%Y%W', date) = strftime('%Y%W', $2) and
            strftime('%w', date) != 0 and strftime('%w', date) != 6"#,
        user.id,
        selected_date
    )
    .fetch_one(pool)
    .await
    .expect("Database error")
    .count as i64;

    let user_reservations_as_guest_count = query!(
        r#"select count(*) as 'count!' from guests
            where created_by = $1 and
            strftime('%Y%W', date) = strftime('%Y%W', $2) and
            strftime('%w', date) != 0 and strftime('%w', date) != 6"#,
        user.id,
        selected_date
    )
    .fetch_one(pool)
    .await
    .expect("Database error")
    .count as i64;

    let is_free_day = is_free_day(pool, &selected_date).await;

    // Think of all cases
    if user_reservations_count >= role.reservations && !is_free_day {
        if user_reservations_as_guest_count >= role.as_guest && !is_free_day {
            return Err("Ți-ai epuizat rezervările pe săptămâna aceasta".to_string());
        }

        query!("insert into guests (guest_name, location, date, hour, created_by) values ($1, $2, $3, $4, $5)",
                user.name, state.location.id, selected_date, selected_hour, user.id)
            .execute(pool)
            .await
            .expect("Database error");

        // TODO Review message for all cases
        Ok(format!("Ai fost înscris ca invitat de la ora {}. Nu poți face decât {} rezervări pe săptămână (luni - vineri) ca și {}",
                   selected_hour, role.reservations, role.name))
    } else {
        query!("insert into reservations (user_id, location, date, hour) values ($1, $2, $3, $4)",
                user.id, state.location.id, selected_date, selected_hour)
            .execute(pool)
            .await
            .expect("Database error");
        
        Ok(format!(
            "Ai rezervare pe data de <b>{}</b> de la ora <b>{selected_hour}:00</b>",
            selected_date.format("%d.%m.%Y")
        ))
    }
}
