use crate::complete::completion::Completion;
use crate::complete::completion::CompletionBuilder;
use crate::complete::completion::CompletionKind;
use crate::complete::completion::InsertTextFormat;
use crate::complete::context::ClauseKind;
use crate::complete::context::Context;
use crate::complete::context::Location;
use crate::dialect::SpecFunction;
use crate::lex::Keyword;
use crate::lex::TokenKind;
use crate::schema;

pub fn complete(ctx: &mut Context<'_>, builder: &mut CompletionBuilder) {
    if !should_complete(ctx) {
        return;
    }

    // Collect all available functions
    let available = get_available_functions(ctx);

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
        builder.add(completion, 0);
    }
}

fn should_complete(ctx: &Context<'_>) -> bool {
    match ctx.clause {
        ClauseKind::Select => match &ctx.cursor.location {
            Location::Space(inner) => {
                matches!(
                    **inner,
                    Location::Comma | Location::Keyword(Keyword::Select)
                )
            }
            // Location::Dot => true,
            Location::Ident
                if ctx
                    .cursor
                    .preceding_matches([TokenKind::Comma, TokenKind::Identifier]) =>
            {
                true
            }
            _ => false,
        },
        _ => false,
    }
}

fn get_available_functions<'a>(ctx: &'a Context<'_>) -> Vec<AvailableFunction<'a>> {
    ctx.spec
        .functions
        .values()
        .map(|func| AvailableFunction::Spec(func))
        .chain(
            ctx.schema
                .get_functions()
                .iter()
                .map(|func| AvailableFunction::Schema(func)),
        )
        .filter(|func| match (func.function_type(), ctx.clause) {
            (schema::FunctionType::Scalar, ClauseKind::Select) => true,
            _ => false,
        })
        .collect::<Vec<_>>()
}

#[derive(Debug, Clone, PartialEq)]
enum AvailableFunction<'schema> {
    Spec(&'static SpecFunction),
    Schema(&'schema schema::Function),
}

impl<'schema> AvailableFunction<'schema> {
    fn function_type(&self) -> schema::FunctionType {
        match self {
            AvailableFunction::Spec(func) => func.function_type,
            AvailableFunction::Schema(func) => func.function_type,
        }
    }
    fn function_name(&self) -> String {
        match self {
            AvailableFunction::Spec(func) => func.function_name.to_string(),
            AvailableFunction::Schema(func) => func.function_name.to_string(),
        }
    }
    fn parameter_types(&self) -> Vec<schema::DataType> {
        match self {
            AvailableFunction::Spec(func) => func.parameter_types.to_vec(),
            AvailableFunction::Schema(func) => func.parameter_types.to_vec(),
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
                        schema::FunctionType::Scalar,
                        &[schema::DataType::Text],
                        schema::DataType::Text,
                    )
                    .build(),
            )
            .run_with(complete)
    }
}
