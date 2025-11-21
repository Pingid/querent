use crate::dialect::rule::Con;
use crate::dialect::rule::Kw;
use crate::dialect::rule::Next;
use crate::dialect::rule::Op;
use crate::dialect::rule::Rule;
use crate::dialect::rule::Rules;
use crate::dialect::rule::Tk;
use crate::dialect::rule::kw;
use crate::dialect::rule::op;
use crate::dialect::rule::tk;

pub const RULES: Rules = Rules(&[
    START_RULE,
    SELECT_RULE,
    SELECT_AS_RULE,
    FROM_RULE,
    WHERE_RULE,
    ORDER_BY_DIR_RULE,
    ORDER_BY_DIR_AFTER_COMMA_RULE,
    ORDER_BY_NULLS_PLACEMENT_RULE,
    ORDER_BY_FOLLOW_RULE,
    ORDER_BY_RULE,
    ORDER_BY_SUGGEST_RULE,
    GROUP_BY_RULE,
    GROUP_BY_SUGGEST_RULE,
    LIMIT_RULE,
    SUBQUERY_RULE,
    WITH_CTE_RECURSIVE_RULE,
    WITH_CTE_AS_RULE,
    JOIN_RULE,
    JOIN_AFTER_FROM_RULE,
    INNER_JOIN_RULE,
    LEFT_JOIN_RULE,
    RIGHT_JOIN_RULE,
    FULL_JOIN_RULE,
    CROSS_JOIN_RULE,
    LEFT_OUTER_JOIN_RULE,
    RIGHT_OUTER_JOIN_RULE,
    FULL_OUTER_JOIN_RULE,
    NATURAL_JOIN_RULE,
    HAVING_RULE,
    HAVING_AFTER_GROUP_BY_RULE,
    INSERT_INTO_RULE,
    CREATE_TABLE_RULE,
    PRIMARY_KEY_RULE,
    FOREIGN_KEY_RULE,
    CASE_WHEN_RULE,
    WHEN_THEN_RULE,
    THEN_FOLLOW_RULE,
    ELSE_END_RULE,
    SET_OPERATION_ALL_RULE,
    SET_OP_RULE,
]);

const START_RULE: Rule = Rule(
    Con::or(&[
        Con::seq(&[END_CONDITION, tk(Tk::Identifier)]),
        END_CONDITION,
    ]),
    &[
        Next::Kw(Kw::Select),
        Next::Kw(Kw::Insert),
        Next::Kw(Kw::Update),
        Next::Kw(Kw::Delete),
        Next::Kw(Kw::Create),
        Next::Kw(Kw::Alter),
        Next::Kw(Kw::Drop),
        Next::Kw(Kw::Merge),
        Next::Kw(Kw::With),
    ],
);

const SELECT_RULE: Rule = Rule(kw(Kw::Select), &[Next::Kw(Kw::Distinct), Next::Kw(Kw::All)]);

const SELECT_AS_RULE: Rule = Rule(
    Con::seq(&[
        kw(Kw::Select),
        Con::many(&Con::not(&kw(Kw::From))),
        SIMPLE_VALUE,
    ]),
    &[Next::Kw(Kw::As)],
);

const FROM_RULE: Rule = Rule(
    Con::or(&[
        Con::seq(&[
            Con::or(&[kw(Kw::Select), tk(Tk::Identifier), tk(Tk::Comma)]),
            Con::or(&[SIMPLE_VALUE, op(Op::Mul)]),
        ]),
        Con::seq(&[
            Con::peek(&kw(Kw::Select)),
            Con::many(&Con::not(&kw(Kw::From))),
            SIMPLE_VALUE,
        ]),
    ]),
    &[Next::Kw(Kw::From)],
);

const WHERE_RULE: Rule = Rule(
    Con::seq(&[
        kw(Kw::From),
        Con::many(&Con::not(&Con::or(&[kw(Kw::From), kw(Kw::Where)]))),
        SIMPLE_VALUE,
    ]),
    &[Next::Kw(Kw::Where)],
);

const ORDER_BY_RULE: Rule = Rule(kw(Kw::Order), &[Next::Kw(Kw::By)]);

const ORDER_BY_SUGGEST_RULE: Rule = Rule(
    Con::seq(&[
        Con::not(&Con::or(&[
            kw(Kw::Select),
            kw(Kw::Where),
            kw(Kw::Group),
            kw(Kw::Order),
            kw(Kw::By),
            kw(Kw::Having),
            kw(Kw::With),
            op(Op::And),
            op(Op::Or),
            tk(Tk::Dot),
        ])),
        SIMPLE_VALUE,
    ]),
    &[Next::KwSeq(&[Kw::Order, Kw::By])],
);

const ORDER_BY_DIR_RULE: Rule = Rule(
    Con::seq(&[
        kw(Kw::Order),
        kw(Kw::By),
        Con::many(&Con::not(&Con::or(&[
            kw(Kw::By),
            tk(Tk::Comma),
            kw(Kw::Asc),
            kw(Kw::Desc),
            kw(Kw::Nulls),
            kw(Kw::Offset),
            kw(Kw::Fetch),
            kw(Kw::Limit),
        ]))),
        SIMPLE_VALUE,
    ]),
    &[Next::Kw(Kw::Asc), Next::Kw(Kw::Desc), Next::Kw(Kw::Nulls)],
);

const ORDER_BY_NULLS_PLACEMENT_RULE: Rule =
    Rule(kw(Kw::Nulls), &[Next::Kw(Kw::First), Next::Kw(Kw::Last)]);

// Suggest ASC/DESC/NULLS after a column following a comma in ORDER BY
const ORDER_BY_DIR_AFTER_COMMA_RULE: Rule = Rule(
    Con::seq(&[
        kw(Kw::Order),
        kw(Kw::By),
        Con::many(&Con::not(&Con::or(&[
            kw(Kw::By),
            kw(Kw::Offset),
            kw(Kw::Fetch),
            kw(Kw::Limit),
            kw(Kw::Union),
            kw(Kw::Intersect),
            kw(Kw::Except),
        ]))),
        tk(Tk::Comma),
        SIMPLE_VALUE,
    ]),
    &[Next::Kw(Kw::Asc), Next::Kw(Kw::Desc), Next::Kw(Kw::Nulls)],
);

const ORDER_BY_FOLLOW_RULE: Rule = Rule(
    Con::seq(&[
        kw(Kw::Order),
        kw(Kw::By),
        Con::many(&Con::not(&Con::or(&[
            kw(Kw::By),
            kw(Kw::Offset),
            kw(Kw::Fetch),
            kw(Kw::Limit),
            kw(Kw::Union),
            kw(Kw::Intersect),
            kw(Kw::Except),
        ]))),
        SIMPLE_VALUE,
    ]),
    &[Next::Kw(Kw::Offset), Next::Kw(Kw::Fetch)],
);

const LIMIT_RULE: Rule = Rule(
    Con::seq(&[
        Con::not(&Con::or(&[
            kw(Kw::Where),
            kw(Kw::Group),
            kw(Kw::Order),
            kw(Kw::By),
            kw(Kw::Having),
            kw(Kw::With),
            op(Op::And),
            op(Op::Or),
            tk(Tk::Dot),
        ])),
        SIMPLE_VALUE,
    ]),
    &[Next::Kw(Kw::Limit)],
);

const SUBQUERY_RULE: Rule = Rule(tk(Tk::LeftParen), &[Next::Kw(Kw::Select)]);

const WITH_CTE_RECURSIVE_RULE: Rule = Rule(kw(Kw::With), &[Next::Kw(Kw::Recursive)]);
const WITH_CTE_AS_RULE: Rule = Rule(
    Con::seq(&[kw(Kw::With), tk(Tk::Identifier)]),
    &[Next::Seq(&[Next::Kw(Kw::As), Next::Query])],
);

const GROUP_BY_RULE: Rule = Rule(kw(Kw::Group), &[Next::Kw(Kw::By)]);

const GROUP_BY_SUGGEST_RULE: Rule = Rule(
    Con::seq(&[
        Con::not(&Con::or(&[
            kw(Kw::Select),
            kw(Kw::Where),
            kw(Kw::Group),
            kw(Kw::Order),
            kw(Kw::By),
            kw(Kw::Having),
            kw(Kw::With),
            op(Op::And),
            op(Op::Or),
            tk(Tk::Dot),
        ])),
        SIMPLE_VALUE,
    ]),
    &[Next::KwSeq(&[Kw::Group, Kw::By])],
);

const JOIN_RULE: Rule = Rule(kw(Kw::Join), &[Next::Kw(Kw::On), Next::Kw(Kw::Using)]);

const INNER_JOIN_RULE: Rule = Rule(kw(Kw::Inner), &[Next::Kw(Kw::Join)]);

const LEFT_JOIN_RULE: Rule = Rule(kw(Kw::Left), &[Next::Kw(Kw::Join), Next::Kw(Kw::Outer)]);

const RIGHT_JOIN_RULE: Rule = Rule(kw(Kw::Right), &[Next::Kw(Kw::Join), Next::Kw(Kw::Outer)]);

const FULL_JOIN_RULE: Rule = Rule(kw(Kw::Full), &[Next::Kw(Kw::Join), Next::Kw(Kw::Outer)]);

const CROSS_JOIN_RULE: Rule = Rule(kw(Kw::Cross), &[Next::Kw(Kw::Join)]);

// Suggest JOIN keywords after a table in FROM clause
const JOIN_AFTER_FROM_RULE: Rule = Rule(
    Con::seq(&[
        kw(Kw::From),
        Con::peek(&Con::not(&Con::or(&[
            kw(Kw::Select),
            kw(Kw::Where),
            kw(Kw::Join),
            kw(Kw::Inner),
            kw(Kw::Left),
            kw(Kw::Right),
            kw(Kw::Full),
            kw(Kw::Cross),
            kw(Kw::Natural),
            kw(Kw::Group),
            kw(Kw::Order),
            kw(Kw::By),
            kw(Kw::Having),
            kw(Kw::Limit),
            kw(Kw::With),
            tk(Tk::Dot),
        ]))),
        Con::or(&[
            SUB_QUERY,
            tk(Tk::Identifier),
            Con::seq(&[tk(Tk::Identifier), tk(Tk::Identifier)]),
        ]),
    ]),
    &[
        Next::Kw(Kw::Join),
        Next::KwSeq(&[Kw::Inner, Kw::Join]),
        Next::KwSeq(&[Kw::Left, Kw::Join]),
        Next::KwSeq(&[Kw::Right, Kw::Join]),
        Next::KwSeq(&[Kw::Full, Kw::Join]),
        Next::KwSeq(&[Kw::Cross, Kw::Join]),
        Next::KwSeq(&[Kw::Natural, Kw::Join]),
    ],
);

// OUTER can follow LEFT, RIGHT, or FULL
const LEFT_OUTER_JOIN_RULE: Rule = Rule(
    Con::seq(&[kw(Kw::Left), kw(Kw::Outer)]),
    &[Next::Kw(Kw::Join)],
);

const RIGHT_OUTER_JOIN_RULE: Rule = Rule(
    Con::seq(&[kw(Kw::Right), kw(Kw::Outer)]),
    &[Next::Kw(Kw::Join)],
);

const FULL_OUTER_JOIN_RULE: Rule = Rule(
    Con::seq(&[kw(Kw::Full), kw(Kw::Outer)]),
    &[Next::Kw(Kw::Join)],
);

// NATURAL can combine with join types
const NATURAL_JOIN_RULE: Rule = Rule(
    kw(Kw::Natural),
    &[
        Next::Kw(Kw::Join),
        Next::Kw(Kw::Inner),
        Next::Kw(Kw::Left),
        Next::Kw(Kw::Right),
        Next::Kw(Kw::Full),
    ],
);

const HAVING_RULE: Rule = Rule(kw(Kw::Group), &[Next::KwSeq(&[Kw::By])]);

const HAVING_AFTER_GROUP_BY_RULE: Rule = Rule(
    Con::seq(&[
        kw(Kw::Group),
        kw(Kw::By),
        Con::many(&Con::not(&Con::or(&[
            kw(Kw::By),
            kw(Kw::Group),
            kw(Kw::Order),
            kw(Kw::Limit),
            kw(Kw::Having),
            kw(Kw::Union),
            kw(Kw::Intersect),
            kw(Kw::Except),
            kw(Kw::Offset),
            kw(Kw::Fetch),
        ]))),
        SIMPLE_VALUE,
    ]),
    &[Next::Kw(Kw::Having)],
);

const INSERT_INTO_RULE: Rule = Rule(kw(Kw::Insert), &[Next::Kw(Kw::Into)]);

const CREATE_TABLE_RULE: Rule = Rule(
    kw(Kw::Create),
    &[
        Next::Kw(Kw::Table),
        Next::Kw(Kw::View),
        Next::Kw(Kw::Schema),
    ],
);

const PRIMARY_KEY_RULE: Rule = Rule(kw(Kw::Primary), &[Next::KwSeq(&[Kw::Key])]);

const FOREIGN_KEY_RULE: Rule = Rule(kw(Kw::Foreign), &[Next::KwSeq(&[Kw::Key])]);

// CASE statement rules
// Suggest WHEN after CASE keyword
const CASE_WHEN_RULE: Rule = Rule(kw(Kw::Case), &[Next::Kw(Kw::When)]);

// Suggest THEN after a value following WHEN
const WHEN_THEN_RULE: Rule = Rule(
    Con::seq(&[
        kw(Kw::When),
        Con::many(&Con::not(&Con::or(&[
            kw(Kw::When),
            kw(Kw::Then),
            kw(Kw::Else),
            kw(Kw::End),
        ]))),
        SIMPLE_VALUE,
    ]),
    &[Next::Kw(Kw::Then)],
);

// Suggest WHEN, ELSE, END after a value following THEN
const THEN_FOLLOW_RULE: Rule = Rule(
    Con::seq(&[
        kw(Kw::Then),
        Con::many(&Con::not(&Con::or(&[
            kw(Kw::When),
            kw(Kw::Then),
            kw(Kw::Else),
            kw(Kw::End),
        ]))),
        SIMPLE_VALUE,
    ]),
    &[Next::Kw(Kw::When), Next::Kw(Kw::Else), Next::Kw(Kw::End)],
);

// Suggest END after a value following ELSE
const ELSE_END_RULE: Rule = Rule(
    Con::seq(&[
        kw(Kw::Else),
        Con::many(&Con::not(&Con::or(&[kw(Kw::Else), kw(Kw::End)]))),
        SIMPLE_VALUE,
    ]),
    &[Next::Kw(Kw::End)],
);

const SET_OPERATION_ALL_RULE: Rule = Rule(
    Con::or(&[kw(Kw::Union), kw(Kw::Intersect), kw(Kw::Except)]),
    &[Next::KwSeq(&[Kw::All])],
);

const SET_OP_RULE: Rule = Rule(
    Con::seq(&[
        Con::not(&Con::or(&[
            kw(Kw::Select),
            kw(Kw::Where),
            kw(Kw::Group),
            kw(Kw::Order),
            kw(Kw::Having),
            kw(Kw::With),
            tk(Tk::Comma),
            op(Op::And),
            op(Op::Or),
            tk(Tk::Dot),
        ])),
        SIMPLE_VALUE,
    ]),
    &[
        Next::Kw(Kw::Union),
        Next::Kw(Kw::Intersect),
        Next::Kw(Kw::Except),
    ],
);

// ---------------- Utility Rules ----------------
const SIMPLE_VALUE: Con = Con::or(&[
    tk(Tk::Identifier),
    tk(Tk::RightParen),
    tk(Tk::Number),
    tk(Tk::Str),
]);

const SUB_QUERY: Con = Con::Scoped(&tk(Tk::LeftParen), &tk(Tk::RightParen));

const END_CONDITION: Con = Con::or(&[
    Con::Eof,
    tk(Tk::Semicolon),
    tk(Tk::LeftParen),
    Con::seq(&[kw(Kw::With), Con::Until(&kw(Kw::With)), tk(Tk::RightParen)]),
]);

#[cfg(test)]
mod tests {
    use pat::match_all;

    use super::*;
    use crate::dialect::ansi;
    use crate::lex::lex;

    #[test]
    fn start_rule() {
        // Should match at the start
        matches(true, &START_RULE, "");
        matches(false, &START_RULE, "SELECT");

        // Should match after semicolon
        matches(true, &START_RULE, ";");
        matches(true, &START_RULE, ";SE");

        // Should match after CTE definition
        matches(true, &START_RULE, "WITH cte (SELECT a FROM b) ");
        matches(true, &START_RULE, "WITH cte (SELECT a FROM b) S");

        // Should match after multiple CTEs
        matches(
            true,
            &START_RULE,
            "WITH cte1 (SELECT a FROM b), cte2 (SELECT c FROM d) ",
        );

        // Should match after nested subquery within CTE
        matches(
            true,
            &START_RULE,
            "WITH cte (SELECT a FROM (SELECT b FROM c)) ",
        );

        // Should NOT match after table subqueries (not a CTE context)
        matches(
            false,
            &START_RULE,
            "SELECT * FROM (SELECT a FROM b)
        ",
        );
        matches(
            false,
            &START_RULE,
            "SELECT * FROM (SELECT
        a FROM b) t ",
        );

        // Should NOT match after WHERE clause subqueries
        matches(
            false,
            &START_RULE,
            "SELECT * FROM t WHERE id IN (SELECT id FROM users) ",
        );
    }

    #[test]
    fn qualified_column_exclusion() {
        // After qualified columns (table.column), should NOT suggest clause keywords
        // Should suggest operators/AND/OR instead

        // Should NOT suggest LIMIT after qualified column
        matches(
            false,
            &LIMIT_RULE,
            "SELECT data FROM ingest WHERE ingest.data ",
        );

        // Should NOT suggest ORDER BY after qualified column
        matches(
            false,
            &ORDER_BY_SUGGEST_RULE,
            "SELECT data FROM ingest WHERE ingest.data ",
        );

        // Should NOT suggest GROUP BY after qualified column
        matches(
            false,
            &GROUP_BY_SUGGEST_RULE,
            "SELECT data FROM ingest WHERE ingest.data ",
        );

        // Should NOT suggest JOIN after qualified column
        matches(
            false,
            &JOIN_AFTER_FROM_RULE,
            "SELECT data FROM ingest WHERE ingest.data ",
        );

        // Should NOT suggest set operations after qualified column
        matches(
            false,
            &SET_OP_RULE,
            "SELECT data FROM ingest WHERE ingest.data ",
        );

        // But unqualified columns should still work
        matches(true, &LIMIT_RULE, "SELECT data FROM ingest WHERE data = 1 ");
        matches(
            true,
            &ORDER_BY_SUGGEST_RULE,
            "SELECT data FROM ingest WHERE data = 1 ",
        );
    }

    #[test]
    fn select_rule() {
        matches(true, &SELECT_RULE, "SELECT ");
        matches(false, &SELECT_RULE, "SELECT DISTINCT");
        matches(false, &SELECT_RULE, "SELECT ALL");
        matches(false, &SELECT_RULE, "FROM");
        matches(false, &SELECT_RULE, "");
        matches(false, &SELECT_RULE, "SELECT column");
    }

    #[test]
    fn select_as_rule() {
        matches(true, &SELECT_AS_RULE, "SELECT a ");
        matches(true, &SELECT_AS_RULE, "SELECT a, b ");
        matches(false, &SELECT_AS_RULE, "SELECT a, ");
        matches(false, &SELECT_AS_RULE, "SELECT a FROM");
    }

    #[test]
    fn from_rule() {
        matches(true, &FROM_RULE, "SELECT a ");
        matches(true, &FROM_RULE, "SELECT column1, column2 ");
        matches(true, &FROM_RULE, "SELECT (1 + 2) ");
        matches(true, &FROM_RULE, "SELECT * ");
        matches(true, &FROM_RULE, "SELECT a, * ");
        matches(true, &FROM_RULE, "SELECT a + b ");
        matches(true, &FROM_RULE, "SELECT 1 * 2 ");
        matches(false, &FROM_RULE, "SELECT 1 as ");
        matches(false, &FROM_RULE, "SELECT 1 * ");
        matches(false, &FROM_RULE, "SELECT a FROM");
        matches(false, &FROM_RULE, "SELECT a FROM users");
        matches(true, &FROM_RULE, "WITH foo AS (SELECT a FROM b) SELECT * ");
        matches(
            true,
            &FROM_RULE,
            "WITH foo AS (SELECT a FROM b) SELECT a, * ",
        );
        matches(
            true,
            &FROM_RULE,
            "WITH something AS (SELECT * FROM ai_chat WHERE connection_id =
        '123') SELECT connection_id ",
        );
        matches(
            true,
            &FROM_RULE,
            "WITH foo AS (SELECT a FROM b) SELECT a, b ",
        );
    }

    #[test]
    fn where_rule() {
        matches(true, &WHERE_RULE, "SELECT a FROM users ");
        matches(true, &WHERE_RULE, "SELECT a FROM table1, table2 ");
        matches(true, &WHERE_RULE, "SELECT a FROM (SELECT b FROM c) ");
        matches(false, &WHERE_RULE, "SELECT a FROM users WHERE");
        matches(false, &WHERE_RULE, "SELECT a FROM users WHERE id = 1");
        matches(false, &WHERE_RULE, "");
        matches(false, &WHERE_RULE, "SELECT a ");
    }

    #[test]
    fn if_match_limit() {
        matches(true, &LIMIT_RULE, "SELECT a FROM users ");
        matches(false, &LIMIT_RULE, "SELECT a FROM users WHERE");
        matches(false, &LIMIT_RULE, "SELECT a FROM users WHERE ");
        matches(false, &LIMIT_RULE, "SELECT a FROM users WHERE id");
        matches(false, &LIMIT_RULE, "SELECT a FROM users WHERE id =");
        matches(true, &LIMIT_RULE, "SELECT a FROM users WHERE id = 1 ");
        matches(
            true,
            &LIMIT_RULE,
            "SELECT a FROM users WHERE name = 'John'
        ",
        );
        matches(
            false,
            &LIMIT_RULE,
            "SELECT a FROM users WHERE name = 'John' AND",
        );
        matches(
            false,
            &LIMIT_RULE,
            "SELECT a FROM users WHERE name = 'John' AND age",
        );
    }

    #[test]
    fn subquery_rule() {
        matches(true, &SUBQUERY_RULE, "(");
        matches(true, &SUBQUERY_RULE, "SELECT a FROM (");
        matches(false, &SUBQUERY_RULE, "");
        matches(false, &SUBQUERY_RULE, "SELECT");
        matches(false, &SUBQUERY_RULE, ")");
    }

    #[test]
    fn with_cte_rule() {
        matches(true, &WITH_CTE_RECURSIVE_RULE, "WITH ");
        matches(false, &WITH_CTE_RECURSIVE_RULE, "WITH RECURSIVE");
        matches(false, &WITH_CTE_RECURSIVE_RULE, "");
        matches(false, &WITH_CTE_RECURSIVE_RULE, "SELECT");
        matches(false, &WITH_CTE_RECURSIVE_RULE, "RECURSIVE");
        matches(false, &WITH_CTE_RECURSIVE_RULE, "WITH cte_name");
        matches(true, &WITH_CTE_AS_RULE, "WITH cte_name");
    }

    #[test]
    fn order_by_rule() {
        matches(true, &ORDER_BY_RULE, "ORDER ");
        matches(false, &ORDER_BY_RULE, "ORDER BY");
        matches(false, &ORDER_BY_RULE, "");
        matches(false, &ORDER_BY_RULE, "SELECT");
        matches(false, &ORDER_BY_RULE, "BY");
        matches(false, &ORDER_BY_RULE, "ORDER id");
    }

    #[test]
    fn order_by_suggest_rule() {
        // Should suggest ORDER BY after complete SELECT/FROM/WHERE clauses
        matches(true, &ORDER_BY_SUGGEST_RULE, "SELECT a FROM users ");
        matches(
            true,
            &ORDER_BY_SUGGEST_RULE,
            "SELECT a FROM users WHERE id = 1 ",
        );
        matches(
            true,
            &ORDER_BY_SUGGEST_RULE,
            "SELECT a FROM users WHERE name = 'John' ",
        );

        // Should NOT suggest after incomplete clauses
        matches(false, &ORDER_BY_SUGGEST_RULE, "SELECT ");
        matches(false, &ORDER_BY_SUGGEST_RULE, "SELECT a");
        matches(false, &ORDER_BY_SUGGEST_RULE, "SELECT a, ");
        matches(false, &ORDER_BY_SUGGEST_RULE, "SELECT a FROM users WHERE");
        matches(false, &ORDER_BY_SUGGEST_RULE, "SELECT a FROM users WHERE ");
        matches(
            false,
            &ORDER_BY_SUGGEST_RULE,
            "SELECT a FROM users WHERE id",
        );
        matches(
            false,
            &ORDER_BY_SUGGEST_RULE,
            "SELECT a FROM users WHERE id =",
        );
        matches(
            false,
            &ORDER_BY_SUGGEST_RULE,
            "SELECT a FROM users WHERE name = 'John' AND",
        );
        matches(
            false,
            &ORDER_BY_SUGGEST_RULE,
            "SELECT a FROM users WHERE name = 'John' AND age",
        );

        // Should NOT suggest after clause keywords
        matches(false, &ORDER_BY_SUGGEST_RULE, "SELECT a FROM users GROUP");
        matches(false, &ORDER_BY_SUGGEST_RULE, "SELECT a FROM users ORDER");
        matches(false, &ORDER_BY_SUGGEST_RULE, "SELECT a FROM users HAVING");

        // Should NOT suggest after GROUP BY (BY keyword is excluded to prevent
        // suggesting after ORDER BY)
        matches(
            false,
            &ORDER_BY_SUGGEST_RULE,
            "SELECT COUNT(*) FROM users GROUP BY name ",
        );

        // Should NOT suggest after ORDER BY is already used
        matches(
            false,
            &ORDER_BY_SUGGEST_RULE,
            "SELECT a FROM users ORDER BY name ",
        );
    }

    #[test]
    fn group_by_rule() {
        matches(true, &GROUP_BY_RULE, "GROUP ");
        matches(false, &GROUP_BY_RULE, "GROUP BY");
        matches(false, &GROUP_BY_RULE, "");
        matches(false, &GROUP_BY_RULE, "SELECT");
        matches(false, &GROUP_BY_RULE, "BY");
    }

    #[test]
    fn group_by_suggest_rule() {
        // Should suggest GROUP BY after complete SELECT/FROM/WHERE clauses
        matches(true, &GROUP_BY_SUGGEST_RULE, "SELECT a FROM users ");
        matches(
            true,
            &GROUP_BY_SUGGEST_RULE,
            "SELECT a FROM users WHERE id = 1 ",
        );
        matches(
            true,
            &GROUP_BY_SUGGEST_RULE,
            "SELECT a FROM users WHERE name = 'John' ",
        );

        // Should NOT suggest after incomplete clauses
        matches(false, &GROUP_BY_SUGGEST_RULE, "SELECT ");
        matches(false, &GROUP_BY_SUGGEST_RULE, "SELECT a");
        matches(false, &GROUP_BY_SUGGEST_RULE, "SELECT a, ");
        matches(false, &GROUP_BY_SUGGEST_RULE, "SELECT a FROM users WHERE");
        matches(false, &GROUP_BY_SUGGEST_RULE, "SELECT a FROM users WHERE ");
        matches(
            false,
            &GROUP_BY_SUGGEST_RULE,
            "SELECT a FROM users WHERE id",
        );
        matches(
            false,
            &GROUP_BY_SUGGEST_RULE,
            "SELECT a FROM users WHERE id =",
        );
        matches(
            false,
            &GROUP_BY_SUGGEST_RULE,
            "SELECT a FROM users WHERE name = 'John' AND",
        );
        matches(
            false,
            &GROUP_BY_SUGGEST_RULE,
            "SELECT a FROM users WHERE name = 'John' AND age",
        );

        // Should NOT suggest after GROUP/ORDER/HAVING keywords
        matches(false, &GROUP_BY_SUGGEST_RULE, "SELECT a FROM users GROUP");
        matches(false, &GROUP_BY_SUGGEST_RULE, "SELECT a FROM users ORDER");
        matches(false, &GROUP_BY_SUGGEST_RULE, "SELECT a FROM users HAVING");
    }

    #[test]
    fn having_after_group_by_rule() {
        // Should suggest HAVING after GROUP BY with column
        matches(
            true,
            &HAVING_AFTER_GROUP_BY_RULE,
            "SELECT COUNT(*) FROM users GROUP BY name ",
        );
        matches(
            true,
            &HAVING_AFTER_GROUP_BY_RULE,
            "SELECT a FROM users GROUP BY col1, col2 ",
        );

        // Should NOT suggest immediately after GROUP BY keywords
        matches(
            false,
            &HAVING_AFTER_GROUP_BY_RULE,
            "SELECT a FROM users GROUP",
        );
        matches(
            false,
            &HAVING_AFTER_GROUP_BY_RULE,
            "SELECT a FROM users GROUP ",
        );
        matches(
            false,
            &HAVING_AFTER_GROUP_BY_RULE,
            "SELECT a FROM users GROUP BY",
        );
        matches(
            false,
            &HAVING_AFTER_GROUP_BY_RULE,
            "SELECT a FROM users GROUP BY ",
        );

        // Should NOT suggest after HAVING is already used
        matches(
            false,
            &HAVING_AFTER_GROUP_BY_RULE,
            "SELECT a FROM users GROUP BY name HAVING COUNT(*) > 1 ",
        );

        // Should NOT suggest after ORDER BY or LIMIT
        matches(
            false,
            &HAVING_AFTER_GROUP_BY_RULE,
            "SELECT a FROM users GROUP BY name ORDER BY name ",
        );
        matches(
            false,
            &HAVING_AFTER_GROUP_BY_RULE,
            "SELECT a FROM users GROUP BY name LIMIT 10 ",
        );
    }

    #[test]
    fn join_rule() {
        matches(true, &JOIN_RULE, "SELECT * FROM users JOIN ");
        matches(false, &JOIN_RULE, "SELECT * FROM users JOIN posts ON");
        matches(false, &JOIN_RULE, "");
        matches(false, &JOIN_RULE, "SELECT");
    }

    #[test]
    fn inner_join_rule() {
        matches(true, &INNER_JOIN_RULE, "INNER ");
        matches(false, &INNER_JOIN_RULE, "INNER JOIN");
        matches(false, &INNER_JOIN_RULE, "");
        matches(false, &INNER_JOIN_RULE, "JOIN");
    }

    #[test]
    fn left_join_rule() {
        // Should suggest both JOIN and OUTER after LEFT
        matches(true, &LEFT_JOIN_RULE, "LEFT ");
        matches(false, &LEFT_JOIN_RULE, "LEFT JOIN");
        matches(false, &LEFT_JOIN_RULE, "LEFT OUTER");
        matches(false, &LEFT_JOIN_RULE, "");
    }

    #[test]
    fn right_join_rule() {
        // Should suggest both JOIN and OUTER after RIGHT
        matches(true, &RIGHT_JOIN_RULE, "RIGHT ");
        matches(false, &RIGHT_JOIN_RULE, "RIGHT JOIN");
        matches(false, &RIGHT_JOIN_RULE, "RIGHT OUTER");
        matches(false, &RIGHT_JOIN_RULE, "");
    }

    #[test]
    fn full_join_rule() {
        // Should suggest both JOIN and OUTER after FULL
        matches(true, &FULL_JOIN_RULE, "FULL ");
        matches(false, &FULL_JOIN_RULE, "FULL JOIN");
        matches(false, &FULL_JOIN_RULE, "FULL OUTER");
        matches(false, &FULL_JOIN_RULE, "");
    }

    #[test]
    fn cross_join_rule() {
        matches(true, &CROSS_JOIN_RULE, "CROSS ");
        matches(false, &CROSS_JOIN_RULE, "CROSS JOIN");
        matches(false, &CROSS_JOIN_RULE, "");
    }

    #[test]
    fn insert_into_rule() {
        matches(true, &INSERT_INTO_RULE, "INSERT ");
        matches(false, &INSERT_INTO_RULE, "INSERT INTO");
        matches(false, &INSERT_INTO_RULE, "");
        matches(false, &INSERT_INTO_RULE, "INTO");
    }

    #[test]
    fn create_table_rule() {
        matches(true, &CREATE_TABLE_RULE, "CREATE ");
        matches(false, &CREATE_TABLE_RULE, "CREATE TABLE");
        matches(false, &CREATE_TABLE_RULE, "CREATE VIEW");
        matches(false, &CREATE_TABLE_RULE, "");
        matches(false, &CREATE_TABLE_RULE, "TABLE");
    }

    #[test]
    fn primary_key_rule() {
        matches(true, &PRIMARY_KEY_RULE, "PRIMARY ");
        matches(false, &PRIMARY_KEY_RULE, "PRIMARY KEY");
        matches(false, &PRIMARY_KEY_RULE, "");
        matches(false, &PRIMARY_KEY_RULE, "KEY");
    }

    #[test]
    fn foreign_key_rule() {
        matches(true, &FOREIGN_KEY_RULE, "FOREIGN ");
        matches(false, &FOREIGN_KEY_RULE, "FOREIGN KEY");
        matches(false, &FOREIGN_KEY_RULE, "");
        matches(false, &FOREIGN_KEY_RULE, "KEY");
    }

    #[test]
    fn set_operation_all_rule() {
        matches(true, &SET_OPERATION_ALL_RULE, "SELECT a FROM users UNION ");
        matches(
            true,
            &SET_OPERATION_ALL_RULE,
            "SELECT a FROM users INTERSECT ",
        );
        matches(true, &SET_OPERATION_ALL_RULE, "SELECT a FROM users EXCEPT ");
        matches(
            false,
            &SET_OPERATION_ALL_RULE,
            "SELECT a FROM users UNION ALL",
        );
        matches(false, &SET_OPERATION_ALL_RULE, "");
    }

    #[test]
    fn set_op_rule() {
        // Should suggest set operations after a complete SELECT statement
        matches(true, &SET_OP_RULE, "SELECT a FROM users ");
        matches(true, &SET_OP_RULE, "SELECT a FROM users WHERE id = 1 ");
        matches(true, &SET_OP_RULE, "SELECT a FROM users ORDER BY name ");

        // Should NOT suggest after incomplete clauses
        matches(false, &SET_OP_RULE, "SELECT a FROM users WHERE");
        matches(false, &SET_OP_RULE, "SELECT a FROM users WHERE ");
        matches(false, &SET_OP_RULE, "SELECT a FROM users WHERE id");
        matches(false, &SET_OP_RULE, "SELECT a FROM users WHERE id =");
        matches(
            false,
            &SET_OP_RULE,
            "SELECT a FROM users WHERE name = 'John' AND",
        );
        matches(
            false,
            &SET_OP_RULE,
            "SELECT a FROM users WHERE name = 'John' AND age",
        );

        // Should NOT suggest in the middle of SELECT list
        matches(false, &SET_OP_RULE, "SELECT ");
        matches(false, &SET_OP_RULE, "SELECT a");
        matches(false, &SET_OP_RULE, "SELECT a, ");
    }

    #[test]
    fn join_after_from_rule() {
        // Should suggest JOIN types after table name in FROM clause
        matches(true, &JOIN_AFTER_FROM_RULE, "SELECT * FROM users ");
        matches(
            true,
            &JOIN_AFTER_FROM_RULE,
            "SELECT * FROM (SELECT id FROM t) ",
        );

        // Should suggest JOIN after a table with alias
        matches(true, &JOIN_AFTER_FROM_RULE, "SELECT * FROM users u");

        // // Should NOT suggest in select
        matches(false, &JOIN_AFTER_FROM_RULE, "SELECT a");
        matches(false, &JOIN_AFTER_FROM_RULE, "SELECT a, b");
        matches(false, &JOIN_AFTER_FROM_RULE, "SELECT a, *");

        // Should NOT suggest after WHERE or other clauses
        matches(false, &JOIN_AFTER_FROM_RULE, "SELECT * FROM users WHERE");
        matches(false, &JOIN_AFTER_FROM_RULE, "SELECT * FROM users WHERE ");
        matches(false, &JOIN_AFTER_FROM_RULE, "SELECT * ");

        // Should NOT suggest if already have JOIN keyword
        matches(false, &JOIN_AFTER_FROM_RULE, "SELECT * FROM users JOIN");
        matches(false, &JOIN_AFTER_FROM_RULE, "SELECT * FROM users INNER");
        matches(false, &JOIN_AFTER_FROM_RULE, "SELECT * FROM users LEFT");
    }

    #[test]
    fn left_outer_join_rule() {
        matches(
            true,
            &LEFT_OUTER_JOIN_RULE,
            "SELECT * FROM users LEFT OUTER ",
        );
        matches(
            false,
            &LEFT_OUTER_JOIN_RULE,
            "SELECT * FROM users LEFT OUTER JOIN",
        );
        matches(false, &LEFT_OUTER_JOIN_RULE, "SELECT * FROM users LEFT ");
        matches(false, &LEFT_OUTER_JOIN_RULE, "");
    }

    #[test]
    fn right_outer_join_rule() {
        matches(
            true,
            &RIGHT_OUTER_JOIN_RULE,
            "SELECT * FROM users RIGHT OUTER ",
        );
        matches(
            false,
            &RIGHT_OUTER_JOIN_RULE,
            "SELECT * FROM users RIGHT OUTER JOIN",
        );
        matches(false, &RIGHT_OUTER_JOIN_RULE, "");
    }

    #[test]
    fn full_outer_join_rule() {
        matches(
            true,
            &FULL_OUTER_JOIN_RULE,
            "SELECT * FROM users FULL OUTER ",
        );
        matches(
            false,
            &FULL_OUTER_JOIN_RULE,
            "SELECT * FROM users FULL OUTER JOIN",
        );
        matches(false, &FULL_OUTER_JOIN_RULE, "");
    }

    #[test]
    fn natural_join_rule() {
        // Should suggest JOIN types after NATURAL
        matches(true, &NATURAL_JOIN_RULE, "SELECT * FROM users NATURAL ");
        matches(
            false,
            &NATURAL_JOIN_RULE,
            "SELECT * FROM users NATURAL JOIN",
        );
        matches(
            false,
            &NATURAL_JOIN_RULE,
            "SELECT * FROM users NATURAL INNER",
        );
        matches(
            false,
            &NATURAL_JOIN_RULE,
            "SELECT * FROM users NATURAL LEFT",
        );
        matches(false, &NATURAL_JOIN_RULE, "");
    }

    #[test]
    fn order_by_dir_rule() {
        // Should suggest ASC, DESC, NULLS after ORDER BY columnname
        matches(
            true,
            &ORDER_BY_DIR_RULE,
            "SELECT a FROM users ORDER BY name ",
        );
        matches(true, &ORDER_BY_DIR_RULE, "SELECT a FROM users ORDER BY id ");
        matches(
            true,
            &ORDER_BY_DIR_RULE,
            "SELECT a, b FROM users ORDER BY a ",
        );

        // Should NOT suggest after ASC/DESC/NULLS is already used
        matches(
            false,
            &ORDER_BY_DIR_RULE,
            "SELECT a FROM users ORDER BY name ASC ",
        );
        matches(
            false,
            &ORDER_BY_DIR_RULE,
            "SELECT a FROM users ORDER BY name DESC ",
        );
        matches(
            false,
            &ORDER_BY_DIR_RULE,
            "SELECT a FROM users ORDER BY name NULLS ",
        );

        // Should NOT suggest before BY
        matches(false, &ORDER_BY_DIR_RULE, "SELECT a FROM users ORDER ");
        matches(false, &ORDER_BY_DIR_RULE, "SELECT a FROM users ORDER BY ");

        // Should NOT match columns after commas (that's ORDER_BY_DIR_AFTER_COMMA_RULE's
        // job)
        matches(
            false,
            &ORDER_BY_DIR_RULE,
            "SELECT a FROM users ORDER BY name ASC, id ",
        );
    }

    #[test]
    fn order_by_dir_after_comma_rule() {
        // Should suggest ASC, DESC, NULLS after column following comma in ORDER BY
        matches(
            true,
            &ORDER_BY_DIR_AFTER_COMMA_RULE,
            "SELECT a FROM users ORDER BY name ASC, id ",
        );
        matches(
            true,
            &ORDER_BY_DIR_AFTER_COMMA_RULE,
            "SELECT a FROM users ORDER BY a, b ",
        );
        matches(
            true,
            &ORDER_BY_DIR_AFTER_COMMA_RULE,
            "SELECT a FROM users ORDER BY a DESC, b ASC, c ",
        );

        // Should NOT suggest after first column (that's ORDER_BY_DIR_RULE's job)
        matches(
            false,
            &ORDER_BY_DIR_AFTER_COMMA_RULE,
            "SELECT a FROM users ORDER BY name ",
        );

        // Should NOT suggest after ASC/DESC/NULLS following the comma
        matches(
            false,
            &ORDER_BY_DIR_AFTER_COMMA_RULE,
            "SELECT a FROM users ORDER BY name ASC, id ASC ",
        );
        matches(
            false,
            &ORDER_BY_DIR_AFTER_COMMA_RULE,
            "SELECT a FROM users ORDER BY name, id DESC ",
        );

        // Should NOT suggest after commas in SELECT list or WHERE clause
        matches(false, &ORDER_BY_DIR_AFTER_COMMA_RULE, "SELECT a, b ");
        matches(
            false,
            &ORDER_BY_DIR_AFTER_COMMA_RULE,
            "SELECT a, b FROM users",
        );
        matches(
            false,
            &ORDER_BY_DIR_AFTER_COMMA_RULE,
            "SELECT a FROM users WHERE id IN (1, 2 ",
        );
    }

    #[test]
    fn subquery_tests() {
        // Test that LIMIT_RULE works in subqueries
        matches(true, &LIMIT_RULE, "SELECT * FROM (SELECT a FROM users ");
        matches(
            true,
            &LIMIT_RULE,
            "SELECT * FROM (SELECT a FROM users WHERE id = 1 ",
        );

        // Test that ORDER_BY_SUGGEST_RULE works in subqueries
        matches(
            true,
            &ORDER_BY_SUGGEST_RULE,
            "SELECT * FROM (SELECT a FROM users ",
        );
        matches(
            true,
            &ORDER_BY_SUGGEST_RULE,
            "SELECT * FROM (SELECT a FROM users WHERE id = 1 ",
        );

        // Test that ORDER_BY_DIR_RULE works in subqueries
        matches(
            true,
            &ORDER_BY_DIR_RULE,
            "SELECT * FROM (SELECT a FROM users ORDER BY name ",
        );
        matches(
            false,
            &ORDER_BY_DIR_RULE,
            "SELECT * FROM (SELECT a FROM users ORDER BY name ASC ",
        );

        // Test that ORDER_BY_DIR_AFTER_COMMA_RULE works in subqueries
        matches(
            true,
            &ORDER_BY_DIR_AFTER_COMMA_RULE,
            "SELECT * FROM (SELECT a FROM users ORDER BY name ASC, id ",
        );

        // Test that GROUP_BY_SUGGEST_RULE works in subqueries
        matches(
            true,
            &GROUP_BY_SUGGEST_RULE,
            "SELECT * FROM (SELECT COUNT(*) FROM users ",
        );

        // Test that SET_OP_RULE works in subqueries
        matches(true, &SET_OP_RULE, "SELECT * FROM (SELECT a FROM users ");
        matches(
            true,
            &SET_OP_RULE,
            "SELECT * FROM (SELECT a FROM users ORDER BY name ",
        );

        // Test that JOIN_AFTER_FROM_RULE works in subqueries
        matches(
            true,
            &JOIN_AFTER_FROM_RULE,
            "SELECT * FROM (SELECT * FROM users ",
        );

        // Test that outer query rules match after closing paren of subquery
        // (The closing paren acts like a table name for the outer query)
        matches(true, &LIMIT_RULE, "SELECT * FROM (SELECT a FROM users) ");
        matches(
            true,
            &ORDER_BY_SUGGEST_RULE,
            "SELECT * FROM (SELECT a FROM users) ",
        );
        matches(
            true,
            &JOIN_AFTER_FROM_RULE,
            "SELECT * FROM (SELECT a FROM users) ",
        );
        matches(true, &SET_OP_RULE, "SELECT * FROM (SELECT a FROM users) ");

        // ORDER_BY_DIR_RULE SHOULD match after closing paren if there's ORDER BY before
        // it This handles cases like: ORDER BY (expression) or ORDER BY
        // (subquery_column)
        matches(
            true,
            &ORDER_BY_DIR_RULE,
            "SELECT * FROM (SELECT a FROM users ORDER BY name) ",
        );

        // Test nested ORDER BY - inner query with ORDER BY
        matches(
            true,
            &ORDER_BY_DIR_RULE,
            "SELECT * FROM t ORDER BY (SELECT name FROM users ",
        );
    }

    #[test]
    fn case_when_rule() {
        // Should suggest WHEN after CASE
        matches(true, &CASE_WHEN_RULE, "SELECT CASE ");
        matches(true, &CASE_WHEN_RULE, "SELECT a, CASE ");
        matches(
            true,
            &CASE_WHEN_RULE,
            "SELECT a FROM users WHERE status = CASE ",
        );

        // Should NOT suggest after WHEN is already used
        matches(false, &CASE_WHEN_RULE, "SELECT CASE WHEN ");
        matches(false, &CASE_WHEN_RULE, "SELECT CASE WHEN a = 1 ");
    }

    #[test]
    fn when_then_rule() {
        // Should suggest THEN after WHEN with a value
        matches(true, &WHEN_THEN_RULE, "SELECT CASE WHEN a = 1 ");
        matches(true, &WHEN_THEN_RULE, "SELECT CASE WHEN status = 'active' ");
        matches(true, &WHEN_THEN_RULE, "SELECT CASE WHEN (a > 0) ");
        // Single column is valid for boolean columns
        matches(true, &WHEN_THEN_RULE, "SELECT CASE WHEN is_active ");

        // Should NOT suggest before value is provided
        matches(false, &WHEN_THEN_RULE, "SELECT CASE WHEN ");
        // Should NOT suggest in the middle of an expression
        matches(false, &WHEN_THEN_RULE, "SELECT CASE WHEN a =");

        // Should NOT suggest after THEN is already used
        matches(false, &WHEN_THEN_RULE, "SELECT CASE WHEN a = 1 THEN ");
        matches(
            false,
            &WHEN_THEN_RULE,
            "SELECT CASE WHEN a = 1 THEN result ",
        );
    }

    #[test]
    fn then_follow_rule() {
        // Should suggest WHEN, ELSE, END after THEN with a value
        matches(
            true,
            &THEN_FOLLOW_RULE,
            "SELECT CASE WHEN a = 1 THEN result ",
        );
        matches(
            true,
            &THEN_FOLLOW_RULE,
            "SELECT CASE WHEN a = 1 THEN 'yes' ",
        );
        matches(
            true,
            &THEN_FOLLOW_RULE,
            "SELECT CASE WHEN a = 1 THEN (SELECT x FROM t) ",
        );

        // Should NOT suggest before value is provided
        matches(false, &THEN_FOLLOW_RULE, "SELECT CASE WHEN a = 1 THEN ");

        // Should work with multiple WHEN clauses
        matches(
            true,
            &THEN_FOLLOW_RULE,
            "SELECT CASE WHEN a = 1 THEN 'one' WHEN a = 2 THEN 'two' ",
        );
    }

    #[test]
    fn else_end_rule() {
        // Should suggest END after ELSE with a value
        matches(
            true,
            &ELSE_END_RULE,
            "SELECT CASE WHEN a = 1 THEN 'yes' ELSE 'no' ",
        );
        matches(
            true,
            &ELSE_END_RULE,
            "SELECT CASE WHEN a = 1 THEN result ELSE other ",
        );
        matches(
            true,
            &ELSE_END_RULE,
            "SELECT CASE WHEN a = 1 THEN (SELECT x FROM t) ELSE (SELECT y FROM t) ",
        );

        // Should NOT suggest before value is provided
        matches(
            false,
            &ELSE_END_RULE,
            "SELECT CASE WHEN a = 1 THEN result ELSE ",
        );

        // Should NOT suggest after END is already used
        matches(
            false,
            &ELSE_END_RULE,
            "SELECT CASE WHEN a = 1 THEN 'yes' ELSE 'no' END ",
        );
    }

    #[test]
    fn case_statement_full_flow() {
        // Test complete CASE statement flow
        // Simple CASE with single WHEN
        matches(true, &CASE_WHEN_RULE, "SELECT CASE ");
        matches(true, &WHEN_THEN_RULE, "SELECT CASE WHEN status = 'active' ");
        matches(
            true,
            &THEN_FOLLOW_RULE,
            "SELECT CASE WHEN status = 'active' THEN 1 ",
        );
        matches(
            true,
            &ELSE_END_RULE,
            "SELECT CASE WHEN status = 'active' THEN 1 ELSE 0 ",
        );

        // CASE with multiple WHEN clauses
        matches(
            true,
            &THEN_FOLLOW_RULE,
            "SELECT CASE WHEN a = 1 THEN 'one' ",
        );
        matches(
            true,
            &WHEN_THEN_RULE,
            "SELECT CASE WHEN a = 1 THEN 'one' WHEN a = 2 ",
        );
        matches(
            true,
            &THEN_FOLLOW_RULE,
            "SELECT CASE WHEN a = 1 THEN 'one' WHEN a = 2 THEN 'two' ",
        );

        // CASE in different contexts
        matches(true, &CASE_WHEN_RULE, "SELECT a, CASE ");
        matches(
            true,
            &CASE_WHEN_RULE,
            "SELECT a FROM users WHERE id = CASE ",
        );
        matches(true, &CASE_WHEN_RULE, "SELECT a FROM users ORDER BY CASE ");
    }

    fn matches(expected: bool, rule: &Rule, sql: &str) {
        let tokens = lex(&ansi::SPEC, sql);
        let kinds = tokens.iter().map(|t| t.kind).collect::<Vec<Tk>>();
        let kinds = &kinds[0..kinds.len().saturating_sub(1)]; // ignore the last token (EOF)

        let result = match_all::<true, _>(&rule.0, kinds);

        assert_eq!(
            result.is_ok(),
            expected,
            "\n\tevaluated to ({}) expected ({})\n\tinput: {:?}\n\trule {:?}\n\tresult: {:?}\n\tkinds: {}",
            result.is_ok(),
            expected,
            sql,
            rule,
            result,
            kinds.len(),
        );
    }
}
