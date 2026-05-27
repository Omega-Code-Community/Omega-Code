use anyhow::{Context, Result};
use log::{debug, error, info};
use rusqlite::{
    Connection,
    OptionalExtension,
    Result as RusqliteResult,
    Row,
    ToSql,
};
use rust_i18n::t;

use crate::platform::get_database_path;

mod db_init;

/// Model Trait
pub trait Model: Sized + Clone {
    fn table_name() -> &'static str;

    fn create_table_sql() -> &'static str;

    fn from_row(row: &Row) -> RusqliteResult<Self>;

    fn insert_columns() -> &'static str;

    fn insert_values(&self) -> Vec<Box<dyn ToSql>>;

    fn update_set_clause(&self) -> String;

    fn update_values(&self) -> Vec<Box<dyn ToSql>>;

    fn primary_key_column() -> &'static str {
        "id"
    }

    fn primary_key_value(&self) -> Box<dyn ToSql>;
}

/// DatabaseManager
pub struct DatabaseManager {
    pub conn: Connection,
}

impl DatabaseManager {
    /// init database
    pub fn new() -> Result<Self> {
        let db_path = get_database_path()?;

        let conn = Connection::open(&db_path)
            .with_context(|| {
                format!(
                    "Failed to open database: {}",
                    db_path.display()
                )
            })?;

        info!(
    "{}",
    t!(
        "database_path",
        database_path = db_path.display().to_string()
    )
);

        db_init::init_all_tables(&conn)
            .context("Failed to initialize database")?;

        info!("{}", t!("database_initialized"));

        Ok(Self { conn })
    }

    /// health
    pub fn health_check(&self) -> Result<()> {
        info!("HEALTH CHECK starting");

        let sql = "SELECT 1";
        debug!("SQL => {}", sql);

        self.conn.query_row(sql, [], |row| {
            let val: i32 = row.get(0)?;
            Ok(val)
        })?;

        let version_sql = "SELECT sqlite_version()";
        debug!("SQL => {}", version_sql);

        let sqlite_version: String = self.conn.query_row(version_sql, [], |row| row.get(0))?;

        info!(
        "HEALTH CHECK success - SQLite version: {}",
        sqlite_version
    );

        Ok(())
    }

    /// create
    pub fn create<T>(&self, model: T) -> Result<i64>
    where
        T: Model,
    {
        let values = model.insert_values();

        let placeholders =
            vec!["?"; values.len()].join(", ");

        let sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            T::table_name(),
            T::insert_columns(),
            placeholders
        );

        debug!("SQL => {}", sql);

        let params: Vec<&dyn ToSql> =
            values.iter().map(|v| v.as_ref()).collect();

        match self.conn.execute(&sql, params.as_slice()) {
            Ok(_) => {
                let id = self.conn.last_insert_rowid();

                info!(
                    "INSERT success table={} id={}",
                    T::table_name(),
                    id
                );

                Ok(id)
            }

            Err(err) => {
                error!(
                    "INSERT failed table={} error={}",
                    T::table_name(),
                    err
                );

                Err(err.into())
            }
        }
    }

    /// read by id
    pub fn read_by_id<T>(&self, id: i32) -> Result<Option<T>>
    where
        T: Model,
    {
        let sql = format!(
            "SELECT * FROM {} WHERE {} = ?1",
            T::table_name(),
            T::primary_key_column()
        );

        debug!("SQL => {}", sql);

        match self
            .conn
            .query_row(&sql, [id], T::from_row)
            .optional()
        {
            Ok(data) => {
                info!(
                    "SELECT success table={} id={}",
                    T::table_name(),
                    id
                );

                Ok(data)
            }

            Err(err) => {
                error!(
                    "SELECT failed table={} id={} error={}",
                    T::table_name(),
                    id,
                    err
                );

                Err(err.into())
            }
        }
    }

    /// read all
    pub fn read_all<T>(&self) -> Result<Vec<T>>
    where
        T: Model,
    {
        let sql = format!(
            "SELECT * FROM {}",
            T::table_name()
        );

        debug!("SQL => {}", sql);

        let mut stmt = self.conn.prepare(&sql)?;

        let iter = stmt.query_map([], T::from_row)?;

        let mut result = vec![];

        for item in iter {
            result.push(item?);
        }

        info!(
            "SELECT ALL success table={} count={}",
            T::table_name(),
            result.len()
        );

        Ok(result)
    }

    /// pagination
    pub fn paginate<T>(
        &self,
        page: Option<usize>,
        page_size: Option<usize>,
    ) -> Result<(Vec<T>, usize)>
    where
        T: Model,
    {
        let page = page.unwrap_or(1);

        let page_size = page_size.unwrap_or(10);

        let offset = (page - 1) * page_size;

        let count_sql = format!(
            "SELECT COUNT(*) FROM {}",
            T::table_name()
        );

        debug!("SQL => {}", count_sql);

        let total: i64 =
            self.conn.query_row(&count_sql, [], |row| {
                row.get(0)
            })?;

        let total = total as usize;

        let sql = format!(
            "SELECT * FROM {} LIMIT ? OFFSET ?",
            T::table_name()
        );

        debug!("SQL => {}", sql);

        let mut stmt = self.conn.prepare(&sql)?;

        let iter = stmt.query_map(
            [
                &(page_size as i64) as &dyn ToSql,
                &(offset as i64),
            ],
            T::from_row,
        )?;

        let mut result = vec![];

        for item in iter {
            result.push(item?);
        }

        info!(
            "PAGINATE success table={} total={} page={} size={}",
            T::table_name(),
            total,
            page,
            page_size
        );

        Ok((result, total))
    }

    /// where query
    pub fn find_where<T>(
        &self,
        where_sql: &str,
        params: &[&dyn ToSql],
    ) -> Result<Vec<T>>
    where
        T: Model,
    {
        let sql = format!(
            "SELECT * FROM {} WHERE {}",
            T::table_name(),
            where_sql
        );

        debug!("SQL => {}", sql);

        let mut stmt = self.conn.prepare(&sql)?;

        let iter = stmt.query_map(
            params,
            T::from_row,
        )?;

        let mut result = vec![];

        for item in iter {
            result.push(item?);
        }

        info!(
            "WHERE QUERY success table={} count={}",
            T::table_name(),
            result.len()
        );

        Ok(result)
    }

    /// update
    pub fn update<T>(&self, model: T) -> Result<usize>
    where
        T: Model,
    {
        let mut values = model.update_values();

        values.push(model.primary_key_value());

        let sql = format!(
            "UPDATE {} SET {} WHERE {} = ?",
            T::table_name(),
            model.update_set_clause(),
            T::primary_key_column()
        );

        debug!("SQL => {}", sql);

        let params: Vec<&dyn ToSql> =
            values.iter().map(|v| v.as_ref()).collect();

        match self.conn.execute(&sql, params.as_slice()) {
            Ok(rows) => {
                info!(
                    "UPDATE success table={} affected={}",
                    T::table_name(),
                    rows
                );

                Ok(rows)
            }

            Err(err) => {
                error!(
                    "UPDATE failed table={} error={}",
                    T::table_name(),
                    err
                );

                Err(err.into())
            }
        }
    }

    /// delete
    pub fn delete<T>(&self, id: i32) -> Result<usize>
    where
        T: Model,
    {
        let sql = format!(
            "DELETE FROM {} WHERE {} = ?1",
            T::table_name(),
            T::primary_key_column()
        );

        debug!("SQL => {}", sql);

        match self.conn.execute(&sql, [id]) {
            Ok(rows) => {
                info!(
                    "DELETE success table={} affected={}",
                    T::table_name(),
                    rows
                );

                Ok(rows)
            }

            Err(err) => {
                error!(
                    "DELETE failed table={} error={}",
                    T::table_name(),
                    err
                );

                Err(err.into())
            }
        }
    }
}