use crate::ast;
use crate::schema;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct QualifiedIdent<'a> {
    pub database: Option<&'a str>,
    pub schema: Option<&'a str>,
    pub parent: Option<&'a str>, // table for columns, none for tables
    pub name: &'a str,
    pub kind: IdentKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentKind {
    Column,
    Table,
    Function,
}

impl<'a> QualifiedIdent<'a> {
    pub fn from_slice(kind: IdentKind, parts: &[&'a str]) -> Option<Self> {
        let mut ident = QualifiedIdent {
            kind,
            name: parts.last()?,
            database: None,
            schema: None,
            parent: None,
        };
        match kind {
            IdentKind::Column => {
                ident.parent = nth_from_end(parts, 2);
                ident.schema = nth_from_end(parts, 3);
                ident.database = nth_from_end(parts, 4);
            }
            IdentKind::Table => {
                ident.schema = nth_from_end(parts, 2);
                ident.database = nth_from_end(parts, 3);
            }
            IdentKind::Function => {
                ident.schema = nth_from_end(parts, 2);
                ident.database = nth_from_end(parts, 3);
            }
        }
        Some(ident)
    }

    pub fn from_str(kind: IdentKind, s: &'a str) -> Self {
        QualifiedIdent {
            kind,
            name: s,
            parent: None,
            schema: None,
            database: None,
        }
    }

    pub fn from_qualified_name(kind: IdentKind, text: &'a str, name: &ast::QualifiedName) -> Self {
        let parts = name
            .parts
            .items
            .iter()
            .map(|part| part.span.as_str(text))
            .collect::<Vec<_>>();
        Self::from_slice(kind, &parts).unwrap()
    }

    /// Get the column name for a column identifier
    pub fn name(&self) -> &'a str {
        self.name
    }

    pub fn table(&self) -> Option<&'a str> {
        match self.kind {
            IdentKind::Column => self.parent,
            IdentKind::Table => Some(self.name),
            IdentKind::Function => None,
        }
    }

    pub fn schema(&self) -> Option<&'a str> {
        self.schema
    }

    pub fn database(&self) -> Option<&'a str> {
        self.database
    }

    /// Returns true if this column reference has no table qualifier (for columns only)
    pub fn has_no_table(&self) -> bool {
        matches!(self.kind, IdentKind::Column) && self.parent.is_none()
    }

    /// Returns true if this column is a star (for columns only)
    pub fn is_wildcard(&self) -> bool {
        matches!(self.kind, IdentKind::Column) && self.name == "*"
    }

    /// Returns true if the table qualifier matches either the table name or the given alias (for columns only)
    pub fn matches_table_or_alias_or_unqualified(
        &self, table: &QualifiedIdent<'a>, alias: Option<&'a str>,
    ) -> bool {
        if !matches!(self.kind, IdentKind::Column) {
            return false;
        }
        // Unqualified columns match any table
        if self.has_no_table() {
            return true;
        }
        // Check if it matches the alias first
        if let Some(alias) = alias {
            if self.parent == Some(alias) {
                return true;
            }
        }
        // Otherwise check if it matches the table name
        self.matches(table)
    }

    /// Returns true if this column reference can refer to the provided alias (for columns only)
    pub fn matches_alias(&self, alias: Option<&'a str>) -> bool {
        matches!(self.kind, IdentKind::Column)
            && (self.has_no_table() || option_eq_or_unspecified(self.parent, alias))
    }

    /// Returns true if the column name matches (considering star expansion) (for columns only)
    pub fn matches_column(&self, column: &QualifiedIdent<'a>) -> bool {
        matches!(self.kind, IdentKind::Column)
            && matches!(column.kind, IdentKind::Column)
            && (self.is_wildcard() || self.name == column.name)
    }

    /// Check if two identifiers have matching qualifiers (context-aware)
    pub fn matches(&self, other: &QualifiedIdent<'a>) -> bool {
        use IdentKind::*;
        match (self.kind, other.kind) {
            (Column, Column) => {
                self.name == other.name
                    && option_eq_or_unspecified(self.parent, other.parent)
                    && option_eq_or_unspecified(self.schema, other.schema)
                    && option_eq_or_unspecified(self.database, other.database)
            }
            (Table, Table) | (Function, Function) => {
                self.schema == other.schema && self.database == other.database
            }
            (Column, Table) => {
                self.parent == Some(other.name)
                    && option_eq_or_unspecified(self.schema, other.schema)
                    && option_eq_or_unspecified(self.database, other.database)
            }
            (Table, Column) => {
                other.parent == Some(self.name)
                    && option_eq_or_unspecified(self.schema, other.schema)
                    && option_eq_or_unspecified(self.database, other.database)
            }
            _ => false,
        }
    }

    pub fn variants(&self) -> Vec<QualifiedIdent<'a>> {
        let mut out = Vec::with_capacity(4);
        let mut parts = vec![self.name];
        out.push(QualifiedIdent::from_slice(self.kind, &parts).unwrap());
        if let Some(p) = self.parent {
            parts.insert(0, p);
            out.push(QualifiedIdent::from_slice(self.kind, &parts).unwrap());
        }
        if let Some(s) = self.schema {
            parts.insert(0, s);
            out.push(QualifiedIdent::from_slice(self.kind, &parts).unwrap());
        }
        if let Some(d) = self.database {
            parts.insert(0, d);
            out.push(QualifiedIdent::from_slice(self.kind, &parts).unwrap());
        }
        out
    }

    pub fn ident_variants(&self) -> Vec<String> {
        let mut out = Vec::with_capacity(4);
        match self.kind {
            IdentKind::Column => {
                out.push(self.name.to_string());
                if let Some(p) = self.parent {
                    out.push(format!("{p}.{}", self.name));
                    if let Some(s) = self.schema {
                        out.push(format!("{s}.{p}.{}", self.name));
                        if let Some(d) = self.database {
                            out.push(format!("{d}.{s}.{p}.{}", self.name));
                        }
                    }
                }
            }
            IdentKind::Table | IdentKind::Function => {
                out.push(self.name.to_string());
                if let Some(s) = self.schema {
                    out.push(format!("{s}.{}", self.name));
                    if let Some(d) = self.database {
                        out.push(format!("{d}.{s}.{}", self.name));
                    }
                }
            }
        }
        out
    }
}

impl<'a> From<&'a schema::Column> for QualifiedIdent<'a> {
    fn from(col: &'a schema::Column) -> Self {
        QualifiedIdent {
            kind: IdentKind::Column,
            name: col.column_name.as_str(),
            parent: col.table_name.as_ref().map(|s| s.as_str()),
            schema: col.schema_name.as_ref().map(|s| s.as_str()),
            database: None,
        }
    }
}

impl<'a> From<&'a schema::Table> for QualifiedIdent<'a> {
    fn from(table: &'a schema::Table) -> Self {
        QualifiedIdent {
            kind: IdentKind::Table,
            name: table.table_name.as_str(),
            parent: None,
            schema: table.schema_name.as_ref().map(|s| s.as_str()),
            database: None,
        }
    }
}

impl<'a> From<&'a schema::Function> for QualifiedIdent<'a> {
    fn from(func: &'a schema::Function) -> Self {
        QualifiedIdent {
            kind: IdentKind::Function,
            name: func.function_name.as_str(),
            parent: None,
            schema: func.schema_name.as_ref().map(|s| s.as_str()),
            database: func.database_name.as_ref().map(|s| s.as_str()),
        }
    }
}

impl<'a> std::fmt::Display for QualifiedIdent<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut parts: Vec<&str> = Vec::with_capacity(4);
        if let Some(d) = self.database {
            parts.push(d);
        }
        if let Some(s) = self.schema {
            parts.push(s);
        }
        if let Some(p) = self.parent {
            parts.push(p);
        }
        parts.push(self.name);
        f.write_str(&parts.join("."))
    }
}

fn nth_from_end<T: Copy>(parts: &[T], n: usize) -> Option<T> {
    if parts.len() < n {
        return None;
    }
    Some(parts[parts.len() - n])
}

fn option_eq_or_unspecified<T: PartialEq>(a: Option<T>, b: Option<T>) -> bool {
    match (a, b) {
        (Some(x), Some(y)) => x == y,
        _ => true,
    }
}
