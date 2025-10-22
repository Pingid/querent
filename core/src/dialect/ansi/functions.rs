use super::super::DialectFunction;
use crate::schema::{DataType, FunctionType};

pub const FUNCTIONS: &[DialectFunction] = &[
    // Scalar numeric functions
    DialectFunction {
        function_name: "ABS",
        parameter_types: &[DataType::Double],
        return_type: DataType::Double,
        function_type: FunctionType::Scalar,
        description: "Returns the absolute value of a numeric expression.",
    },
    DialectFunction {
        function_name: "CEILING",
        parameter_types: &[DataType::Double],
        return_type: DataType::Double,
        function_type: FunctionType::Scalar,
        description: "Returns the smallest integer greater than or equal to a number.",
    },
    DialectFunction {
        function_name: "FLOOR",
        parameter_types: &[DataType::Double],
        return_type: DataType::Double,
        function_type: FunctionType::Scalar,
        description: "Returns the largest integer less than or equal to a number.",
    },
    DialectFunction {
        function_name: "POWER",
        parameter_types: &[DataType::Double, DataType::Double],
        return_type: DataType::Double,
        function_type: FunctionType::Scalar,
        description: "Raises a number to the power of another number.",
    },
    DialectFunction {
        function_name: "ROUND",
        parameter_types: &[DataType::Double, DataType::Integer],
        return_type: DataType::Double,
        function_type: FunctionType::Scalar,
        description: "Rounds a number to a specified number of decimal places.",
    },
    // Scalar string functions
    DialectFunction {
        function_name: "UPPER",
        parameter_types: &[DataType::Text],
        return_type: DataType::Text,
        function_type: FunctionType::Scalar,
        description: "Converts a string to uppercase.",
    },
    DialectFunction {
        function_name: "LOWER",
        parameter_types: &[DataType::Text],
        return_type: DataType::Text,
        function_type: FunctionType::Scalar,
        description: "Converts a string to lowercase.",
    },
    DialectFunction {
        function_name: "LENGTH",
        parameter_types: &[DataType::Text],
        return_type: DataType::Integer,
        function_type: FunctionType::Scalar,
        description: "Returns the number of characters in a string.",
    },
    DialectFunction {
        function_name: "CONCAT",
        parameter_types: &[DataType::Text, DataType::Text],
        return_type: DataType::Text,
        function_type: FunctionType::Scalar,
        description: "Concatenates two strings.",
    },
    DialectFunction {
        function_name: "SUBSTRING",
        parameter_types: &[DataType::Text, DataType::Integer, DataType::Integer],
        return_type: DataType::Text,
        function_type: FunctionType::Scalar,
        description: "Extracts a substring from a string.",
    },
    // Date/time functions
    DialectFunction {
        function_name: "CURRENT_DATE",
        parameter_types: &[],
        return_type: DataType::Date,
        function_type: FunctionType::Scalar,
        description: "Returns the current date.",
    },
    DialectFunction {
        function_name: "CURRENT_TIMESTAMP",
        parameter_types: &[],
        return_type: DataType::Timestamp,
        function_type: FunctionType::Scalar,
        description: "Returns the current timestamp.",
    },
    DialectFunction {
        function_name: "EXTRACT",
        parameter_types: &[DataType::Text, DataType::Timestamp],
        return_type: DataType::Integer,
        function_type: FunctionType::Scalar,
        description: "Extracts a field (e.g., YEAR, MONTH) from a date or time value.",
    },
    // Conditional / null-handling
    DialectFunction {
        function_name: "COALESCE",
        parameter_types: &[DataType::Any, DataType::Any],
        return_type: DataType::Any,
        function_type: FunctionType::Scalar,
        description: "Returns the first non-NULL value in the list.",
    },
    DialectFunction {
        function_name: "NULLIF",
        parameter_types: &[DataType::Any, DataType::Any],
        return_type: DataType::Any,
        function_type: FunctionType::Scalar,
        description: "Returns NULL if the two arguments are equal; otherwise returns the first.",
    },
    // Aggregate functions
    DialectFunction {
        function_name: "COUNT",
        parameter_types: &[DataType::Any],
        return_type: DataType::Integer,
        function_type: FunctionType::Aggregate,
        description: "Counts the number of input rows.",
    },
    DialectFunction {
        function_name: "SUM",
        parameter_types: &[DataType::Double],
        return_type: DataType::Double,
        function_type: FunctionType::Aggregate,
        description: "Returns the sum of all non-null values.",
    },
    DialectFunction {
        function_name: "AVG",
        parameter_types: &[DataType::Double],
        return_type: DataType::Double,
        function_type: FunctionType::Aggregate,
        description: "Returns the average value of a numeric expression.",
    },
    DialectFunction {
        function_name: "MIN",
        parameter_types: &[DataType::Any],
        return_type: DataType::Any,
        function_type: FunctionType::Aggregate,
        description: "Returns the smallest non-null value.",
    },
    DialectFunction {
        function_name: "MAX",
        parameter_types: &[DataType::Any],
        return_type: DataType::Any,
        function_type: FunctionType::Aggregate,
        description: "Returns the largest non-null value.",
    },
];
