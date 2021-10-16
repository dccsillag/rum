use chrono::{DateTime, Local, Utc};

pub mod tail;


pub fn format_datetime(datetime: DateTime<Utc>) -> String {
    datetime.with_timezone(&Local).format("%c").to_string()
}
