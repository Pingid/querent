use super::super::DialectFunction;
use crate::schema::{DataType, FunctionType};

pub(crate) static FUNCTIONS: phf::Map<&'static str, DialectFunction> = phf::phf_map! {
    // Scalar numeric functions
    "ABS" => DialectFunction {
        function_name: "ABS",
        parameter_types: &[DataType::Double],
        return_type: DataType::Double,
        function_type: FunctionType::Scalar,
        description: "Returns the absolute value of a numeric expression.",
    },
    "CEILING" => DialectFunction {
        function_name: "CEILING",
        parameter_types: &[DataType::Double],
        return_type: DataType::Double,
        function_type: FunctionType::Scalar,
        description: "Returns the smallest integer greater than or equal to a number.",
    },
    "FLOOR" => DialectFunction {
        function_name: "FLOOR",
        parameter_types: &[DataType::Double],
        return_type: DataType::Double,
        function_type: FunctionType::Scalar,
        description: "Returns the largest integer less than or equal to a number.",
    },
    "POWER" => DialectFunction {
        function_name: "POWER",
        parameter_types: &[DataType::Double, DataType::Double],
        return_type: DataType::Double,
        function_type: FunctionType::Scalar,
        description: "Raises a number to the power of another number.",
    },
    "ROUND" => DialectFunction {
        function_name: "ROUND",
        parameter_types: &[DataType::Double, DataType::Integer],
        return_type: DataType::Double,
        function_type: FunctionType::Scalar,
        description: "Rounds a number to a specified number of decimal places.",
    },

    // Scalar string functions
    "UPPER" => DialectFunction {
        function_name: "UPPER",
        parameter_types: &[DataType::Text],
        return_type: DataType::Text,
        function_type: FunctionType::Scalar,
        description: "Converts a string to uppercase.",
    },
    "LOWER" => DialectFunction {
        function_name: "LOWER",
        parameter_types: &[DataType::Text],
        return_type: DataType::Text,
        function_type: FunctionType::Scalar,
        description: "Converts a string to lowercase.",
    },
    "LENGTH" => DialectFunction {
        function_name: "LENGTH",
        parameter_types: &[DataType::Text],
        return_type: DataType::Integer,
        function_type: FunctionType::Scalar,
        description: "Returns the number of characters in a string.",
    },
    "CONCAT" => DialectFunction {
        function_name: "CONCAT",
        parameter_types: &[DataType::Text, DataType::Text],
        return_type: DataType::Text,
        function_type: FunctionType::Scalar,
        description: "Concatenates two strings.",
    },
    "SUBSTRING" => DialectFunction {
        function_name: "SUBSTRING",
        parameter_types: &[DataType::Text, DataType::Integer, DataType::Integer],
        return_type: DataType::Text,
        function_type: FunctionType::Scalar,
        description: "Extracts a substring from a string.",
    },
    // Date/time functions
    "CURRENT_DATE" => DialectFunction {
        function_name: "CURRENT_DATE",
        parameter_types: &[],
        return_type: DataType::Date,
        function_type: FunctionType::Scalar,
        description: "Returns the current date.",
    },
    "CURRENT_TIMESTAMP" => DialectFunction {
        function_name: "CURRENT_TIMESTAMP",
        parameter_types: &[],
        return_type: DataType::Timestamp,
        function_type: FunctionType::Scalar,
        description: "Returns the current timestamp.",
    },
    "EXTRACT" => DialectFunction {
        function_name: "EXTRACT",
        parameter_types: &[DataType::Text, DataType::Timestamp],
        return_type: DataType::Integer,
        function_type: FunctionType::Scalar,
        description: "Extracts a field (e.g., YEAR, MONTH) from a date or time value.",
    },
    // Conditional / null-handling
    "COALESCE" => DialectFunction {
        function_name: "COALESCE",
        parameter_types: &[DataType::Any, DataType::Any],
        return_type: DataType::Any,
        function_type: FunctionType::Scalar,
        description: "Returns the first non-NULL value in the list.",
    },
    "NULLIF" => DialectFunction {
        function_name: "NULLIF",
        parameter_types: &[DataType::Any, DataType::Any],
        return_type: DataType::Any,
        function_type: FunctionType::Scalar,
        description: "Returns NULL if the two arguments are equal; otherwise returns the first.",
    },
    // Aggregate functions
    "COUNT" => DialectFunction {
        function_name: "COUNT",
        parameter_types: &[DataType::Any],
        return_type: DataType::Integer,
        function_type: FunctionType::Aggregate,
        description: "Counts the number of input rows.",
    },
    "SUM" =>    DialectFunction {
        function_name: "SUM",
        parameter_types: &[DataType::Double],
        return_type: DataType::Double,
        function_type: FunctionType::Aggregate,
        description: "Returns the sum of all non-null values.",
    },
    "AVG" => DialectFunction {
        function_name: "AVG",
        parameter_types: &[DataType::Double],
        return_type: DataType::Double,
        function_type: FunctionType::Aggregate,
        description: "Returns the average value of a numeric expression.",
    },
    "MIN" => DialectFunction {
        function_name: "MIN",
        parameter_types: &[DataType::Any],
        return_type: DataType::Any,
        function_type: FunctionType::Aggregate,
        description: "Returns the smallest non-null value.",
    },
    "MAX" => DialectFunction {
        function_name: "MAX",
        parameter_types: &[DataType::Any],
        return_type: DataType::Any,
        function_type: FunctionType::Aggregate,
        description: "Returns the largest non-null value.",
    },
};
