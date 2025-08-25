use crate::utils::code_formatter::CodeFormatter;
use crate::utils::error::{DevToolError, DevToolResult};
use crate::utils::response::DevToolResponse;
use serde::{Deserialize, Serialize};
use sqlparser::ast::{
    ColumnDef, ColumnOption, ColumnOptionDef, CreateTable, DataType, ExactNumberInfo, ObjectName,
    Statement, TableConstraint,
};
use sqlparser::dialect::{GenericDialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect};
use sqlparser::parser::Parser;
use std::collections::HashMap;
use tauri::command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlToGoOptions {
    pub enable_pluralization: bool,
    pub exported_fields: bool,
    pub is_go_124_or_above: bool,
    pub json_null_handling: String, // "none", "omitempty", "omitzero"
    pub selected_tags: HashMap<String, bool>, // json, gorm, db, sql, etc.
}

impl Default for SqlToGoOptions {
    fn default() -> Self {
        let mut selected_tags = HashMap::new();
        selected_tags.insert("json".to_string(), true);
        selected_tags.insert("gorm".to_string(), true);
        selected_tags.insert("db".to_string(), false);
        selected_tags.insert("sql".to_string(), false);
        selected_tags.insert("yaml".to_string(), false);
        selected_tags.insert("toml".to_string(), false);
        selected_tags.insert("env".to_string(), false);
        selected_tags.insert("ini".to_string(), false);

        Self {
            enable_pluralization: true,
            exported_fields: true,
            is_go_124_or_above: true,
            json_null_handling: "omitempty".to_string(),
            selected_tags,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ColumnDefinition {
    pub name: String,
    pub sql_type: String,
    pub nullable: bool,
    pub is_primary_key: bool,
    pub is_auto_increment: bool,
    pub is_unique: bool,
    pub has_index: bool,
    pub index_names: Vec<String>,
    pub unique_index_names: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TableDefinition {
    pub name: String,
    pub columns: Vec<ColumnDefinition>,
}

#[derive(Debug, Serialize)]
pub struct GoStructOutput {
    pub outputs: HashMap<String, String>,
    pub table_names: Vec<String>,
}

pub struct SqlParser;

impl SqlParser {
    /// Parse multiple CREATE TABLE SQL statements using sqlparser and return table definitions
    pub fn parse_sql_tables(sql: &str) -> DevToolResult<Vec<TableDefinition>> {
        // Try different SQL dialects
        let dialects = vec![
            Box::new(GenericDialect {}) as Box<dyn sqlparser::dialect::Dialect>,
            Box::new(MySqlDialect {}),
            Box::new(PostgreSqlDialect {}),
            Box::new(SQLiteDialect {}),
        ];

        let mut parsed_statements = None;
        let mut last_error = None;

        // Try parsing with different dialects
        for dialect in dialects {
            match Parser::parse_sql(dialect.as_ref(), sql) {
                Ok(statements) => {
                    parsed_statements = Some(statements);
                    break;
                }
                Err(e) => {
                    last_error = Some(e);
                    continue;
                }
            }
        }

        let statements = parsed_statements.ok_or_else(|| {
            DevToolError::ParseError(
                "SQL".to_string(),
                format!(
                    "无法解析 SQL 语句: {}",
                    last_error
                        .map(|e| e.to_string())
                        .unwrap_or_else(|| "未知错误".to_string())
                ),
            )
        })?;

        let mut tables = Vec::new();

        // Process each CREATE TABLE statement
        for statement in statements {
            if let Statement::CreateTable(CreateTable {
                name,
                columns,
                constraints,
                ..
            }) = statement
            {
                let table_name = Self::extract_table_name_from_object(&name);
                let table_columns = Self::parse_table_columns(&columns, &constraints)?;

                tables.push(TableDefinition {
                    name: table_name,
                    columns: table_columns,
                });
            }
        }

        if tables.is_empty() {
            return Err(DevToolError::ParseError(
                "SQL".to_string(),
                "未找到有效的 CREATE TABLE 语句".to_string(),
            ));
        }

        Ok(tables)
    }

    /// Parse CREATE TABLE SQL using sqlparser and return table definition (single table)
    pub fn parse_create_table(sql: &str) -> DevToolResult<TableDefinition> {
        let tables = Self::parse_sql_tables(sql)?;
        tables.into_iter().next().ok_or_else(|| {
            DevToolError::ParseError(
                "SQL".to_string(),
                "未找到有效的 CREATE TABLE 语句".to_string(),
            )
        })
    }

    /// Extract table name from ObjectName
    fn extract_table_name_from_object(name: &ObjectName) -> String {
        if let Some(last_part) = name.0.last() {
            // Clean table name by removing backticks, quotes and brackets
            last_part
                .to_string()
                .replace(['`', '"', '\'', '[', ']'], "")
        } else {
            "unknown_table".to_string()
        }
    }

    /// Parse table columns and constraints
    fn parse_table_columns(
        columns: &[ColumnDef],
        constraints: &[TableConstraint],
    ) -> DevToolResult<Vec<ColumnDefinition>> {
        let mut result_columns = Vec::new();

        // Parse constraints
        let primary_keys = Self::extract_primary_keys_from_constraints(constraints);
        let unique_constraints = Self::extract_unique_constraints_from_constraints(constraints);
        let index_constraints = Self::extract_index_constraints_from_constraints(constraints);

        for column_def in columns {
            let column_name = column_def.name.value.clone();

            // Check if column is in constraints
            let is_primary_key = Self::has_primary_key_constraint(&column_def.options)
                || primary_keys.contains(&column_name);
            let is_unique = Self::has_unique_constraint(&column_def.options)
                || unique_constraints
                    .iter()
                    .any(|(_, fields)| fields.contains(&column_name));
            let has_index = is_unique
                || is_primary_key
                || index_constraints
                    .iter()
                    .any(|(_, fields)| fields.contains(&column_name));

            // Extract index names
            let mut index_names = Vec::new();
            let mut unique_index_names = Vec::new();

            for (index_name, fields) in &index_constraints {
                if fields.contains(&column_name) {
                    if let Some(name) = index_name {
                        index_names.push(name.clone());
                    }
                }
            }

            for (index_name, fields) in &unique_constraints {
                if fields.contains(&column_name) {
                    if let Some(name) = index_name {
                        unique_index_names.push(name.clone());
                    }
                }
            }

            let column = ColumnDefinition {
                name: column_name,
                sql_type: Self::data_type_to_string(&column_def.data_type),
                nullable: !Self::has_not_null_constraint(&column_def.options) && !is_primary_key,
                is_primary_key,
                is_auto_increment: Self::has_auto_increment_constraint(&column_def.options),
                is_unique,
                has_index,
                index_names,
                unique_index_names,
            };

            result_columns.push(column);
        }

        Ok(result_columns)
    }

    /// Extract primary key column names from table constraints
    fn extract_primary_keys_from_constraints(constraints: &[TableConstraint]) -> Vec<String> {
        let mut primary_keys = Vec::new();

        for constraint in constraints {
            if let TableConstraint::PrimaryKey { columns, .. } = constraint {
                for column in columns {
                    if let sqlparser::ast::Expr::Identifier(ident) = &column.column.expr {
                        primary_keys.push(ident.value.clone());
                    }
                }
            }
        }

        primary_keys
    }

    /// Extract unique constraint field names and index names
    fn extract_unique_constraints_from_constraints(
        constraints: &[TableConstraint],
    ) -> Vec<(Option<String>, Vec<String>)> {
        let mut unique_constraints = Vec::new();

        for constraint in constraints {
            if let TableConstraint::Unique { columns, name, .. } = constraint {
                let column_names: Vec<String> = columns
                    .iter()
                    .filter_map(|c| {
                        if let sqlparser::ast::Expr::Identifier(ident) = &c.column.expr {
                            Some(ident.value.clone())
                        } else {
                            None
                        }
                    })
                    .collect();
                let index_name = name.as_ref().map(|n| n.to_string());
                unique_constraints.push((index_name, column_names));
            }
        }

        unique_constraints
    }

    /// Extract index constraint field names and index names
    fn extract_index_constraints_from_constraints(
        constraints: &[TableConstraint],
    ) -> Vec<(Option<String>, Vec<String>)> {
        let mut index_constraints = Vec::new();

        // Note: Standard SQL doesn't have INDEX constraints in table definitions,
        // but some dialects do. We'll handle this based on the specific constraint types
        // that sqlparser supports.

        for constraint in constraints {
            match constraint {
                TableConstraint::Check { name, .. } => {
                    // For MySQL dialect, some CHECK constraints might represent indexes
                    if let Some(index_name) = name {
                        // This is a simplified approach - in reality, we'd need to parse
                        // the check expression to extract column names
                        index_constraints.push((Some(index_name.to_string()), vec![]));
                    }
                }
                _ => {} // Other constraint types don't represent indexes
            }
        }

        index_constraints
    }

    /// Convert DataType to string representation
    fn data_type_to_string(data_type: &DataType) -> String {
        match data_type {
            DataType::Char(size) => {
                if let Some(size_val) = size {
                    format!("CHAR({})", size_val)
                } else {
                    "CHAR".to_string()
                }
            }
            DataType::Varchar(size) => {
                if let Some(size_val) = size {
                    format!("VARCHAR({})", size_val)
                } else {
                    "VARCHAR".to_string()
                }
            }
            DataType::Text => "TEXT".to_string(),
            DataType::TinyInt(_) => "TINYINT".to_string(),
            DataType::SmallInt(_) => "SMALLINT".to_string(),
            DataType::MediumInt(_) => "MEDIUMINT".to_string(),
            DataType::Int(_) => "INT".to_string(),
            DataType::Integer(_) => "INTEGER".to_string(),
            DataType::BigInt(_) => "BIGINT".to_string(),
            DataType::Float(_) => "FLOAT".to_string(),
            DataType::Double(_) => "DOUBLE".to_string(),
            DataType::Real => "REAL".to_string(),
            DataType::Decimal(info) => match info {
                ExactNumberInfo::None => "DECIMAL".to_string(),
                ExactNumberInfo::Precision(p) => format!("DECIMAL({})", p),
                ExactNumberInfo::PrecisionAndScale(p, s) => format!("DECIMAL({},{})", p, s),
            },
            DataType::Numeric(info) => match info {
                ExactNumberInfo::None => "NUMERIC".to_string(),
                ExactNumberInfo::Precision(p) => format!("NUMERIC({})", p),
                ExactNumberInfo::PrecisionAndScale(p, s) => format!("NUMERIC({},{})", p, s),
            },
            DataType::Boolean => "BOOLEAN".to_string(),
            DataType::Date => "DATE".to_string(),
            DataType::Time(_, _) => "TIME".to_string(),
            DataType::Datetime(_) => "DATETIME".to_string(),
            DataType::Timestamp(_, _) => "TIMESTAMP".to_string(),
            DataType::Binary(_) => "BINARY".to_string(),
            DataType::Varbinary(_) => "VARBINARY".to_string(),
            DataType::Blob(_) => "BLOB".to_string(),
            DataType::JSON => "JSON".to_string(),
            DataType::Uuid => "UUID".to_string(),
            DataType::Array(_) => "ARRAY".to_string(),
            _ => "TEXT".to_string(), // Default fallback
        }
    }

    /// Check if column has NOT NULL constraint
    fn has_not_null_constraint(options: &[ColumnOptionDef]) -> bool {
        options
            .iter()
            .any(|opt| matches!(opt.option, ColumnOption::NotNull))
    }

    /// Check if column has PRIMARY KEY constraint
    fn has_primary_key_constraint(options: &[ColumnOptionDef]) -> bool {
        options.iter().any(|opt| {
            matches!(
                opt.option,
                ColumnOption::Unique {
                    is_primary: true,
                    ..
                }
            )
        })
    }

    /// Check if column has UNIQUE constraint
    fn has_unique_constraint(options: &[ColumnOptionDef]) -> bool {
        options.iter().any(|opt| {
            matches!(
                opt.option,
                ColumnOption::Unique {
                    is_primary: false,
                    ..
                }
            )
        })
    }

    /// Check if column has AUTO_INCREMENT constraint
    fn has_auto_increment_constraint(options: &[ColumnOptionDef]) -> bool {
        options
            .iter()
            .any(|opt| matches!(opt.option, ColumnOption::DialectSpecific(_)))
            || options
                .iter()
                .any(|opt| matches!(opt.option, ColumnOption::Generated { .. }))
    }
}

pub struct GoStructGenerator;

impl GoStructGenerator {
    /// Generate Go structs for multiple tables
    pub fn generate_structs(
        tables: &[TableDefinition],
        options: &SqlToGoOptions,
    ) -> DevToolResult<GoStructOutput> {
        let mut outputs = HashMap::new();
        let mut table_names = Vec::new();

        for table in tables {
            let struct_name = Self::generate_struct_name(&table.name, options.enable_pluralization);
            let go_code = Self::generate_struct(table, options)?;

            outputs.insert(struct_name.clone(), go_code);
            table_names.push(struct_name);
        }

        Ok(GoStructOutput {
            outputs,
            table_names,
        })
    }

    /// Generate Go struct from table definition
    pub fn generate_struct(
        table: &TableDefinition,
        options: &SqlToGoOptions,
    ) -> DevToolResult<String> {
        let struct_name = Self::generate_struct_name(&table.name, options.enable_pluralization);

        // Check what imports are needed
        let needs_time = table.columns.iter().any(|col| {
            Self::sql_type_to_go_type(&col.sql_type, col.nullable).contains("time.Time")
        });
        let needs_json = table.columns.iter().any(|col| {
            Self::sql_type_to_go_type(&col.sql_type, col.nullable).contains("json.RawMessage")
        });

        let mut result = String::new();

        // Add imports if needed
        if needs_time || needs_json {
            result.push_str("import (\n");
            if needs_json {
                result.push_str("\t\"encoding/json\"\n");
            }
            if needs_time {
                result.push_str("\t\"time\"\n");
            }
            result.push_str(")\n\n");
        }

        // Generate struct definition
        result.push_str(&format!("type {} struct {{\n", struct_name));

        // Generate fields
        let field_definitions = Self::generate_field_definitions(&table.columns, options)?;
        for field in &field_definitions {
            result.push_str(&format!("\t{}\n", field));
        }

        result.push_str("}\n\n");

        // Generate TableName method
        result.push_str(&format!(
            "// TableName returns the table name\n\
             func ({}) TableName() string {{\n\
             \treturn \"{}\"\n\
             }}\n\n",
            struct_name, table.name
        ));

        // Generate PK method if there are primary keys
        let primary_key_fields: Vec<&ColumnDefinition> = table
            .columns
            .iter()
            .filter(|col| col.is_primary_key)
            .collect();

        if !primary_key_fields.is_empty() {
            result.push_str("// PK returns the primary key field name(s)\n");
            if primary_key_fields.len() == 1 {
                result.push_str(&format!(
                    "func ({}) PK() string {{\n\
                     \treturn \"{}\"\n\
                     }}\n",
                    struct_name, primary_key_fields[0].name
                ));
            } else {
                let pk_names: Vec<String> = primary_key_fields
                    .iter()
                    .map(|col| format!("\"{}\"", col.name))
                    .collect();
                result.push_str(&format!(
                    "func ({}) PK() []string {{\n\
                     \treturn []string{{{}}}\n\
                     }}\n",
                    struct_name,
                    pk_names.join(", ")
                ));
            }
        }

        Ok(CodeFormatter::format_go_code(&result))
    }

    /// Generate struct name from table name
    fn generate_struct_name(table_name: &str, enable_pluralization: bool) -> String {
        let mut name = table_name.to_string();
        if enable_pluralization {
            name = Self::singularize(&name);
        }
        Self::to_camel_case(&name, true)
    }

    /// Convert plural to singular (improved implementation)
    fn singularize(word: &str) -> String {
        if word.len() <= 1 {
            return word.to_string();
        }

        let lower = word.to_lowercase();

        // Handle irregular plurals first
        match lower.as_str() {
            "children" => return "child".to_string(),
            "people" => return "person".to_string(),
            "men" => return "man".to_string(),
            "women" => return "woman".to_string(),
            "feet" => return "foot".to_string(),
            "teeth" => return "tooth".to_string(),
            "mice" => return "mouse".to_string(),
            "geese" => return "goose".to_string(),
            _ => {}
        }

        // Handle regular pluralization patterns
        if lower.ends_with("ies") && word.len() > 3 {
            // companies -> company, stories -> story
            format!("{}y", &word[..word.len() - 3])
        } else if lower.ends_with("ves") && word.len() > 3 {
            // knives -> knife, lives -> life
            format!("{}fe", &word[..word.len() - 3])
        } else if lower.ends_with("ses")
            || lower.ends_with("ches")
            || lower.ends_with("shes")
            || lower.ends_with("xes")
        {
            // boxes -> box, dishes -> dish, classes -> class, churches -> church
            word[..word.len() - 2].to_string()
        } else if lower.ends_with("oes") && word.len() > 3 {
            // heroes -> hero, potatoes -> potato
            word[..word.len() - 2].to_string()
        } else if lower.ends_with('s')
            && !lower.ends_with("ss")
            && !lower.ends_with("us")
            && !lower.ends_with("is")
        {
            // Remove trailing 's' but preserve words ending in 'ss', 'us', 'is'
            // pipelines -> pipeline, users -> user, posts -> post
            // but preserve: class -> class, status -> status, analysis -> analysis
            word[..word.len() - 1].to_string()
        } else {
            // No change needed - already singular or special case
            word.to_string()
        }
    }

    /// Convert to camelCase or PascalCase
    fn to_camel_case(s: &str, capitalize_first: bool) -> String {
        let words: Vec<&str> = s.split(&['_', '-'][..]).collect();
        let mut result = String::new();

        for (i, word) in words.iter().enumerate() {
            if word.is_empty() {
                continue;
            }
            if i == 0 && !capitalize_first {
                result.push_str(&word.to_lowercase());
            } else {
                let mut chars = word.chars();
                if let Some(first) = chars.next() {
                    result.push(first.to_uppercase().next().unwrap());
                    result.push_str(&chars.as_str().to_lowercase());
                }
            }
        }

        result
    }

    /// Generate field name from column name
    fn to_field_name(column_name: &str, exported: bool) -> String {
        let camel = Self::to_camel_case(column_name, false);
        if exported {
            Self::to_camel_case(column_name, true)
        } else {
            camel
        }
    }

    /// Convert SQL type to Go type
    fn sql_type_to_go_type(sql_type: &str, nullable: bool) -> String {
        // Normalize to lowercase for easier matching
        let sql_lower = sql_type.to_lowercase();
        
        // Check if this is an unsigned integer type
        let is_unsigned = sql_lower.contains("int unsigned") || 
                         sql_lower.contains("bigint unsigned") ||
                         sql_lower.contains("tinyint unsigned") ||
                         sql_lower.contains("smallint unsigned") ||
                         sql_lower.contains("mediumint unsigned") ||
                         sql_lower.contains(" unsigned") ||
                         sql_lower.ends_with(" unsigned");
                         
        let clean_type = sql_type
            .replace(|c: char| c == '(' || c == ')' || c.is_ascii_digit(), "")
            .to_uppercase()
            .replace(" UNSIGNED", "")
            .replace("UNSIGNED", "")
            .trim()
            .to_string();

        let base_type = match clean_type.as_str() {
            // Integer types - handle unsigned variants
            "BIT" | "TINYINT" => {
                if is_unsigned {
                    if nullable { "*uint8" } else { "uint8" }
                } else {
                    if nullable { "*int8" } else { "int8" }
                }
            }
            "SMALLINT" => {
                if is_unsigned {
                    if nullable { "*uint16" } else { "uint16" }
                } else {
                    if nullable { "*int16" } else { "int16" }
                }
            }
            "MEDIUMINT" | "INT" | "INTEGER" => {
                if is_unsigned {
                    if nullable { "*uint32" } else { "uint32" }
                } else {
                    if nullable { "*int32" } else { "int32" }
                }
            }
            "BIGINT" => {
                if is_unsigned {
                    if nullable { "*uint64" } else { "uint64" }
                } else {
                    if nullable { "*int64" } else { "int64" }
                }
            }

            // Float types
            "FLOAT" | "REAL" => {
                if nullable {
                    "*float32"
                } else {
                    "float32"
                }
            }
            "DOUBLE" | "DOUBLE PRECISION" => {
                if nullable {
                    "*float64"
                } else {
                    "float64"
                }
            }
            "DECIMAL" | "NUMERIC" | "MONEY" => {
                if nullable {
                    "*float64"
                } else {
                    "float64"
                }
            }

            // String types
            "CHAR" | "VARCHAR" | "TEXT" | "TINYTEXT" | "MEDIUMTEXT" | "LONGTEXT" | "NCHAR"
            | "NVARCHAR" | "NTEXT" | "CLOB" => {
                if nullable {
                    "*string"
                } else {
                    "string"
                }
            }

            // Date/Time types
            "DATE" | "TIME" | "DATETIME" | "DATETIME2" | "TIMESTAMP" | "TIMESTAMPTZ" => {
                if nullable {
                    "*time.Time"
                } else {
                    "time.Time"
                }
            }

            // Boolean types
            "BOOLEAN" | "BOOL" => {
                if nullable {
                    "*bool"
                } else {
                    "bool"
                }
            }

            // Binary types
            "BINARY" | "VARBINARY" | "BLOB" | "TINYBLOB" | "MEDIUMBLOB" | "LONGBLOB" | "BYTEA" => {
                "[]byte"
            }

            // JSON types
            "JSON" | "JSONB" => {
                if nullable {
                    "*json.RawMessage"
                } else {
                    "json.RawMessage"
                }
            }

            // UUID type
            "UUID" => {
                if nullable {
                    "*string"
                } else {
                    "string"
                }
            }

            // Enum type
            "ENUM" => {
                if nullable {
                    "*string"
                } else {
                    "string"
                }
            }

            // Default
            _ => {
                if nullable {
                    "*string"
                } else {
                    "string"
                }
            }
        };

        base_type.to_string()
    }

    /// Generate field definitions
    fn generate_field_definitions(
        columns: &[ColumnDefinition],
        options: &SqlToGoOptions,
    ) -> DevToolResult<Vec<String>> {
        let mut definitions = Vec::new();
        let mut max_name_len = 0;
        let mut max_type_len = 0;

        // Calculate max lengths for alignment
        for column in columns {
            let field_name = Self::to_field_name(&column.name, options.exported_fields);
            let field_type = Self::sql_type_to_go_type(&column.sql_type, column.nullable);
            max_name_len = max_name_len.max(field_name.len());
            max_type_len = max_type_len.max(field_type.len());
        }

        // Generate field definitions
        for column in columns {
            let field_name = Self::to_field_name(&column.name, options.exported_fields);
            let field_type = Self::sql_type_to_go_type(&column.sql_type, column.nullable);
            let tags = Self::generate_field_tags(column, &field_name, options);

            let name_padded = format!("{:width$}", field_name, width = max_name_len);
            let type_padded = format!("{:width$}", field_type, width = max_type_len);

            let definition = if tags.is_empty() {
                format!("{} {}", name_padded, type_padded)
            } else {
                format!("{} {} `{}`", name_padded, type_padded, tags)
            };

            definitions.push(definition);
        }

        Ok(definitions)
    }

    /// Generate field tags
    fn generate_field_tags(
        column: &ColumnDefinition,
        field_name: &str,
        options: &SqlToGoOptions,
    ) -> String {
        let mut tags = Vec::new();

        for (tag_type, &is_selected) in &options.selected_tags {
            if !is_selected {
                continue;
            }

            let tag = match tag_type.as_str() {
                "json" => Self::generate_json_tag(column, options),
                "yaml" => format!("yaml:\"{}\"", column.name),
                "gorm" => Self::generate_gorm_tag(column, field_name, options),
                "db" => format!("db:\"{}\"", column.name),
                "sql" => format!("sql:\"{}\"", column.name),
                "toml" => format!("toml:\"{}\"", column.name),
                "env" => format!("env:\"{}\"", column.name.to_uppercase()),
                "ini" => format!("ini:\"{}\"", column.name),
                _ => continue,
            };

            tags.push(tag);
        }

        tags.join(" ")
    }

    /// Generate JSON tag
    fn generate_json_tag(column: &ColumnDefinition, options: &SqlToGoOptions) -> String {
        let mut json_tag = format!("json:\"{}\"", column.name);

        if options.json_null_handling != "none" {
            if !column.is_primary_key && !column.is_auto_increment {
                match options.json_null_handling.as_str() {
                    "omitempty" => json_tag = format!("json:\"{},omitempty\"", column.name),
                    "omitzero" => {
                        if options.is_go_124_or_above
                            && Self::should_use_omitzero(&column.sql_type, column.nullable)
                        {
                            json_tag = format!("json:\"{},omitzero\"", column.name);
                        } else {
                            json_tag = format!("json:\"{},omitempty\"", column.name);
                        }
                    }
                    _ => {}
                }
            }
        }

        json_tag
    }

    /// Determine if field should use omitzero
    fn should_use_omitzero(sql_type: &str, nullable: bool) -> bool {
        if nullable {
            return false; // Pointer types should use omitempty
        }

        let clean_type = sql_type
            .replace(|c: char| c == '(' || c == ')' || c.is_ascii_digit(), "")
            .to_uppercase();

        matches!(
            clean_type.as_str(),
            "INT"
                | "INTEGER"
                | "TINYINT"
                | "SMALLINT"
                | "MEDIUMINT"
                | "BIGINT"
                | "FLOAT"
                | "DOUBLE"
                | "DECIMAL"
                | "NUMERIC"
                | "BOOLEAN"
                | "BOOL"
        )
    }

    /// Generate GORM tag
    fn generate_gorm_tag(
        column: &ColumnDefinition,
        _field_name: &str,
        _options: &SqlToGoOptions,
    ) -> String {
        let mut attrs = Vec::new();

        // Always add column tag to ensure proper database mapping
        attrs.push(format!("column:{}", column.name));

        // Primary key
        if column.is_primary_key {
            attrs.push("primaryKey".to_string());
        }

        // Auto increment
        if column.is_auto_increment {
            attrs.push("autoIncrement".to_string());
        }

        // Not null constraint
        if !column.nullable && !column.is_primary_key {
            attrs.push("not null".to_string());
        }

        // Unique constraints
        if column.is_unique && !column.is_primary_key {
            if !column.unique_index_names.is_empty() {
                for index_name in &column.unique_index_names {
                    attrs.push(format!("uniqueIndex:{}", index_name));
                }
            } else {
                attrs.push("unique".to_string());
            }
        }

        // Index constraints
        if column.has_index && !column.is_primary_key && !column.is_unique {
            if !column.index_names.is_empty() {
                for index_name in &column.index_names {
                    attrs.push(format!("index:{}", index_name));
                }
            } else {
                attrs.push("index".to_string());
            }
        }

        // Type mapping
        if let Some(gorm_type) = Self::get_gorm_type(&column.sql_type) {
            attrs.push(format!("type:{}", gorm_type));
        }

        format!("gorm:\"{}\"", attrs.join(";"))
    }

    /// Get GORM type mapping
    fn get_gorm_type(sql_type: &str) -> Option<String> {
        let clean_type = sql_type
            .replace(|c: char| c == '(' || c == ')' || c.is_ascii_digit(), "")
            .to_uppercase();

        match clean_type.as_str() {
            "CHAR" => Some("char(255)".to_string()),
            "VARCHAR" => Some("varchar(255)".to_string()),
            "TEXT" => Some("text".to_string()),
            "TINYTEXT" => Some("tinytext".to_string()),
            "MEDIUMTEXT" => Some("mediumtext".to_string()),
            "LONGTEXT" => Some("longtext".to_string()),
            "TINYINT" => Some("tinyint".to_string()),
            "SMALLINT" => Some("smallint".to_string()),
            "MEDIUMINT" => Some("mediumint".to_string()),
            "INT" | "INTEGER" => Some("int".to_string()),
            "BIGINT" => Some("bigint".to_string()),
            "FLOAT" => Some("float".to_string()),
            "DOUBLE" => Some("double".to_string()),
            "DECIMAL" | "NUMERIC" => Some("decimal(10,2)".to_string()),
            "DATE" => Some("date".to_string()),
            "TIME" => Some("time".to_string()),
            "DATETIME" => Some("datetime".to_string()),
            "TIMESTAMP" => Some("timestamp".to_string()),
            "BOOLEAN" | "BOOL" => Some("boolean".to_string()),
            "BINARY" => Some("binary(255)".to_string()),
            "VARBINARY" => Some("varbinary(255)".to_string()),
            "BLOB" => Some("blob".to_string()),
            "TINYBLOB" => Some("tinyblob".to_string()),
            "MEDIUMBLOB" => Some("mediumblob".to_string()),
            "LONGBLOB" => Some("longblob".to_string()),
            "JSON" => Some("json".to_string()),
            "JSONB" => Some("jsonb".to_string()),
            "UUID" => Some("uuid".to_string()),
            "ENUM" => Some("varchar(255)".to_string()),
            _ => None,
        }
    }
}

#[command]
pub async fn convert_sql_to_go(
    sql: String,
    options: Option<SqlToGoOptions>,
) -> Result<DevToolResponse<GoStructOutput>, String> {
    let options = options.unwrap_or_default();

    // Validate input
    if sql.trim().is_empty() {
        return Ok(DevToolResponse::error("SQL内容不能为空"));
    }

    match SqlParser::parse_sql_tables(&sql) {
        Ok(tables) => match GoStructGenerator::generate_structs(&tables, &options) {
            Ok(go_structs) => Ok(DevToolResponse::success(go_structs)),
            Err(e) => Ok(DevToolResponse::error(format!("生成Go结构体失败: {}", e))),
        },
        Err(e) => Ok(DevToolResponse::error(format!("SQL解析失败: {}", e))),
    }
}
