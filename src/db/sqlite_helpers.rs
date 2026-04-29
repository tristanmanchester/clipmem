use std::str::FromStr;

use anyhow::{Context, Result};
use rusqlite::Row;

pub(in crate::db) fn collect_rows<T, I>(rows: I) -> Result<Vec<T>>
where
    I: Iterator<Item = rusqlite::Result<T>>,
{
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

pub(in crate::db) fn usize_to_i64(value: usize) -> Result<i64> {
    Context::context(i64::try_from(value), "value exceeds SQLite INTEGER range")
}

pub(in crate::db) fn row_usize(row: &Row<'_>, index: usize) -> rusqlite::Result<usize> {
    let value = row.get::<_, i64>(index)?;
    usize::try_from(value).map_err(|source| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Integer,
            Box::new(source),
        )
    })
}

pub(in crate::db) fn row_u64(row: &Row<'_>, index: usize) -> rusqlite::Result<u64> {
    let value = row.get::<_, i64>(index)?;
    u64::try_from(value).map_err(|source| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Integer,
            Box::new(source),
        )
    })
}

pub(in crate::db) fn row_enum<T>(row: &Row<'_>, index: usize) -> rusqlite::Result<T>
where
    T: FromStr,
    T::Err: std::error::Error + Send + Sync + 'static,
{
    let value = row.get::<_, String>(index)?;
    value.parse().map_err(|source| {
        rusqlite::Error::FromSqlConversionFailure(
            index,
            rusqlite::types::Type::Text,
            Box::new(source),
        )
    })
}
