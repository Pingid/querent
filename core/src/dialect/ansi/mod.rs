use crate::{
    dialect::{CaseFold, CaseRules, CommentStyle, Dialect, DialectSpec, FollowWord},
    token::{Keyword, QuoteStyle},
};

// Include the generated ANSI keywords
mod keyword;
use keyword::KEYWORDS;

mod operator;
use operator::OP_TABLE;

#[derive(Debug, Clone, Copy, Default)]
pub struct Ansi;

impl Dialect for Ansi {
    fn get_spec(&self) -> &DialectSpec {
        &SPEC
    }
}

// Helper constant for concise FollowWord::Keyword wrapping
use FollowWord::Keyword as K;

/// The global ANSI dialect spec — no runtime alloc, no cloning.
pub static SPEC: DialectSpec = DialectSpec {
    keywords: &KEYWORDS,
    operators: &OP_TABLE,
    quote_styles: &[QuoteStyle::Double],
    case_rules: CaseRules {
        keywords_case_insensitive: true,
        word_ops_case_insensitive: true,
        unquoted_identifier_fold: CaseFold::Upper,
        quoted_identifiers_case_sensitive: true,
    },
    comment_styles: &[CommentStyle::DoubleDash, CommentStyle::SlashStar],
    follow_keywords: &[
        // — Statement starters —
        (
            &[],
            &[
                &[K(Keyword::Select)],
                &[K(Keyword::Insert)],
                &[K(Keyword::Update)],
                &[K(Keyword::Delete)],
                &[K(Keyword::Create)],
                &[K(Keyword::Alter)],
                &[K(Keyword::Drop)],
                &[K(Keyword::Merge)],
                &[K(Keyword::With)],
            ],
        ),
        // — WITH CTE —
        (
            &[K(Keyword::With)],
            &[
                &[K(Keyword::Recursive)],
                &[K(Keyword::Select)],
                &[K(Keyword::Insert)],
                &[K(Keyword::Update)],
                &[K(Keyword::Delete)],
                &[K(Keyword::Merge)],
            ],
        ),
        // — SELECT head —
        (
            &[K(Keyword::Select)],
            &[
                &[K(Keyword::Distinct)],
                &[K(Keyword::All)],
                &[K(Keyword::From)],
            ],
        ),
        (&[K(Keyword::Distinct)], &[&[K(Keyword::From)]]),
        (&[K(Keyword::All)], &[&[K(Keyword::From)]]),
        // — FROM & table refs / joins / trailing clauses —
        (
            &[K(Keyword::From)],
            &[
                // common trailing clauses
                &[K(Keyword::Where)],
                &[K(Keyword::Join)],
                &[K(Keyword::Group), K(Keyword::By)],
                &[K(Keyword::Order), K(Keyword::By)],
                &[K(Keyword::Limit)],
                // set operations
                &[K(Keyword::Union)],
                &[K(Keyword::Union), K(Keyword::All)],
                &[K(Keyword::Intersect)],
                &[K(Keyword::Intersect), K(Keyword::All)],
                &[K(Keyword::Except)],
                &[K(Keyword::Except), K(Keyword::All)],
                // join types
                &[K(Keyword::Inner), K(Keyword::Join)],
                &[K(Keyword::Left), K(Keyword::Join)],
                &[K(Keyword::Left), K(Keyword::Outer), K(Keyword::Join)],
                &[K(Keyword::Right), K(Keyword::Join)],
                &[K(Keyword::Right), K(Keyword::Outer), K(Keyword::Join)],
                &[K(Keyword::Full), K(Keyword::Join)],
                &[K(Keyword::Full), K(Keyword::Outer), K(Keyword::Join)],
                &[K(Keyword::Outer), K(Keyword::Join)],
                &[K(Keyword::Cross), K(Keyword::Join)],
                &[K(Keyword::Natural), K(Keyword::Join)],
            ],
        ),
        // — JOIN families —
        (
            &[K(Keyword::Join)],
            &[&[K(Keyword::On)], &[K(Keyword::Using)]],
        ),
        (&[K(Keyword::Using)], &[]), // column list next (no more keywords)
        // — WHERE → typical trailing clauses —
        (
            &[K(Keyword::Where)],
            &[
                &[K(Keyword::Group), K(Keyword::By)],
                &[K(Keyword::Order), K(Keyword::By)],
                &[K(Keyword::Limit)],
            ],
        ),
        // — ORDER BY scaffolding —
        (
            &[K(Keyword::Order), K(Keyword::By)],
            &[
                &[K(Keyword::Asc)],
                &[K(Keyword::Desc)],
                &[K(Keyword::Nulls), K(Keyword::First)],
                &[K(Keyword::Nulls), K(Keyword::Last)],
            ],
        ),
        (
            &[K(Keyword::Asc)],
            &[
                &[K(Keyword::Nulls), K(Keyword::First)],
                &[K(Keyword::Nulls), K(Keyword::Last)],
            ],
        ),
        (
            &[K(Keyword::Desc)],
            &[
                &[K(Keyword::Nulls), K(Keyword::First)],
                &[K(Keyword::Nulls), K(Keyword::Last)],
            ],
        ),
        // — GROUP BY tails —
        (
            &[K(Keyword::Group), K(Keyword::By)],
            &[
                &[K(Keyword::Having)],
                &[K(Keyword::Order), K(Keyword::By)],
                &[K(Keyword::Limit)],
            ],
        ),
        (
            &[K(Keyword::Having)],
            &[&[K(Keyword::Order), K(Keyword::By)], &[K(Keyword::Limit)]],
        ),
        // — OFFSET/FETCH (ANSI paging); LIMIT kept for pragmatism since your code suggests it —
        (&[K(Keyword::Limit)], &[&[K(Keyword::Offset)]]),
        (
            &[K(Keyword::Offset)],
            &[&[K(Keyword::Rows)], &[K(Keyword::Fetch)]],
        ),
        (
            &[K(Keyword::Fetch)],
            &[&[K(Keyword::Next)], &[K(Keyword::First)]],
        ),
        (
            &[K(Keyword::Next)],
            &[&[K(Keyword::Row)], &[K(Keyword::Rows)]],
        ),
        (
            &[K(Keyword::First)],
            &[&[K(Keyword::Row)], &[K(Keyword::Rows)]],
        ),
        (&[K(Keyword::Row)], &[&[K(Keyword::Only)]]),
        (&[K(Keyword::Rows)], &[&[K(Keyword::Only)]]),
        // — INSERT —
        (
            &[K(Keyword::Insert)],
            &[&[K(Keyword::Into)], &[K(Keyword::Default)]],
        ),
        (
            &[K(Keyword::Into)],
            &[&[K(Keyword::Values)], &[K(Keyword::Select)]],
        ),
        (&[K(Keyword::Default)], &[&[K(Keyword::Values)]]),
        // — UPDATE —
        (
            &[K(Keyword::Update)],
            &[&[K(Keyword::Set)], &[K(Keyword::Where)]],
        ),
        (&[K(Keyword::Set)], &[&[K(Keyword::Where)]]),
        // — DELETE —
        (
            &[K(Keyword::Delete)],
            &[&[K(Keyword::From)], &[K(Keyword::Where)]],
        ),
        // — MERGE (ANSI/Oracle style) —
        (&[K(Keyword::Merge)], &[&[K(Keyword::Into)]]),
        (&[K(Keyword::Using)], &[&[K(Keyword::On)]]),
        (&[K(Keyword::On)], &[&[K(Keyword::When)]]),
        (
            &[K(Keyword::When)],
            &[&[K(Keyword::Matched)], &[K(Keyword::Then)]],
        ),
        (&[K(Keyword::Matched)], &[&[K(Keyword::Then)]]),
        // — CASE expression —
        (&[K(Keyword::Case)], &[&[K(Keyword::When)]]),
        (&[K(Keyword::When)], &[&[K(Keyword::Then)]]),
        // More specific rule for CASE expressions: WHEN THEN suggests WHEN, ELSE, END
        (&[K(Keyword::When), K(Keyword::Then)], &[&[K(Keyword::When)], &[K(Keyword::Else)], &[K(Keyword::End)]]),
        // Fallback rule for THEN in CASE context
        (&[K(Keyword::Then)], &[&[K(Keyword::When)], &[K(Keyword::Else)], &[K(Keyword::End)]]),
        (&[K(Keyword::Else)], &[&[K(Keyword::End)]]),
        // — Set operators —
        (
            &[K(Keyword::Union)],
            &[&[K(Keyword::All)], &[K(Keyword::Select)]],
        ),
        (
            &[K(Keyword::Union), K(Keyword::All)],
            &[&[K(Keyword::Select)]],
        ),
        (
            &[K(Keyword::Intersect)],
            &[&[K(Keyword::All)], &[K(Keyword::Select)]],
        ),
        (
            &[K(Keyword::Intersect), K(Keyword::All)],
            &[&[K(Keyword::Select)]],
        ),
        (
            &[K(Keyword::Except)],
            &[&[K(Keyword::All)], &[K(Keyword::Select)]],
        ),
        (
            &[K(Keyword::Except), K(Keyword::All)],
            &[&[K(Keyword::Select)]],
        ),
        (&[K(Keyword::To)], &[&[K(Keyword::Escape)]]),
        // — DDL —
        (
            &[K(Keyword::Create)],
            &[
                &[K(Keyword::Table)],
                &[K(Keyword::View)],
                &[K(Keyword::Schema)],
            ],
        ),
        (&[K(Keyword::Alter)], &[&[K(Keyword::Table)]]),
        (
            &[K(Keyword::Drop)],
            &[
                &[K(Keyword::Table)],
                &[K(Keyword::View)],
                &[K(Keyword::Schema)],
            ],
        ),
        (&[K(Keyword::Add)], &[&[K(Keyword::Constraint)]]),
        (&[K(Keyword::Primary)], &[&[K(Keyword::Key)]]),
        (&[K(Keyword::Foreign)], &[&[K(Keyword::Key)]]),
    ],
};
