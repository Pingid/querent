use super::super::SpecFunction;
use crate::schema::DataType;
use crate::schema::FunctionType;

pub(crate) static FUNCTIONS: phf::Map<&'static str, SpecFunction> = phf::phf_map! {
    // Scalar numeric functions
    "ABS" => SpecFunction {
        function_name: "ABS",
        parameter_types: &[DataType::Double],
        return_type: DataType::Double,
        function_type: FunctionType::Scalar,
        description: "Returns the absolute value of a numeric expression.",
    },
    "CEILING" => SpecFunction {
        function_name: "CEILING",
        parameter_types: &[DataType::Double],
        return_type: DataType::Double,
        function_type: FunctionType::Scalar,
        description: "Returns the smallest integer greater than or equal to a number.",
    },
    "FLOOR" => SpecFunction {
        function_name: "FLOOR",
        parameter_types: &[DataType::Double],
        return_type: DataType::Double,
        function_type: FunctionType::Scalar,
        description: "Returns the largest integer less than or equal to a number.",
    },
    "POWER" => SpecFunction {
        function_name: "POWER",
        parameter_types: &[DataType::Double, DataType::Double],
        return_type: DataType::Double,
        function_type: FunctionType::Scalar,
        description: "Raises a number to the power of another number.",
    },
    "ROUND" => SpecFunction {
        function_name: "ROUND",
        parameter_types: &[DataType::Double, DataType::Integer],
        return_type: DataType::Double,
        function_type: FunctionType::Scalar,
        description: "Rounds a number to a specified number of decimal places.",
    },

    // Scalar string functions
    "UPPER" => SpecFunction {
        function_name: "UPPER",
        parameter_types: &[DataType::Text],
        return_type: DataType::Text,
        function_type: FunctionType::Scalar,
        description: "Converts a string to uppercase.",
    },
    "LOWER" => SpecFunction {
        function_name: "LOWER",
        parameter_types: &[DataType::Text],
        return_type: DataType::Text,
        function_type: FunctionType::Scalar,
        description: "Converts a string to lowercase.",
    },
    "LENGTH" => SpecFunction {
        function_name: "LENGTH",
        parameter_types: &[DataType::Text],
        return_type: DataType::Integer,
        function_type: FunctionType::Scalar,
        description: "Returns the number of characters in a string.",
    },
    "CONCAT" => SpecFunction {
        function_name: "CONCAT",
        parameter_types: &[DataType::Text, DataType::Text],
        return_type: DataType::Text,
        function_type: FunctionType::Scalar,
        description: "Concatenates two strings.",
    },
    "SUBSTRING" => SpecFunction {
        function_name: "SUBSTRING",
        parameter_types: &[DataType::Text, DataType::Integer, DataType::Integer],
        return_type: DataType::Text,
        function_type: FunctionType::Scalar,
        description: "Extracts a substring from a string.",
    },
    // Date/time functions
    "CURRENT_DATE" => SpecFunction {
        function_name: "CURRENT_DATE",
        parameter_types: &[],
        return_type: DataType::Date,
        function_type: FunctionType::Scalar,
        description: "Returns the current date.",
    },
    "CURRENT_TIMESTAMP" => SpecFunction {
        function_name: "CURRENT_TIMESTAMP",
        parameter_types: &[],
        return_type: DataType::Timestamp,
        function_type: FunctionType::Scalar,
        description: "Returns the current timestamp.",
    },
    "EXTRACT" => SpecFunction {
        function_name: "EXTRACT",
        parameter_types: &[DataType::Text, DataType::Timestamp],
        return_type: DataType::Integer,
        function_type: FunctionType::Scalar,
        description: "Extracts a field (e.g., YEAR, MONTH) from a date or time value.",
    },
    // Conditional / null-handling
    "COALESCE" => SpecFunction {
        function_name: "COALESCE",
        parameter_types: &[DataType::Any, DataType::Any],
        return_type: DataType::Any,
        function_type: FunctionType::Scalar,
        description: "Returns the first non-NULL value in the list.",
    },
    "NULLIF" => SpecFunction {
        function_name: "NULLIF",
        parameter_types: &[DataType::Any, DataType::Any],
        return_type: DataType::Any,
        function_type: FunctionType::Scalar,
        description: "Returns NULL if the two arguments are equal; otherwise returns the first.",
    },
    // Aggregate functions
    "COUNT" => SpecFunction {
        function_name: "COUNT",
        parameter_types: &[DataType::Any],
        return_type: DataType::Integer,
        function_type: FunctionType::Aggregate,
        description: "Counts the number of input rows.",
    },
    "SUM" =>    SpecFunction {
        function_name: "SUM",
        parameter_types: &[DataType::Double],
        return_type: DataType::Double,
        function_type: FunctionType::Aggregate,
        description: "Returns the sum of all non-null values.",
    },
    "AVG" => SpecFunction {
        function_name: "AVG",
        parameter_types: &[DataType::Double],
        return_type: DataType::Double,
        function_type: FunctionType::Aggregate,
        description: "Returns the average value of a numeric expression.",
    },
    "MIN" => SpecFunction {
        function_name: "MIN",
        parameter_types: &[DataType::Any],
        return_type: DataType::Any,
        function_type: FunctionType::Aggregate,
        description: "Returns the smallest non-null value.",
    },
    "MAX" => SpecFunction {
        function_name: "MAX",
        parameter_types: &[DataType::Any],
        return_type: DataType::Any,
        function_type: FunctionType::Aggregate,
        description: "Returns the largest non-null value.",
    },
};
