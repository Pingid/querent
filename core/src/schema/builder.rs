use crate::schema::Cache;
use crate::schema::Column;
use crate::schema::DataType;
use crate::schema::FuncReturnType;
use crate::schema::Function;
use crate::schema::Table;
use crate::schema::TableColumn;
use crate::schema::TableType;

pub struct CacheBuilder {
    columns: Vec<Column>,
    tables: Vec<Table>,
    functions: Vec<Function>,
}

impl CacheBuilder {
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
            tables: Vec::new(),
            functions: Vec::new(),
        }
    }

    pub fn column(mut self, name: &str, data_type: DataType) -> Self {
        self.columns.push(Column {
            column_name: name.to_string(),
            table_name: None,
            schema_name: None,
            database_name: None,
            data_type,
            is_nullable: None,
        });
        self
    }

    pub fn column_in<'a>(
        mut self, name: &str, data_type: DataType, table: impl Into<Name<'a>>,
        schema: impl Into<Name<'a>>, database: impl Into<Name<'a>>,
    ) -> Self {
        self.columns.push(Column {
            column_name: name.to_string(),
            table_name: table.into().to_string(),
            schema_name: schema.into().to_string(),
            database_name: database.into().to_string(),
            data_type,
            is_nullable: None,
        });
        self
    }

    pub fn table(mut self, name: &str, table_type: Option<TableType>) -> Self {
        self.tables.push(Table {
            table_name: name.to_string(),
            table_type,
            schema_name: None,
            database_name: None,
        });
        self
    }

    pub fn table_in<'a>(
        mut self, name: &str, schema_name: impl Into<Name<'a>>, database_name: impl Into<Name<'a>>,
    ) -> Self {
        self.tables.push(Table {
            table_name: name.to_string(),
            table_type: Some(TableType::Table),
            schema_name: schema_name.into().to_string(),
            database_name: database_name.into().to_string(),
        });
        self
    }

    pub fn scalar_function(
        mut self, name: &str, params: &[DataType], return_type: DataType,
    ) -> Self {
        self.functions.push(Function {
            function_name: name.to_string(),
            parameter_types: params.to_vec(),
            return_type: FuncReturnType::Scalar(return_type),
            description: None,
            schema_name: None,
            database_name: None,
        });
        self
    }

    pub fn aggregate_function(
        mut self, name: &str, params: &[DataType], return_type: DataType,
    ) -> Self {
        self.functions.push(Function {
            function_name: name.to_string(),
            parameter_types: params.to_vec(),
            return_type: FuncReturnType::Aggregate(return_type),
            description: None,
            schema_name: None,
            database_name: None,
        });
        self
    }

    pub fn table_function(
        mut self, name: &str, params: &[DataType], return_type: Vec<(&str, DataType)>,
    ) -> Self {
        self.functions.push(Function {
            function_name: name.to_string(),
            parameter_types: params.to_vec(),
            return_type: FuncReturnType::Table(
                return_type
                    .into_iter()
                    .map(|(name, data_type)| TableColumn {
                        column_name: name.to_string(),
                        data_type,
                    })
                    .collect(),
            ),
            description: None,
            schema_name: None,
            database_name: None,
        });
        self
    }

    pub fn build(self) -> Cache {
        Cache {
            columns: self.columns,
            tables: self.tables,
            functions: self.functions,
        }
    }
}

pub struct Name<'a>(Option<&'a str>);
impl<'a> Name<'a> {
    pub fn to_string(&self) -> Option<String> {
        self.0.map(|s| s.to_string())
    }
}

impl<'a> From<&'a str> for Name<'a> {
    fn from(s: &'a str) -> Self {
        Name(Some(s))
    }
}

impl<'a> From<Option<&'a str>> for Name<'a> {
    fn from(s: Option<&'a str>) -> Self {
        Name(s)
    }
}
