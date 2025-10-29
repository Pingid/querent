use crate::complete::context::ClausePosition;
use crate::complete::context::Context;
use crate::complete::context::Location;
use crate::lex::Keyword;
use crate::lex::TokenKind;
use crate::schema;

impl<'a> Context<'a> {
    pub fn expected_data_type(&self) -> Option<schema::DataType> {
        let func = self.clause.func.as_ref()?;
        let func_name = func.name.to_string();
        let func_def = self.functions().find(|f| f.function_name() == func_name)?;
        func_def.parameter_types().get(func.arg).copied()
    }

    pub fn is_expression_left_completion_eligible(&self) -> bool {
        if !matches!(self.clause.pos, Some(ClausePosition::ExprLeft)) {
            return false;
        }
        match &self.cursor.location {
            Location::Space(inner) => matches!(
                **inner,
                Location::Comma | Location::Keyword(Keyword::Select) | Location::Paren
            ),
            // Complete qualified columns
            Location::Dot => true,
            // Completes partial qualified identifier
            Location::Ident
                if self
                    .cursor
                    .preceding_matches([TokenKind::Dot, TokenKind::Identifier])
                    || self.cursor.preceding_matches([TokenKind::LeftParen]) =>
            {
                true
            }
            // Completes function parameters
            Location::Paren => true,
            _ => false,
        }
    }
}
