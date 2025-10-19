use crate::{
    dialect::{
        CaseFold, CommentStyle, DialectSpec, DialectSpecProvider, If, Rule, RuleSet, StyleRules,
        Then,
    },
    lex::{Keyword, QuoteStyle, TokenKind},
};

// Include the generated ANSI keywords
mod keyword;
use keyword::KEYWORDS;

mod operator;
use operator::OP_TABLE;

#[derive(Debug, Clone, Copy)]
pub struct Ansi {
    pub spec: &'static DialectSpec,
}

impl Default for Ansi {
    fn default() -> Self {
        Self { spec: &SPEC }
    }
}

impl DialectSpecProvider for Ansi {
    fn get_spec(&self) -> &'static DialectSpec {
        self.spec
    }
}

/// The global ANSI dialect spec — no runtime alloc, no cloning.
pub static SPEC: DialectSpec = DialectSpec {
    name: "ansi",
    keywords: &KEYWORDS,
    operators: &OP_TABLE,
    style_rules: StyleRules {
        keywords_case_insensitive: true,
        word_ops_case_insensitive: true,
        unquoted_identifier_fold: CaseFold::Upper,
        quoted_identifiers_case_sensitive: true,
        comments: &[CommentStyle::DoubleDash, CommentStyle::SlashStar],
        quotes: &[QuoteStyle::Double],
    },
    follow_rules: &[ANSI_RULE_SETS],
};

pub static ANSI_RULE_SETS: RuleSet = RuleSet(&[
    Rule(
        If::Start,
        &[
            Then::Kw(Keyword::Select),
            Then::Kw(Keyword::Insert),
            Then::Kw(Keyword::Update),
            Then::Kw(Keyword::Delete),
            Then::Kw(Keyword::Create),
            Then::Kw(Keyword::Alter),
            Then::Kw(Keyword::Drop),
            Then::Kw(Keyword::Merge),
            Then::Kw(Keyword::With),
        ],
    ),
    // - Select -
    Rule(If::Kw(Keyword::Select), &[Then::Kw(Keyword::Distinct)]),
    Rule(If::Kw(Keyword::Select), &[Then::Kw(Keyword::All)]),
    Rule(
        If::Match(&[
            If::Kw(Keyword::Select),
            If::While(&If::Not(&If::AnyOf(&[
                If::Kw(Keyword::From),
                If::Kw(Keyword::Select),
            ]))),
            If::AnyOf(&[
                If::Kind(TokenKind::Identifier),
                If::Kind(TokenKind::RightParen),
            ]),
        ]),
        &[Then::Kw(Keyword::From)],
    ),
    // - Subqueries -
    Rule(If::Kind(TokenKind::LeftParen), &[Then::Kw(Keyword::Select)]),
    // — WITH CTE —
    Rule(If::Kw(Keyword::With), &[Then::Kw(Keyword::Recursive)]),
    // ORDER BY
    Rule(If::Kw(Keyword::Order), &[Then::Kw(Keyword::By)]),
]);

// — Statement starters —

// Rule(If::)
// (
//     &[K(Keyword::With)],
//     &[
//         &[K(Keyword::Recursive)],
//         &[K(Keyword::Select)],
//         &[K(Keyword::Insert)],
//         &[K(Keyword::Update)],
//         &[K(Keyword::Delete)],
//         &[K(Keyword::Merge)],
//     ],
// ),
// // — SELECT head —
// (
//     &[K(Keyword::Select)],
//     &[
//         &[K(Keyword::Distinct)],
//         &[K(Keyword::All)],
//         &[K(Keyword::From)],
//     ],
// ),
// (&[K(Keyword::Distinct)], &[&[K(Keyword::From)]]),
// (&[K(Keyword::All)], &[&[K(Keyword::From)]]),
// // — FROM & table refs / joins / trailing clauses —
// (
//     &[K(Keyword::From)],
//     &[
//         // common trailing clauses
//         &[K(Keyword::Where)],
//         &[K(Keyword::Join)],
//         &[K(Keyword::Group), K(Keyword::By)],
//         &[K(Keyword::Order), K(Keyword::By)],
//         &[K(Keyword::Limit)],
//         // set operations
//         &[K(Keyword::Union)],
//         &[K(Keyword::Union), K(Keyword::All)],
//         &[K(Keyword::Intersect)],
//         &[K(Keyword::Intersect), K(Keyword::All)],
//         &[K(Keyword::Except)],
//         &[K(Keyword::Except), K(Keyword::All)],
//         // join types
//         &[K(Keyword::Inner), K(Keyword::Join)],
//         &[K(Keyword::Left), K(Keyword::Join)],
//         &[K(Keyword::Left), K(Keyword::Outer), K(Keyword::Join)],
//         &[K(Keyword::Right), K(Keyword::Join)],
//         &[K(Keyword::Right), K(Keyword::Outer), K(Keyword::Join)],
//         &[K(Keyword::Full), K(Keyword::Join)],
//         &[K(Keyword::Full), K(Keyword::Outer), K(Keyword::Join)],
//         &[K(Keyword::Outer), K(Keyword::Join)],
//         &[K(Keyword::Cross), K(Keyword::Join)],
//         &[K(Keyword::Natural), K(Keyword::Join)],
//     ],
// ),
// // — JOIN families —
// (
//     &[K(Keyword::Join)],
//     &[&[K(Keyword::On)], &[K(Keyword::Using)]],
// ),
// (&[K(Keyword::Using)], &[]), // column list next (no more keywords)
// // — WHERE → typical trailing clauses —
// (
//     &[K(Keyword::Where)],
//     &[
//         &[K(Keyword::Group), K(Keyword::By)],
//         &[K(Keyword::Order), K(Keyword::By)],
//         &[K(Keyword::Limit)],
//     ],
// ),
// // — ORDER BY scaffolding —
// (
//     &[K(Keyword::Order), K(Keyword::By)],
//     &[
//         &[K(Keyword::Asc)],
//         &[K(Keyword::Desc)],
//         &[K(Keyword::Nulls), K(Keyword::First)],
//         &[K(Keyword::Nulls), K(Keyword::Last)],
//     ],
// ),
// (
//     &[K(Keyword::Asc)],
//     &[
//         &[K(Keyword::Nulls), K(Keyword::First)],
//         &[K(Keyword::Nulls), K(Keyword::Last)],
//     ],
// ),
// (
//     &[K(Keyword::Desc)],
//     &[
//         &[K(Keyword::Nulls), K(Keyword::First)],
//         &[K(Keyword::Nulls), K(Keyword::Last)],
//     ],
// ),
// // — GROUP BY tails —
// (
//     &[K(Keyword::Group), K(Keyword::By)],
//     &[
//         &[K(Keyword::Having)],
//         &[K(Keyword::Order), K(Keyword::By)],
//         &[K(Keyword::Limit)],
//     ],
// ),
// (
//     &[K(Keyword::Having)],
//     &[&[K(Keyword::Order), K(Keyword::By)], &[K(Keyword::Limit)]],
// ),
// // — OFFSET/FETCH (ANSI paging); LIMIT kept for pragmatism since your code suggests it —
// (&[K(Keyword::Limit)], &[&[K(Keyword::Offset)]]),
// (
//     &[K(Keyword::Offset)],
//     &[&[K(Keyword::Rows)], &[K(Keyword::Fetch)]],
// ),
// (
//     &[K(Keyword::Fetch)],
//     &[&[K(Keyword::Next)], &[K(Keyword::First)]],
// ),
// (
//     &[K(Keyword::Next)],
//     &[&[K(Keyword::Row)], &[K(Keyword::Rows)]],
// ),
// (
//     &[K(Keyword::First)],
//     &[&[K(Keyword::Row)], &[K(Keyword::Rows)]],
// ),
// (&[K(Keyword::Row)], &[&[K(Keyword::Only)]]),
// (&[K(Keyword::Rows)], &[&[K(Keyword::Only)]]),
// // — INSERT —
// (
//     &[K(Keyword::Insert)],
//     &[&[K(Keyword::Into)], &[K(Keyword::Default)]],
// ),
// (
//     &[K(Keyword::Into)],
//     &[&[K(Keyword::Values)], &[K(Keyword::Select)]],
// ),
// (&[K(Keyword::Default)], &[&[K(Keyword::Values)]]),
// // — UPDATE —
// (
//     &[K(Keyword::Update)],
//     &[&[K(Keyword::Set)], &[K(Keyword::Where)]],
// ),
// (&[K(Keyword::Set)], &[&[K(Keyword::Where)]]),
// // — DELETE —
// (
//     &[K(Keyword::Delete)],
//     &[&[K(Keyword::From)], &[K(Keyword::Where)]],
// ),
// // — MERGE (ANSI/Oracle style) —
// (&[K(Keyword::Merge)], &[&[K(Keyword::Into)]]),
// (&[K(Keyword::Using)], &[&[K(Keyword::On)]]),
// (&[K(Keyword::On)], &[&[K(Keyword::When)]]),
// (
//     &[K(Keyword::When)],
//     &[&[K(Keyword::Matched)], &[K(Keyword::Then)]],
// ),
// (&[K(Keyword::Matched)], &[&[K(Keyword::Then)]]),
// // — CASE expression —
// (&[K(Keyword::Case)], &[&[K(Keyword::When)]]),
// (&[K(Keyword::When)], &[&[K(Keyword::Then)]]),
// // More specific rule for CASE expressions: WHEN THEN suggests WHEN, ELSE, END
// (
//     &[K(Keyword::When), K(Keyword::Then)],
//     &[&[K(Keyword::When)], &[K(Keyword::Else)], &[K(Keyword::End)]],
// ),
// // Fallback rule for THEN in CASE context
// (
//     &[K(Keyword::Then)],
//     &[&[K(Keyword::When)], &[K(Keyword::Else)], &[K(Keyword::End)]],
// ),
// (&[K(Keyword::Else)], &[&[K(Keyword::End)]]),
// // — Set operators —
// (
//     &[K(Keyword::Union)],
//     &[&[K(Keyword::All)], &[K(Keyword::Select)]],
// ),
// (
//     &[K(Keyword::Union), K(Keyword::All)],
//     &[&[K(Keyword::Select)]],
// ),
// (
//     &[K(Keyword::Intersect)],
//     &[&[K(Keyword::All)], &[K(Keyword::Select)]],
// ),
// (
//     &[K(Keyword::Intersect), K(Keyword::All)],
//     &[&[K(Keyword::Select)]],
// ),
// (
//     &[K(Keyword::Except)],
//     &[&[K(Keyword::All)], &[K(Keyword::Select)]],
// ),
// (
//     &[K(Keyword::Except), K(Keyword::All)],
//     &[&[K(Keyword::Select)]],
// ),
// (&[K(Keyword::To)], &[&[K(Keyword::Escape)]]),
// // — DDL —
// (
//     &[K(Keyword::Create)],
//     &[
//         &[K(Keyword::Table)],
//         &[K(Keyword::View)],
//         &[K(Keyword::Schema)],
//     ],
// ),
// (&[K(Keyword::Alter)], &[&[K(Keyword::Table)]]),
// (
//     &[K(Keyword::Drop)],
//     &[
//         &[K(Keyword::Table)],
//         &[K(Keyword::View)],
//         &[K(Keyword::Schema)],
//     ],
// ),
// (&[K(Keyword::Add)], &[&[K(Keyword::Constraint)]]),
// (&[K(Keyword::Primary)], &[&[K(Keyword::Key)]]),
// (&[K(Keyword::Foreign)], &[&[K(Keyword::Key)]]),
