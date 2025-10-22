pub static QUERY_TABLES: &str = r#"
SELECT
    m.name AS table_name,
    NULL AS schema_name,
    NULL AS database_name,
    CASE m.type
        WHEN 'table' THEN 'table'
        WHEN 'view' THEN 'view'
        ELSE 'table'
    END AS table_type
FROM sqlite_master m
WHERE m.type IN ('table', 'view')
    AND m.name NOT LIKE 'sqlite_%'
ORDER BY m.name
"#;

pub static QUERY_COLUMNS: &str = r#"
SELECT
    p.name AS column_name,
    m.name AS table_name,
    NULL AS schema_name,
    p.type AS data_type,
    CASE p."notnull"
        WHEN 0 THEN 1
        ELSE 0
    END AS is_nullable
FROM sqlite_master m
JOIN pragma_table_info(m.name) p
WHERE m.type IN ('table', 'view')
    AND m.name NOT LIKE 'sqlite_%'
ORDER BY m.name, p.cid
"#;
