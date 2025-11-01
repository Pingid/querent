use schema::DataType::*;

use crate::schema;

pub fn users_schema() -> schema::Cache {
    schema::CacheBuilder::new()
        .table_in("users", "public", None)
        .column_in("id", Integer, "users", "public", None)
        .column_in("name", Text, "users", "public", None)
        .column_in("email", Text, "users", "public", None)
        .build()
}

pub fn posts_schema() -> schema::Cache {
    schema::CacheBuilder::new()
        .table_in("posts", "public", None)
        .column_in("id", Integer, "posts", "public", None)
        .column_in("title", Text, "posts", "public", None)
        .column_in("content", Text, "posts", "public", None)
        .build()
}

pub fn comments_schema() -> schema::Cache {
    schema::CacheBuilder::new()
        .table_in("comments", "public", None)
        .column_in("id", Integer, "comments", "public", None)
        .column_in("body", Text, "comments", "public", None)
        .column_in("user_id", Integer, "comments", "public", None)
        .column_in("post_id", Integer, "comments", "public", None)
        .build()
}

pub fn funcs_schema() -> schema::Cache {
    schema::CacheBuilder::new()
        .scalar_function("upper", &[Text], Text)
        .scalar_function("concat", &[Text, Text], Text)
        .scalar_function("substr", &[Text, Integer, Integer], Text)
        .scalar_function("replace", &[Text, Text, Text], Text)
        .aggregate_function("count", &[Any], Integer)
        .table_function("generate_series", &[Integer, Integer], vec![("i", Integer)])
        .table_function("unnest", &[Any], vec![("x", Any)])
        .build()
}
