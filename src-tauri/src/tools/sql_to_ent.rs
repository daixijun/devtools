use crate::utils::code_formatter::CodeFormatter;
use serde::{Deserialize, Serialize};
use sqlparser::ast::{
    ColumnDef, ColumnOption, ColumnOptionDef, CreateTable, DataType, ExactNumberInfo, Expr,
    ObjectName, Statement, TableConstraint, Value,
};
use sqlparser::dialect::{GenericDialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect};
use sqlparser::parser::Parser;
use std::collections::HashMap;
use tauri::command;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqlToEntOptions {
    pub generate_edges: bool,
    pub generate_mixin: bool,
    pub generate_hooks: bool,
    pub generate_policy: bool,
    pub use_uuid_primary_key: bool,
    pub enable_soft_delete: bool,
    pub enable_pluralization: bool,
    pub package_name: String,
}

impl Default for SqlToEntOptions {
    fn default() -> Self {
        Self {
            generate_edges: true,
            generate_mixin: true,
            generate_hooks: false,
            generate_policy: false,
            use_uuid_primary_key: false,
            enable_soft_delete: false,
            enable_pluralization: true,
            package_name: "schema".to_string(),
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
    pub default_value: Option<String>,
    pub references: Option<ForeignKeyReference>,
}

#[derive(Debug, Clone)]
pub struct ForeignKeyReference {
    pub table: String,
    pub column: String,
}

#[derive(Debug, Clone)]
pub struct TableDefinition {
    pub name: String,
    pub columns: Vec<ColumnDefinition>,
}

#[derive(Debug, Serialize)]
pub struct EntSchemaOutput {
    pub outputs: HashMap<String, String>,
    pub table_names: Vec<String>,
}

pub struct SqlToEntParser;

impl SqlToEntParser {
    /// Parse multiple CREATE TABLE statements using sqlparser
    pub fn parse_sql_tables(sql: &str) -> Result<Vec<TableDefinition>, String> {
        let mut tables = Vec::new();

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
            format!(
                "无法解析 SQL 语句: {}",
                last_error
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "未知错误".to_string())
            )
        })?;

        // Process each CREATE TABLE statement
        for statement in statements {
            if let Statement::CreateTable(CreateTable {
                name,
                columns,
                constraints,
                if_not_exists: _,
                or_replace: _,
                temporary: _,
                external: _,
                global: _,
                transient: _,
                volatile: _,
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
            return Err("未找到有效的 CREATE TABLE 语句".to_string());
        }

        Ok(tables)
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
    ) -> Result<Vec<ColumnDefinition>, String> {
        let mut result_columns = Vec::new();

        // Parse primary key constraints
        let primary_keys = Self::extract_primary_keys_from_constraints(constraints);

        // Parse foreign key constraints
        let foreign_keys = Self::extract_foreign_keys_from_constraints(constraints);

        // Parse unique constraints
        let unique_constraints = Self::extract_unique_constraints_from_constraints(constraints);

        for column_def in columns {
            let column_name = column_def.name.value.clone();

            let mut column = ColumnDefinition {
                name: column_name.clone(),
                sql_type: Self::data_type_to_string(&column_def.data_type),
                nullable: !Self::has_not_null_constraint(&column_def.options),
                is_primary_key: Self::has_primary_key_constraint(&column_def.options)
                    || primary_keys.contains(&column_name),
                is_auto_increment: Self::has_auto_increment_constraint(&column_def.options),
                is_unique: Self::has_unique_constraint(&column_def.options)
                    || unique_constraints
                        .iter()
                        .any(|uc| uc.contains(&column_name)),
                default_value: Self::extract_default_value_from_options(&column_def.options),
                references: None,
            };

            // Set foreign key reference if exists
            if let Some((ref_table, ref_column)) = foreign_keys.get(&column_name) {
                column.references = Some(ForeignKeyReference {
                    table: ref_table.clone(),
                    column: ref_column.clone(),
                });
            }

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

    /// Extract foreign key mappings from table constraints
    fn extract_foreign_keys_from_constraints(
        constraints: &[TableConstraint],
    ) -> HashMap<String, (String, String)> {
        let mut foreign_keys = HashMap::new();

        for constraint in constraints {
            if let TableConstraint::ForeignKey {
                columns,
                foreign_table,
                referred_columns,
                ..
            } = constraint
            {
                if let (Some(column), Some(ref_column)) =
                    (columns.first(), referred_columns.first())
                {
                    let table_name = Self::extract_table_name_from_object(foreign_table);
                    foreign_keys
                        .insert(column.value.clone(), (table_name, ref_column.value.clone()));
                }
            }
        }

        foreign_keys
    }

    /// Extract unique constraint column names from table constraints
    fn extract_unique_constraints_from_constraints(
        constraints: &[TableConstraint],
    ) -> Vec<Vec<String>> {
        let mut unique_constraints = Vec::new();

        for constraint in constraints {
            if let TableConstraint::Unique { columns, .. } = constraint {
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
                unique_constraints.push(column_names);
            }
        }

        unique_constraints
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
            DataType::IntUnsigned(_) => "INT".to_string(),
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

    /// Extract default value from column options
    fn extract_default_value_from_options(options: &[ColumnOptionDef]) -> Option<String> {
        for option in options {
            if let ColumnOption::Default(expr) = &option.option {
                return Some(Self::expr_to_string(expr));
            }
        }
        None
    }

    /// Convert Expression to string
    fn expr_to_string(expr: &Expr) -> String {
        match expr {
            Expr::Value(value_with_span) => match &value_with_span.value {
                Value::SingleQuotedString(s) => s.clone(),
                Value::Number(n, _) => n.clone(),
                Value::Boolean(b) => b.to_string(),
                Value::Null => "NULL".to_string(),
                _ => "".to_string(),
            },
            Expr::Function(func) => {
                if func.name.0.len() == 1
                    && func.name.0[0].to_string().to_uppercase() == "CURRENT_TIMESTAMP"
                {
                    "CURRENT_TIMESTAMP".to_string()
                } else {
                    func.name.to_string()
                }
            }
            _ => "".to_string(),
        }
    }
}

pub struct EntSchemaGenerator;

impl EntSchemaGenerator {
    /// Generate Ent schemas for multiple tables
    pub fn generate_schemas(
        tables: &[TableDefinition],
        options: &SqlToEntOptions,
    ) -> Result<EntSchemaOutput, String> {
        let mut outputs = HashMap::new();
        let mut table_names = Vec::new();

        for table in tables {
            let class_name = Self::get_class_name(&table.name, options.enable_pluralization);
            let schema = Self::generate_single_schema(table, options)?;

            outputs.insert(class_name.clone(), schema);
            table_names.push(class_name);
        }

        Ok(EntSchemaOutput {
            outputs,
            table_names,
        })
    }

    /// Generate schema for a single table
    fn generate_single_schema(
        table: &TableDefinition,
        options: &SqlToEntOptions,
    ) -> Result<String, String> {
        let class_name = Self::get_class_name(&table.name, options.enable_pluralization);

        let mut schema = String::new();

        // Package declaration
        schema.push_str(&format!("package {}\n\n", options.package_name));

        // Imports
        schema.push_str("import (\n");
        schema.push_str("\t\"entgo.io/ent\"\n");
        schema.push_str("\t\"entgo.io/ent/schema/field\"\n");

        if options.generate_edges {
            schema.push_str("\t\"entgo.io/ent/schema/edge\"\n");
        }

        if options.generate_mixin {
            schema.push_str("\t\"entgo.io/ent/schema/mixin\"\n");
        }

        // Check if we need time import
        if Self::needs_time_import(table) || options.enable_soft_delete {
            schema.push_str("\t\"time\"\n");
        }

        // Check if we need UUID import
        if options.use_uuid_primary_key {
            schema.push_str("\t\"github.com/google/uuid\"\n");
        }

        schema.push_str(")\n\n");

        // Schema struct
        schema.push_str(&format!(
            "// {} holds the schema definition for the {} entity.\n",
            class_name, class_name
        ));
        schema.push_str(&format!("type {} struct {{\n", class_name));
        schema.push_str("\tent.Schema\n");
        schema.push_str("}\n\n");

        // Mixin method
        if options.generate_mixin {
            schema.push_str(&format!(
                "// Mixin returns {} mixed-in schema.\n",
                class_name
            ));
            schema.push_str(&format!("func ({}) Mixin() []ent.Mixin {{\n", class_name));
            schema.push_str("\treturn []ent.Mixin{\n");
            if options.enable_soft_delete {
                schema.push_str("\t\t// Add soft delete mixin\n");
                schema.push_str("\t\t// SoftDeleteMixin{},\n");
            }
            schema.push_str("\t}\n");
            schema.push_str("}\n\n");
        }

        // Fields method
        schema.push_str(&format!("// Fields of the {}.\n", class_name));
        schema.push_str(&format!("func ({}) Fields() []ent.Field {{\n", class_name));
        schema.push_str("\treturn []ent.Field{\n");

        // Generate fields
        for column in &table.columns {
            if column.name == "id" && column.is_primary_key && !options.use_uuid_primary_key {
                // Skip default auto-increment ID field as Ent handles it automatically
                continue;
            } else if column.name == "id" && column.is_primary_key && options.use_uuid_primary_key {
                schema.push_str("\t\tfield.UUID(\"id\", uuid.UUID{}).Default(uuid.New),\n");
            } else {
                schema.push_str(&Self::generate_field_definition(column));
            }
        }

        schema.push_str("\t}\n");
        schema.push_str("}\n");

        // Edges method
        if options.generate_edges {
            schema.push_str(&format!("\n// Edges of the {}.\n", class_name));
            schema.push_str(&format!("func ({}) Edges() []ent.Edge {{\n", class_name));
            schema.push_str("\treturn []ent.Edge{\n");

            // Generate edges based on foreign keys
            for column in &table.columns {
                if let Some(ref fk) = column.references {
                    let edge_name = column.name.replace("_id", "");
                    let ref_table_class =
                        Self::get_class_name(&fk.table, options.enable_pluralization);

                    schema.push_str(&format!("\t\t// Reference to {}\n", ref_table_class));
                    schema.push_str(&format!(
                        "\t\tedge.From(\"{}\", {}.Type).Ref(\"{}\").Field(\"{}\").Unique(),\n",
                        edge_name, ref_table_class, table.name, column.name
                    ));
                }
            }

            schema.push_str("\t}\n");
            schema.push_str("}\n");
        }

        // Hooks method
        if options.generate_hooks {
            schema.push_str(&format!("\n// Hooks of the {}.\n", class_name));
            schema.push_str(&format!("func ({}) Hooks() []ent.Hook {{\n", class_name));
            schema.push_str("\treturn []ent.Hook{\n");
            schema.push_str("\t\t// Add hooks here\n");
            schema.push_str("\t}\n");
            schema.push_str("}\n");
        }

        // Policy method
        if options.generate_policy {
            schema.push_str(&format!("\n// Policy of the {}.\n", class_name));
            schema.push_str(&format!("func ({}) Policy() ent.Policy {{\n", class_name));
            schema.push_str("\treturn privacy.Policy{\n");
            schema.push_str("\t\t// Add privacy rules here\n");
            schema.push_str("\t}\n");
            schema.push_str("}\n");
        }

        Ok(CodeFormatter::format_go_code(&schema))
    }

    /// Generate field definition for a column
    fn generate_field_definition(column: &ColumnDefinition) -> String {
        let mut field = format!(
            "\t\tfield.{}",
            Self::sql_type_to_ent_field(&column.sql_type)
        );
        field.push_str(&format!("(\"{}\")", column.name));

        // Add constraints
        if column.nullable {
            field.push_str(".Optional()");
        }

        if column.is_unique && !column.is_primary_key {
            field.push_str(".Unique()");
        }

        // Add default values
        if let Some(ref default_val) = column.default_value {
            let base_type = column.sql_type.split('(').next().unwrap().to_uppercase();

            match base_type.as_str() {
                "BOOLEAN" | "BOOL" => {
                    if default_val.to_uppercase() == "TRUE" || default_val == "1" {
                        field.push_str(".Default(true)");
                    } else if default_val.to_uppercase() == "FALSE" || default_val == "0" {
                        field.push_str(".Default(false)");
                    }
                }
                "INT" | "INTEGER" | "BIGINT" | "SMALLINT" | "TINYINT" => {
                    if let Ok(_) = default_val.parse::<i64>() {
                        field.push_str(&format!(".Default({})", default_val));
                    }
                }
                "FLOAT" | "DOUBLE" | "DECIMAL" | "NUMERIC" => {
                    if let Ok(_) = default_val.parse::<f64>() {
                        field.push_str(&format!(".Default({})", default_val));
                    }
                }
                "TIMESTAMP" | "DATETIME" => {
                    if default_val.to_uppercase().contains("CURRENT_TIMESTAMP") {
                        field.push_str(".Default(time.Now)");
                    } else {
                        field.push_str(&format!(".Default(\"{}\")", default_val));
                    }
                }
                _ => {
                    field.push_str(&format!(".Default(\"{}\")", default_val));
                }
            }
        }

        // Add size constraints for string fields
        if let Some(size) = Self::extract_varchar_size(&column.sql_type) {
            field.push_str(&format!(".MaxLen({})", size));
        }

        // Add comment for foreign keys
        if let Some(ref fk) = column.references {
            field.push_str(&format!(" // Foreign key to {}.{}", fk.table, fk.column));
        }

        field.push_str(",\n");
        field
    }

    /// Convert SQL type to Ent field type
    fn sql_type_to_ent_field(sql_type: &str) -> &'static str {
        let base_type = sql_type
            .split('(')
            .next()
            .unwrap()
            .to_lowercase()
            .replace(" unsigned", "")
            .replace("unsigned", "")
            .trim()
            .to_uppercase();

        match base_type.as_str() {
            "INT" | "INTEGER" => "Int",
            "BIGINT" => "Int64",
            "SMALLINT" | "TINYINT" => "Int",
            "MEDIUMINT" => "Int", // Add MEDIUMINT support
            "VARCHAR" | "TEXT" | "CHAR" | "LONGTEXT" | "MEDIUMTEXT" => "String",
            "BOOLEAN" | "BOOL" => "Bool",
            "TIMESTAMP" | "DATETIME" | "DATE" => "Time",
            "DECIMAL" | "NUMERIC" => "Float64",
            "FLOAT" => "Float",
            "DOUBLE" => "Float64",
            "JSON" => "JSON",
            "BINARY" | "VARBINARY" | "BLOB" => "Bytes",
            _ => "String", // Default fallback
        }
    }

    /// Extract VARCHAR size if present
    fn extract_varchar_size(sql_type: &str) -> Option<u32> {
        // Since we now get the SQL type as a formatted string from sqlparser,
        // we can extract the size from VARCHAR(size) format
        if sql_type.to_uppercase().starts_with("VARCHAR(") && sql_type.ends_with(')') {
            let size_part = &sql_type[8..sql_type.len() - 1]; // Remove "VARCHAR(" and ")"
            size_part.parse::<u32>().ok()
        } else if sql_type.to_uppercase().starts_with("CHAR(") && sql_type.ends_with(')') {
            let size_part = &sql_type[5..sql_type.len() - 1]; // Remove "CHAR(" and ")"
            size_part.parse::<u32>().ok()
        } else {
            None
        }
    }

    /// Check if time import is needed
    fn needs_time_import(table: &TableDefinition) -> bool {
        table.columns.iter().any(|col| {
            let base_type = col.sql_type.split('(').next().unwrap().to_uppercase();
            matches!(base_type.as_str(), "TIMESTAMP" | "DATETIME" | "DATE")
                || (col.default_value.is_some()
                    && col
                        .default_value
                        .as_ref()
                        .unwrap()
                        .to_uppercase()
                        .contains("CURRENT_TIMESTAMP"))
        })
    }

    /// Get class name from table name
    fn get_class_name(table_name: &str, enable_pluralization: bool) -> String {
        let mut name = table_name.to_string();

        if enable_pluralization {
            name = Self::singularize(&name);
        }

        Self::to_pascal_case(&name)
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
        } else if lower.ends_with("ies") && word.len() > 3 {
            // This is duplicate but kept for clarity
            format!("{}y", &word[..word.len() - 3])
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

    /// Convert to PascalCase
    fn to_pascal_case(s: &str) -> String {
        s.split('_')
            .map(|word| {
                let mut chars = word.chars();
                match chars.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                }
            })
            .collect()
    }
}

#[command]
pub async fn convert_sql_to_ent(
    sql: String,
    options: Option<SqlToEntOptions>,
) -> Result<EntSchemaOutput, String> {
    let options = options.unwrap_or_default();

    // Validate input
    if sql.trim().is_empty() {
        return Err("SQL内容不能为空".to_string());
    }

    match SqlToEntParser::parse_sql_tables(&sql) {
        Ok(tables) => match EntSchemaGenerator::generate_schemas(&tables, &options) {
            Ok(schemas) => Ok(schemas),
            Err(e) => Err(format!("生成Ent Schema失败: {}", e)),
        },
        Err(e) => Err(format!("SQL解析失败: {}", e)),
    }
}
