pub static QUERY_TABLES: &str = r#"
SELECT
    c.relname AS table_name,
    n.nspname AS schema_name,
    current_database() AS database_name,
    CASE c.relkind
        WHEN 'r' THEN 'table'
        WHEN 'v' THEN 'view'
        WHEN 'm' THEN 'view'
        ELSE 'table'
    END AS table_type
FROM pg_catalog.pg_class c
JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
WHERE c.relkind IN ('r', 'v', 'm')
    AND n.nspname NOT IN ('pg_catalog', 'information_schema', 'pg_toast')
ORDER BY n.nspname, c.relname
"#;

pub static QUERY_COLUMNS: &str = r#"
SELECT
    a.attname AS column_name,
    c.relname AS table_name,
    n.nspname AS schema_name,
    format_type(a.atttypid, a.atttypmod) AS data_type,
    NOT a.attnotnull AS is_nullable
FROM pg_catalog.pg_attribute a
JOIN pg_catalog.pg_class c ON c.oid = a.attrelid
JOIN pg_catalog.pg_namespace n ON n.oid = c.relnamespace
WHERE a.attnum > 0
    AND NOT a.attisdropped
    AND c.relkind IN ('r', 'v', 'm')
    AND n.nspname NOT IN ('pg_catalog', 'information_schema', 'pg_toast')
ORDER BY n.nspname, c.relname, a.attnum
"#;

pub static QUERY_FUNCTIONS: &str = r#"
SELECT
    p.proname AS function_name,
    n.nspname AS schema_name,
    current_database() AS database_name,
    CASE
        WHEN p.proretset THEN 'table'
        WHEN p.prokind = 'a' THEN 'aggregate'
        ELSE 'scalar'
    END AS function_type,
    pg_catalog.pg_get_function_arguments(p.oid) AS parameters,
    format_type(p.prorettype, NULL) AS return_type,
    d.description
FROM pg_catalog.pg_proc p
JOIN pg_catalog.pg_namespace n ON n.oid = p.pronamespace
LEFT JOIN pg_catalog.pg_description d ON d.objoid = p.oid
WHERE n.nspname NOT IN ('pg_catalog', 'information_schema')
ORDER BY n.nspname, p.proname
"#;
