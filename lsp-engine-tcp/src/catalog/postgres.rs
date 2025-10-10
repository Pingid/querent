use querent_core::catalog::{CatalogRead, schema};
use std::future::Future;
use std::pin::Pin;

type BoxedFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub struct PostgresConnectedCatalog {
    pool: bb8::Pool<
        bb8_postgres::PostgresConnectionManager<tokio_postgres_rustls::MakeRustlsConnect>,
    >,
}

impl PostgresConnectedCatalog {
    pub async fn new(config: tokio_postgres::Config) -> Result<Self, Box<dyn std::error::Error>> {
        // Create TLS connector with native certificates
        let mut root_store = rustls::RootCertStore::empty();
        let certs = rustls_native_certs::load_native_certs();
        for cert in certs.certs {
            root_store.add(cert).ok();
        }

        let tls_config = rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        let tls = tokio_postgres_rustls::MakeRustlsConnect::new(tls_config);

        // Create connection pool
        let manager = bb8_postgres::PostgresConnectionManager::new(config, tls);
        let pool = bb8::Pool::builder()
            .max_size(5)
            .connection_timeout(std::time::Duration::from_secs(10))
            .build(manager)
            .await?;

        // Test the connection immediately
        match tokio::time::timeout(std::time::Duration::from_secs(10), pool.get()).await {
            Ok(Ok(conn)) => drop(conn),
            Ok(Err(e)) => {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    format!("Failed to connect to database: {}", e),
                )));
            }
            Err(_) => {
                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::TimedOut,
                    "Connection timeout",
                )));
            }
        }

        Ok(Self { pool })
    }

    pub async fn from_url(url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // Remove channel_binding parameter if present (not well-supported in tokio-postgres)
        let clean_url = if url.contains("channel_binding=") {
            url.split('&')
                .filter(|s| !s.contains("channel_binding="))
                .collect::<Vec<_>>()
                .join("&")
                .replace("?&", "?")
        } else {
            url.to_string()
        };

        let config = clean_url.parse::<tokio_postgres::Config>()?;
        Self::new(config).await
    }
}

impl CatalogRead for PostgresConnectedCatalog {
    fn list_schemas(&self) -> BoxedFuture<'_, Vec<String>> {
        Box::pin(async move {
            let conn = match self.pool.get().await {
                Ok(c) => c,
                Err(_) => return vec![],
            };

            let rows = conn
                .query(
                    "SELECT schema_name FROM information_schema.schemata
                     WHERE schema_name NOT IN ('pg_catalog', 'information_schema', 'pg_toast')
                     ORDER BY schema_name",
                    &[],
                )
                .await
                .unwrap_or_default();

            rows.iter().map(|row| row.get::<_, String>(0)).collect()
        })
    }

    fn list_tables(&self, schema: &str) -> BoxedFuture<'_, Vec<String>> {
        let schema = schema.to_string();
        Box::pin(async move {
            let conn = match self.pool.get().await {
                Ok(c) => c,
                Err(_) => return vec![],
            };

            let rows = conn
                .query(
                    "SELECT table_name
                     FROM information_schema.tables
                     WHERE table_schema = $1
                     AND table_type IN ('BASE TABLE', 'VIEW', 'FOREIGN')
                     ORDER BY table_name",
                    &[&schema],
                )
                .await
                .unwrap_or_default();

            rows.iter().map(|row| row.get::<_, String>(0)).collect()
        })
    }

    fn list_columns(
        &self,
        table: &str,
        schema: &str,
    ) -> BoxedFuture<'_, Vec<schema::Column>> {
        let table = table.to_string();
        let schema = if schema.is_empty() { "public" } else { schema }.to_string();
        Box::pin(async move {
            let conn = match self.pool.get().await {
                Ok(c) => c,
                Err(_) => return vec![],
            };

            let rows = conn
                .query(
                    "SELECT
                        c.column_name,
                        c.data_type,
                        c.is_nullable,
                        c.column_default,
                        c.collation_name,
                        c.ordinal_position,
                        COALESCE(
                            (SELECT true
                             FROM information_schema.table_constraints tc
                             JOIN information_schema.key_column_usage kcu
                               ON tc.constraint_name = kcu.constraint_name
                               AND tc.table_schema = kcu.table_schema
                             WHERE tc.constraint_type = 'PRIMARY KEY'
                               AND tc.table_schema = c.table_schema
                               AND tc.table_name = c.table_name
                               AND kcu.column_name = c.column_name),
                            false
                        ) as is_pk,
                        COALESCE(c.is_generated = 'ALWAYS', false) as is_generated,
                        pgd.description
                     FROM information_schema.columns c
                     LEFT JOIN pg_catalog.pg_statio_all_tables st
                       ON c.table_schema = st.schemaname
                       AND c.table_name = st.relname
                     LEFT JOIN pg_catalog.pg_description pgd
                       ON pgd.objoid = st.relid
                       AND pgd.objsubid = c.ordinal_position
                     WHERE c.table_schema = $1 AND c.table_name = $2
                     ORDER BY c.ordinal_position",
                    &[&schema, &table],
                )
                .await
                .unwrap_or_default();

            rows.iter()
                .map(|row| {
                    let column_name: String = row.get(0);
                    let data_type_str: String = row.get(1);
                    let is_nullable: String = row.get(2);
                    let default: Option<String> = row.get(3);
                    let collation: Option<String> = row.get(4);
                    let ordinal: i32 = row.get(5);
                    let is_pk: bool = row.get(6);
                    let generated: bool = row.get(7);
                    let comment: Option<String> = row.get(8);

                    schema::Column {
                        column_name,
                        data_type: Some(parse_postgres_type(&data_type_str)),
                        nullable: is_nullable == "YES",
                        default,
                        is_pk,
                        generated,
                        collation,
                        comment,
                        ordinal: Some(ordinal as u32),
                    }
                })
                .collect()
        })
    }

    fn get_table(
        &self,
        table: &str,
        schema: &str,
    ) -> BoxedFuture<'_, Option<schema::Table>> {
        let table = table.to_string();
        let schema_str = if schema.is_empty() { "public" } else { schema }.to_string();
        Box::pin(async move {
            let conn = match self.pool.get().await {
                Ok(c) => c,
                Err(_) => return None,
            };

            // Get table info
            let table_rows = conn
                .query(
                    "SELECT table_type, obj_description((quote_ident($1) || '.' || quote_ident($2))::regclass)
                     FROM information_schema.tables
                     WHERE table_schema = $1 AND table_name = $2",
                    &[&schema_str, &table],
                )
                .await
                .ok()?;

            if table_rows.is_empty() {
                return None;
            }

            let table_type: String = table_rows[0].get(0);
            let description: Option<String> = table_rows[0].get(1);

            let kind = match table_type.as_str() {
                "VIEW" => schema::TableKind::View,
                "MATERIALIZED VIEW" => schema::TableKind::MaterializedView,
                "FOREIGN" => schema::TableKind::External,
                "SYSTEM TABLE" | "SYSTEM VIEW" => schema::TableKind::System,
                _ => schema::TableKind::Table,
            };

            // Get columns
            let columns = self.list_columns(&table, &schema_str).await;

            // Get foreign keys
            let fk_rows = conn
                .query(
                    "SELECT
                        kcu.column_name,
                        ccu.table_schema AS foreign_schema,
                        ccu.table_name AS foreign_table,
                        ccu.column_name AS foreign_column
                     FROM information_schema.table_constraints AS tc
                     JOIN information_schema.key_column_usage AS kcu
                       ON tc.constraint_name = kcu.constraint_name
                       AND tc.table_schema = kcu.table_schema
                     JOIN information_schema.constraint_column_usage AS ccu
                       ON ccu.constraint_name = tc.constraint_name
                       AND ccu.table_schema = tc.table_schema
                     WHERE tc.constraint_type = 'FOREIGN KEY'
                       AND tc.table_schema = $1
                       AND tc.table_name = $2",
                    &[&schema_str, &table],
                )
                .await
                .unwrap_or_default();

            let foreign_keys = fk_rows
                .iter()
                .map(|row| {
                    let column_name: String = row.get(0);
                    let foreign_schema: String = row.get(1);
                    let foreign_table: String = row.get(2);
                    let foreign_column: String = row.get(3);

                    schema::ForeignKey {
                        from: schema::ColumnRef {
                            table: schema::QualifiedName {
                                schema: schema_str.clone(),
                                name: table.clone(),
                            },
                            column: column_name,
                        },
                        to: schema::ColumnRef {
                            table: schema::QualifiedName {
                                schema: foreign_schema,
                                name: foreign_table,
                            },
                            column: foreign_column,
                        },
                    }
                })
                .collect();

            Some(schema::Table {
                name: table,
                kind,
                columns,
                foreign_keys,
                description,
            })
        })
    }

    fn list_functions(&self) -> BoxedFuture<'_, Vec<schema::Function>> {
        Box::pin(async move {
            let conn = match self.pool.get().await {
                Ok(c) => c,
                Err(_) => return vec![],
            };

            let rows = conn
                .query(
                    "SELECT
                        p.proname,
                        pg_catalog.pg_get_function_arguments(p.oid) as args,
                        CASE
                            WHEN p.proretset THEN 'table'
                            WHEN p.prokind = 'a' THEN 'aggregate'
                            ELSE 'scalar'
                        END as function_type,
                        pg_catalog.obj_description(p.oid, 'pg_proc') as description,
                        pg_catalog.format_type(p.prorettype, NULL) as return_type
                     FROM pg_catalog.pg_proc p
                     LEFT JOIN pg_catalog.pg_namespace n ON n.oid = p.pronamespace
                     WHERE n.nspname NOT IN ('pg_catalog', 'information_schema')
                     ORDER BY p.proname",
                    &[],
                )
                .await
                .unwrap_or_default();

            rows.iter()
                .map(|row| {
                    let name: String = row.get(0);
                    let _args: String = row.get(1); // TODO: parse arguments
                    let fn_type: String = row.get(2);
                    let description: Option<String> = row.get(3);
                    let return_type_str: String = row.get(4);

                    let function_type = match fn_type.as_str() {
                        "table" => schema::FunctionType::Table,
                        "aggregate" => schema::FunctionType::Aggregate,
                        _ => schema::FunctionType::Scalar,
                    };

                    schema::Function {
                        name,
                        parameter_types: vec![], // TODO: parse from args
                        function_type,
                        description,
                        return_type: Some(parse_postgres_type(&return_type_str)),
                    }
                })
                .collect()
        })
    }
}

fn parse_postgres_type(pg_type: &str) -> schema::SimpleType {
    let pg_type_lower = pg_type.to_lowercase();

    if pg_type_lower.starts_with("character varying") || pg_type_lower.starts_with("varchar") {
        // Extract length if present
        if let Some(start) = pg_type.find('(') {
            if let Some(end) = pg_type.find(')') {
                if let Ok(len) = pg_type[start + 1..end].parse::<u32>() {
                    return schema::SimpleType::Varchar { len: Some(len) };
                }
            }
        }
        return schema::SimpleType::Varchar { len: None };
    }

    if pg_type_lower.starts_with("numeric") || pg_type_lower.starts_with("decimal") {
        // Extract precision and scale if present
        if let Some(start) = pg_type.find('(') {
            if let Some(end) = pg_type.find(')') {
                let params: Vec<&str> = pg_type[start + 1..end].split(',').collect();
                if params.len() == 2 {
                    if let (Ok(precision), Ok(scale)) = (
                        params[0].trim().parse::<u8>(),
                        params[1].trim().parse::<u8>(),
                    ) {
                        return schema::SimpleType::Decimal { precision, scale };
                    }
                }
            }
        }
    }

    match pg_type_lower.as_str() {
        "boolean" | "bool" => schema::SimpleType::Boolean,
        "smallint" | "int2" | "integer" | "int" | "int4" => schema::SimpleType::Integer,
        "bigint" | "int8" => schema::SimpleType::BigInt,
        "real" | "float4" => schema::SimpleType::Float,
        "double precision" | "float8" => schema::SimpleType::Double,
        "text" | "character" | "char" => schema::SimpleType::Text,
        "timestamp"
        | "timestamp without time zone"
        | "timestamp with time zone"
        | "timestamptz" => schema::SimpleType::Timestamp,
        "date" => schema::SimpleType::Date,
        "time" | "time without time zone" | "time with time zone" => schema::SimpleType::Time,
        "json" | "jsonb" => schema::SimpleType::Json,
        "bytea" => schema::SimpleType::Bytes,
        "uuid" => schema::SimpleType::Uuid,
        _ => schema::SimpleType::Other(pg_type.to_string()),
    }
}
