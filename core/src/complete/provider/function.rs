use crate::complete::completion::Completion;
use crate::complete::completion::CompletionBuilder;
use crate::complete::completion::CompletionKind;
use crate::complete::completion::InsertTextFormat;
use crate::complete::context::ClauseKind;
use crate::complete::context::Context;
use crate::dialect::SpecFunction;
use crate::schema;

pub fn complete(ctx: &mut Context<'_>, builder: &mut CompletionBuilder) {
    if !ctx.is_expression_left_completion_eligible() || ctx.clause.kind != ClauseKind::Select {
        return;
    }

    // Get data type that should be ranked first
    let expected_data_type = ctx.expected_data_type();

    // Collect all available functions
    let available = ctx.functions();

    for func in available {
        let mut completion = Completion::new(
            CompletionKind::Function,
            func.label(),
            ctx.cursor.replace,
            Some(vec![]),
            Some(func.detail()),
        );
        completion.insert_text_format = InsertTextFormat::Snippet;
        completion.insert_text = func.insert_text();

        // if expected_data_type.is_some() && Some(expected_data_type.unwrap()) == func.return_type() {
        //     builder.add(completion, 10, func.return_type());
        // } else {
        //     builder.add(completion, 0, func.return_type());
        // }
    }
}

impl<'a> Context<'a> {
    pub fn functions(&self) -> impl Iterator<Item = AvailableFunction<'a>> {
        self.spec()
            .functions
            .values()
            .map(|func| AvailableFunction::Spec(func))
            .chain(
                self.schema()
                    .get_functions()
                    .iter()
                    .map(|func| AvailableFunction::Schema(func)),
            )
            .filter(|func| match (func.return_type(), self.clause.kind) {
                (schema::FunctionReturnType::Scalar(_), ClauseKind::Select) => true,
                _ => false,
            })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum AvailableFunction<'schema> {
    Spec(&'static SpecFunction),
    Schema(&'schema schema::Function),
}

impl<'schema> AvailableFunction<'schema> {
    pub fn function_name(&self) -> String {
        match self {
            AvailableFunction::Spec(func) => func.function_name.to_string(),
            AvailableFunction::Schema(func) => func.function_name.to_string(),
        }
    }
    pub fn parameter_types(&self) -> Vec<schema::DataType> {
        match self {
            AvailableFunction::Spec(func) => func.parameter_types.to_vec(),
            AvailableFunction::Schema(func) => func.parameter_types.to_vec(),
        }
    }
    fn return_type(&self) -> &schema::FunctionReturnType {
        match self {
            AvailableFunction::Spec(func) => &func.return_type,
            AvailableFunction::Schema(func) => &func.return_type,
        }
    }

    fn description(&self) -> Option<String> {
        match self {
            AvailableFunction::Spec(func) => Some(func.description.to_string()),
            AvailableFunction::Schema(func) => func.description.clone(),
        }
    }

    fn label(&self) -> String {
        format!(
            "{}({})",
            self.function_name(),
            self.parameter_types()
                .iter()
                .map(|ty| ty.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )
    }

    fn detail(&self) -> String {
        self.description().unwrap_or_else(|| self.function_name())
    }

    fn insert_text(&self) -> String {
        let postfix = match self.parameter_types().len() {
            0 => "".to_string(),
            n => ",".repeat(n - 1),
        };
        format!("{}($1{})", self.function_name(), postfix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::CompletionTest;
    use crate::test_util::CompletionTestResult;
    use crate::test_util::SchemaCacheBuilder;

    #[test]
    fn completes_at_appropriate_locations() {
        case("SELECT a ^").assert_empty();
        case("SELECT a as^").assert_empty();
        case("SELECT a as b^").assert_empty();
        case("SELECT a as b ^").assert_empty();
    }

    fn case(input: &str) -> CompletionTestResult {
        CompletionTest::from_input(input)
            .with_schema(
                SchemaCacheBuilder::new()
                    .add_function(
                        "public",
                        "foo",
                        schema::FunctionReturnType::Scalar(schema::DataType::Text),
                        &[schema::DataType::Text],
                    )
                    .build(),
            )
            .run_with(complete)
    }
}
