use crate::dialect::Rules;
use crate::dialect::follow::{Cond, Next, Rule};
use crate::lex::{Keyword, OpTag, TokenKind};

const SIMPLE_VALUE: Cond = Cond::Any(&[
    Cond::Kind(TokenKind::Identifier),
    Cond::Kind(TokenKind::RightParen),
    Cond::Kind(TokenKind::Number),
    Cond::Kind(TokenKind::Str),
]);

#[cfg(test)]
use crate::dialect::follow::Dir;

pub const RULES: Rules = Rules(&[
    START_RULE,
    SELECT_RULE,
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
    WITH_CTE_RULE,
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

const END_CONDITION: Cond = Cond::Any(&[
    Cond::End,
    Cond::Kind(TokenKind::Semicolon),
    Cond::Kind(TokenKind::LeftParen),
    // Match closing paren after WITH keyword (CTE definition)
    Cond::Seq(&[
        Cond::Kw(Keyword::With),
        Cond::Until(&Cond::Kw(Keyword::With)),
        Cond::Kind(TokenKind::RightParen),
    ]),
]);

const START_RULE: Rule = Rule(
    Cond::Any(&[
        END_CONDITION,
        Cond::Seq(&[END_CONDITION, Cond::Kind(TokenKind::Identifier)]),
    ]),
    &[
        Next::Kw(Keyword::Select),
        Next::Kw(Keyword::Insert),
        Next::Kw(Keyword::Update),
        Next::Kw(Keyword::Delete),
        Next::Kw(Keyword::Create),
        Next::Kw(Keyword::Alter),
        Next::Kw(Keyword::Drop),
        Next::Kw(Keyword::Merge),
        Next::Kw(Keyword::With),
    ],
);

const SELECT_RULE: Rule = Rule(
    Cond::Kw(Keyword::Select),
    &[Next::Kw(Keyword::Distinct), Next::Kw(Keyword::All)],
);

const FROM_RULE: Rule = Rule(
    Cond::Seq(&[
        Cond::Kw(Keyword::Select),
        Cond::Many(&Cond::Not(&Cond::Any(&[
            Cond::Kw(Keyword::From),
            Cond::Kw(Keyword::Select),
        ]))),
        Cond::Any(&[
            SIMPLE_VALUE,
            Cond::Seq(&[
                Cond::Any(&[Cond::Kw(Keyword::Select), Cond::Kind(TokenKind::Comma)]),
                Cond::Op(OpTag::Mul),
            ]),
        ]),
    ]),
    &[Next::Kw(Keyword::From)],
);

const WHERE_RULE: Rule = Rule(
    Cond::Seq(&[
        Cond::Kw(Keyword::From),
        Cond::Many(&Cond::Not(&Cond::Any(&[
            Cond::Kw(Keyword::From),
            Cond::Kw(Keyword::Where),
        ]))),
        SIMPLE_VALUE,
    ]),
    &[Next::Kw(Keyword::Where)],
);

const ORDER_BY_RULE: Rule = Rule(Cond::Kw(Keyword::Order), &[Next::Kw(Keyword::By)]);

const ORDER_BY_SUGGEST_RULE: Rule = Rule(
    Cond::Seq(&[
        Cond::Not(&Cond::Any(&[
            Cond::Kw(Keyword::Select),
            Cond::Kw(Keyword::Where),
            Cond::Kw(Keyword::Group),
            Cond::Kw(Keyword::Order),
            Cond::Kw(Keyword::By),
            Cond::Kw(Keyword::Having),
            Cond::Kw(Keyword::With),
            Cond::Op(OpTag::And),
            Cond::Op(OpTag::Or),
            Cond::Kind(TokenKind::Dot),
        ])),
        SIMPLE_VALUE,
    ]),
    &[Next::KwSeq(&[Keyword::Order, Keyword::By])],
);

const ORDER_BY_DIR_RULE: Rule = Rule(
    Cond::Seq(&[
        Cond::Kw(Keyword::Order),
        Cond::Kw(Keyword::By),
        Cond::Many(&Cond::Not(&Cond::Any(&[
            Cond::Kw(Keyword::By),
            Cond::Kind(TokenKind::Comma),
            Cond::Kw(Keyword::Asc),
            Cond::Kw(Keyword::Desc),
            Cond::Kw(Keyword::Nulls),
            Cond::Kw(Keyword::Offset),
            Cond::Kw(Keyword::Fetch),
            Cond::Kw(Keyword::Limit),
        ]))),
        SIMPLE_VALUE,
    ]),
    &[
        Next::Kw(Keyword::Asc),
        Next::Kw(Keyword::Desc),
        Next::Kw(Keyword::Nulls),
    ],
);

const ORDER_BY_NULLS_PLACEMENT_RULE: Rule = Rule(
    Cond::Kw(Keyword::Nulls),
    &[Next::Kw(Keyword::First), Next::Kw(Keyword::Last)],
);

// Suggest ASC/DESC/NULLS after a column following a comma in ORDER BY
const ORDER_BY_DIR_AFTER_COMMA_RULE: Rule = Rule(
    Cond::Seq(&[
        Cond::Kw(Keyword::Order),
        Cond::Kw(Keyword::By),
        Cond::Many(&Cond::Not(&Cond::Any(&[
            Cond::Kw(Keyword::By),
            Cond::Kw(Keyword::Offset),
            Cond::Kw(Keyword::Fetch),
            Cond::Kw(Keyword::Limit),
            Cond::Kw(Keyword::Union),
            Cond::Kw(Keyword::Intersect),
            Cond::Kw(Keyword::Except),
        ]))),
        Cond::Kind(TokenKind::Comma),
        SIMPLE_VALUE,
    ]),
    &[
        Next::Kw(Keyword::Asc),
        Next::Kw(Keyword::Desc),
        Next::Kw(Keyword::Nulls),
    ],
);

const ORDER_BY_FOLLOW_RULE: Rule = Rule(
    Cond::Seq(&[
        Cond::Kw(Keyword::Order),
        Cond::Kw(Keyword::By),
        Cond::Many(&Cond::Not(&Cond::Any(&[
            Cond::Kw(Keyword::By),
            Cond::Kw(Keyword::Offset),
            Cond::Kw(Keyword::Fetch),
            Cond::Kw(Keyword::Limit),
            Cond::Kw(Keyword::Union),
            Cond::Kw(Keyword::Intersect),
            Cond::Kw(Keyword::Except),
        ]))),
        SIMPLE_VALUE,
    ]),
    &[Next::Kw(Keyword::Offset), Next::Kw(Keyword::Fetch)],
);

const LIMIT_RULE: Rule = Rule(
    Cond::Seq(&[
        Cond::Not(&Cond::Any(&[
            Cond::Kw(Keyword::Where),
            Cond::Kw(Keyword::Group),
            Cond::Kw(Keyword::Order),
            Cond::Kw(Keyword::By),
            Cond::Kw(Keyword::Having),
            Cond::Kw(Keyword::With),
            Cond::Op(OpTag::And),
            Cond::Op(OpTag::Or),
            Cond::Kind(TokenKind::Dot),
        ])),
        SIMPLE_VALUE,
    ]),
    &[Next::Kw(Keyword::Limit)],
);

const SUBQUERY_RULE: Rule = Rule(
    Cond::Kind(TokenKind::LeftParen),
    &[Next::Kw(Keyword::Select)],
);

const WITH_CTE_RULE: Rule = Rule(Cond::Kw(Keyword::With), &[Next::Kw(Keyword::Recursive)]);

const GROUP_BY_RULE: Rule = Rule(Cond::Kw(Keyword::Group), &[Next::Kw(Keyword::By)]);

const GROUP_BY_SUGGEST_RULE: Rule = Rule(
    Cond::Seq(&[
        Cond::Not(&Cond::Any(&[
            Cond::Kw(Keyword::Select),
            Cond::Kw(Keyword::Where),
            Cond::Kw(Keyword::Group),
            Cond::Kw(Keyword::Order),
            Cond::Kw(Keyword::By),
            Cond::Kw(Keyword::Having),
            Cond::Kw(Keyword::With),
            Cond::Op(OpTag::And),
            Cond::Op(OpTag::Or),
            Cond::Kind(TokenKind::Dot),
        ])),
        SIMPLE_VALUE,
    ]),
    &[Next::KwSeq(&[Keyword::Group, Keyword::By])],
);

const JOIN_RULE: Rule = Rule(
    Cond::Kw(Keyword::Join),
    &[Next::Kw(Keyword::On), Next::Kw(Keyword::Using)],
);

const INNER_JOIN_RULE: Rule = Rule(Cond::Kw(Keyword::Inner), &[Next::Kw(Keyword::Join)]);

const LEFT_JOIN_RULE: Rule = Rule(
    Cond::Kw(Keyword::Left),
    &[Next::Kw(Keyword::Join), Next::Kw(Keyword::Outer)],
);

const RIGHT_JOIN_RULE: Rule = Rule(
    Cond::Kw(Keyword::Right),
    &[Next::Kw(Keyword::Join), Next::Kw(Keyword::Outer)],
);

const FULL_JOIN_RULE: Rule = Rule(
    Cond::Kw(Keyword::Full),
    &[Next::Kw(Keyword::Join), Next::Kw(Keyword::Outer)],
);

const CROSS_JOIN_RULE: Rule = Rule(Cond::Kw(Keyword::Cross), &[Next::Kw(Keyword::Join)]);

// Suggest JOIN keywords after a table in FROM clause
const JOIN_AFTER_FROM_RULE: Rule = Rule(
    Cond::Seq(&[
        Cond::Not(&Cond::Any(&[
            Cond::Kw(Keyword::Where),
            Cond::Kw(Keyword::Join),
            Cond::Kw(Keyword::Inner),
            Cond::Kw(Keyword::Left),
            Cond::Kw(Keyword::Right),
            Cond::Kw(Keyword::Full),
            Cond::Kw(Keyword::Cross),
            Cond::Kw(Keyword::Natural),
            Cond::Kw(Keyword::Group),
            Cond::Kw(Keyword::Order),
            Cond::Kw(Keyword::By),
            Cond::Kw(Keyword::Having),
            Cond::Kw(Keyword::Limit),
            Cond::Kw(Keyword::With),
            Cond::Kind(TokenKind::Dot),
        ])),
        Cond::Any(&[
            Cond::Kind(TokenKind::Identifier),
            Cond::Kind(TokenKind::RightParen),
        ]),
    ]),
    &[
        Next::Kw(Keyword::Join),
        Next::Kw(Keyword::Inner),
        Next::Kw(Keyword::Left),
        Next::Kw(Keyword::Right),
        Next::Kw(Keyword::Full),
        Next::Kw(Keyword::Cross),
        Next::Kw(Keyword::Natural),
    ],
);

// OUTER can follow LEFT, RIGHT, or FULL
const LEFT_OUTER_JOIN_RULE: Rule = Rule(
    Cond::Seq(&[Cond::Kw(Keyword::Left), Cond::Kw(Keyword::Outer)]),
    &[Next::Kw(Keyword::Join)],
);

const RIGHT_OUTER_JOIN_RULE: Rule = Rule(
    Cond::Seq(&[Cond::Kw(Keyword::Right), Cond::Kw(Keyword::Outer)]),
    &[Next::Kw(Keyword::Join)],
);

const FULL_OUTER_JOIN_RULE: Rule = Rule(
    Cond::Seq(&[Cond::Kw(Keyword::Full), Cond::Kw(Keyword::Outer)]),
    &[Next::Kw(Keyword::Join)],
);

// NATURAL can combine with join types
const NATURAL_JOIN_RULE: Rule = Rule(
    Cond::Kw(Keyword::Natural),
    &[
        Next::Kw(Keyword::Join),
        Next::Kw(Keyword::Inner),
        Next::Kw(Keyword::Left),
        Next::Kw(Keyword::Right),
        Next::Kw(Keyword::Full),
    ],
);

const HAVING_RULE: Rule = Rule(Cond::Kw(Keyword::Group), &[Next::KwSeq(&[Keyword::By])]);

const HAVING_AFTER_GROUP_BY_RULE: Rule = Rule(
    Cond::Seq(&[
        Cond::Kw(Keyword::Group),
        Cond::Kw(Keyword::By),
        Cond::Many(&Cond::Not(&Cond::Any(&[
            Cond::Kw(Keyword::By),
            Cond::Kw(Keyword::Group),
            Cond::Kw(Keyword::Order),
            Cond::Kw(Keyword::Limit),
            Cond::Kw(Keyword::Having),
            Cond::Kw(Keyword::Union),
            Cond::Kw(Keyword::Intersect),
            Cond::Kw(Keyword::Except),
            Cond::Kw(Keyword::Offset),
            Cond::Kw(Keyword::Fetch),
        ]))),
        SIMPLE_VALUE,
    ]),
    &[Next::Kw(Keyword::Having)],
);

const INSERT_INTO_RULE: Rule = Rule(Cond::Kw(Keyword::Insert), &[Next::Kw(Keyword::Into)]);

const CREATE_TABLE_RULE: Rule = Rule(
    Cond::Kw(Keyword::Create),
    &[
        Next::Kw(Keyword::Table),
        Next::Kw(Keyword::View),
        Next::Kw(Keyword::Schema),
    ],
);

const PRIMARY_KEY_RULE: Rule = Rule(Cond::Kw(Keyword::Primary), &[Next::KwSeq(&[Keyword::Key])]);

const FOREIGN_KEY_RULE: Rule = Rule(Cond::Kw(Keyword::Foreign), &[Next::KwSeq(&[Keyword::Key])]);

// CASE statement rules
// Suggest WHEN after CASE keyword
const CASE_WHEN_RULE: Rule = Rule(Cond::Kw(Keyword::Case), &[Next::Kw(Keyword::When)]);

// Suggest THEN after a value following WHEN
const WHEN_THEN_RULE: Rule = Rule(
    Cond::Seq(&[
        Cond::Kw(Keyword::When),
        Cond::Many(&Cond::Not(&Cond::Any(&[
            Cond::Kw(Keyword::When),
            Cond::Kw(Keyword::Then),
            Cond::Kw(Keyword::Else),
            Cond::Kw(Keyword::End),
        ]))),
        SIMPLE_VALUE,
    ]),
    &[Next::Kw(Keyword::Then)],
);

// Suggest WHEN, ELSE, END after a value following THEN
const THEN_FOLLOW_RULE: Rule = Rule(
    Cond::Seq(&[
        Cond::Kw(Keyword::Then),
        Cond::Many(&Cond::Not(&Cond::Any(&[
            Cond::Kw(Keyword::When),
            Cond::Kw(Keyword::Then),
            Cond::Kw(Keyword::Else),
            Cond::Kw(Keyword::End),
        ]))),
        SIMPLE_VALUE,
    ]),
    &[
        Next::Kw(Keyword::When),
        Next::Kw(Keyword::Else),
        Next::Kw(Keyword::End),
    ],
);

// Suggest END after a value following ELSE
const ELSE_END_RULE: Rule = Rule(
    Cond::Seq(&[
        Cond::Kw(Keyword::Else),
        Cond::Many(&Cond::Not(&Cond::Any(&[
            Cond::Kw(Keyword::Else),
            Cond::Kw(Keyword::End),
        ]))),
        SIMPLE_VALUE,
    ]),
    &[Next::Kw(Keyword::End)],
);

const SET_OPERATION_ALL_RULE: Rule = Rule(
    Cond::Any(&[
        Cond::Kw(Keyword::Union),
        Cond::Kw(Keyword::Intersect),
        Cond::Kw(Keyword::Except),
    ]),
    &[Next::KwSeq(&[Keyword::All])],
);

const SET_OP_RULE: Rule = Rule(
    Cond::Seq(&[
        Cond::Not(&Cond::Any(&[
            Cond::Kw(Keyword::Select),
            Cond::Kw(Keyword::Where),
            Cond::Kw(Keyword::Group),
            Cond::Kw(Keyword::Order),
            Cond::Kw(Keyword::Having),
            Cond::Kw(Keyword::With),
            Cond::Kind(TokenKind::Comma),
            Cond::Op(OpTag::And),
            Cond::Op(OpTag::Or),
            Cond::Kind(TokenKind::Dot),
        ])),
        SIMPLE_VALUE,
    ]),
    &[
        Next::Kw(Keyword::Union),
        Next::Kw(Keyword::Intersect),
        Next::Kw(Keyword::Except),
    ],
);

#[cfg(test)]
mod tests {
    use crate::test_util::ansi_tokens;

    use super::*;

    #[test]
    fn start_rule() {
        // Should match at the start
        assert_matches(true, START_RULE, "");
        assert_matches(false, START_RULE, "SELECT");

        // Should match after semicolon
        assert_matches(true, START_RULE, ";");
        assert_matches(true, START_RULE, ";SE");

        // Should match after CTE definition
        assert_matches(true, START_RULE, "WITH cte (SELECT a FROM b) ");
        assert_matches(true, START_RULE, "WITH cte (SELECT a FROM b) S");

        // Should match after multiple CTEs
        assert_matches(
            true,
            START_RULE,
            "WITH cte1 (SELECT a FROM b), cte2 (SELECT c FROM d) ",
        );

        // Should match after nested subquery within CTE
        assert_matches(
            true,
            START_RULE,
            "WITH cte (SELECT a FROM (SELECT b FROM c)) ",
        );

        // Should NOT match after table subqueries (not a CTE context)
        assert_matches(false, START_RULE, "SELECT * FROM (SELECT a FROM b) ");
        assert_matches(false, START_RULE, "SELECT * FROM (SELECT a FROM b) t ");

        // Should NOT match after WHERE clause subqueries
        assert_matches(
            false,
            START_RULE,
            "SELECT * FROM t WHERE id IN (SELECT id FROM users) ",
        );
    }

    #[test]
    fn qualified_column_exclusion() {
        // After qualified columns (table.column), should NOT suggest clause keywords
        // Should suggest operators/AND/OR instead

        // Should NOT suggest LIMIT after qualified column
        assert_matches(
            false,
            LIMIT_RULE,
            "SELECT data FROM ingest WHERE ingest.data ",
        );

        // Should NOT suggest ORDER BY after qualified column
        assert_matches(
            false,
            ORDER_BY_SUGGEST_RULE,
            "SELECT data FROM ingest WHERE ingest.data ",
        );

        // Should NOT suggest GROUP BY after qualified column
        assert_matches(
            false,
            GROUP_BY_SUGGEST_RULE,
            "SELECT data FROM ingest WHERE ingest.data ",
        );

        // Should NOT suggest JOIN after qualified column
        assert_matches(
            false,
            JOIN_AFTER_FROM_RULE,
            "SELECT data FROM ingest WHERE ingest.data ",
        );

        // Should NOT suggest set operations after qualified column
        assert_matches(
            false,
            SET_OP_RULE,
            "SELECT data FROM ingest WHERE ingest.data ",
        );

        // But unqualified columns should still work
        assert_matches(true, LIMIT_RULE, "SELECT data FROM ingest WHERE data = 1 ");
        assert_matches(
            true,
            ORDER_BY_SUGGEST_RULE,
            "SELECT data FROM ingest WHERE data = 1 ",
        );
    }

    #[test]
    fn select_rule() {
        assert_matches(true, SELECT_RULE, "SELECT ");
        assert_matches(false, SELECT_RULE, "SELECT DISTINCT");
        assert_matches(false, SELECT_RULE, "SELECT ALL");
        assert_matches(false, SELECT_RULE, "FROM");
        assert_matches(false, SELECT_RULE, "");
        assert_matches(false, SELECT_RULE, "SELECT column");
    }

    #[test]
    fn from_rule() {
        assert_matches(true, FROM_RULE, "SELECT a ");
        assert_matches(true, FROM_RULE, "SELECT column1, column2 ");
        assert_matches(true, FROM_RULE, "SELECT (1 + 2) ");
        assert_matches(true, FROM_RULE, "SELECT * ");
        assert_matches(true, FROM_RULE, "SELECT a, * ");
        assert_matches(true, FROM_RULE, "SELECT a + b ");
        assert_matches(true, FROM_RULE, "SELECT 1 * 2 ");
        assert_matches(false, FROM_RULE, "SELECT 1 as ");
        assert_matches(false, FROM_RULE, "SELECT 1 * ");
        assert_matches(false, FROM_RULE, "SELECT a FROM");
        assert_matches(false, FROM_RULE, "SELECT a FROM users");
        assert_matches(false, FROM_RULE, "");
    }

    #[test]
    fn where_rule() {
        assert_matches(true, WHERE_RULE, "SELECT a FROM users ");
        assert_matches(true, WHERE_RULE, "SELECT a FROM table1, table2 ");
        assert_matches(true, WHERE_RULE, "SELECT a FROM (SELECT b FROM c) ");
        assert_matches(false, WHERE_RULE, "SELECT a FROM users WHERE");
        assert_matches(false, WHERE_RULE, "SELECT a FROM users WHERE id = 1");
        assert_matches(false, WHERE_RULE, "");
        assert_matches(false, WHERE_RULE, "SELECT a ");
    }

    #[test]
    fn if_match_limit() {
        let rule = LIMIT_RULE;

        assert_matches(true, rule, "SELECT a FROM users ");
        assert_matches(false, rule, "SELECT a FROM users WHERE");
        assert_matches(false, rule, "SELECT a FROM users WHERE ");
        assert_matches(false, rule, "SELECT a FROM users WHERE id");
        assert_matches(false, rule, "SELECT a FROM users WHERE id =");
        assert_matches(true, rule, "SELECT a FROM users WHERE id = 1 ");
        assert_matches(true, rule, "SELECT a FROM users WHERE name = 'John' ");
        assert_matches(false, rule, "SELECT a FROM users WHERE name = 'John' AND");
        assert_matches(
            false,
            rule,
            "SELECT a FROM users WHERE name = 'John' AND age",
        );
    }

    #[test]
    fn subquery_rule() {
        assert_matches(true, SUBQUERY_RULE, "(");
        assert_matches(true, SUBQUERY_RULE, "SELECT a FROM (");
        assert_matches(false, SUBQUERY_RULE, "");
        assert_matches(false, SUBQUERY_RULE, "SELECT");
        assert_matches(false, SUBQUERY_RULE, ")");
    }

    #[test]
    fn with_cte_rule() {
        assert_matches(true, WITH_CTE_RULE, "WITH ");
        assert_matches(false, WITH_CTE_RULE, "WITH RECURSIVE");
        assert_matches(false, WITH_CTE_RULE, "");
        assert_matches(false, WITH_CTE_RULE, "SELECT");
        assert_matches(false, WITH_CTE_RULE, "RECURSIVE");
        assert_matches(false, WITH_CTE_RULE, "WITH cte_name");
    }

    #[test]
    fn order_by_rule() {
        assert_matches(true, ORDER_BY_RULE, "ORDER ");
        assert_matches(false, ORDER_BY_RULE, "ORDER BY");
        assert_matches(false, ORDER_BY_RULE, "");
        assert_matches(false, ORDER_BY_RULE, "SELECT");
        assert_matches(false, ORDER_BY_RULE, "BY");
        assert_matches(false, ORDER_BY_RULE, "ORDER id");
    }

    #[test]
    fn order_by_suggest_rule() {
        let rule = ORDER_BY_SUGGEST_RULE;

        // Should suggest ORDER BY after complete SELECT/FROM/WHERE clauses
        assert_matches(true, rule, "SELECT a FROM users ");
        assert_matches(true, rule, "SELECT a FROM users WHERE id = 1 ");
        assert_matches(true, rule, "SELECT a FROM users WHERE name = 'John' ");

        // Should NOT suggest after incomplete clauses
        assert_matches(false, rule, "SELECT ");
        assert_matches(false, rule, "SELECT a");
        assert_matches(false, rule, "SELECT a, ");
        assert_matches(false, rule, "SELECT a FROM users WHERE");
        assert_matches(false, rule, "SELECT a FROM users WHERE ");
        assert_matches(false, rule, "SELECT a FROM users WHERE id");
        assert_matches(false, rule, "SELECT a FROM users WHERE id =");
        assert_matches(false, rule, "SELECT a FROM users WHERE name = 'John' AND");
        assert_matches(
            false,
            rule,
            "SELECT a FROM users WHERE name = 'John' AND age",
        );

        // Should NOT suggest after clause keywords
        assert_matches(false, rule, "SELECT a FROM users GROUP");
        assert_matches(false, rule, "SELECT a FROM users ORDER");
        assert_matches(false, rule, "SELECT a FROM users HAVING");

        // Should NOT suggest after GROUP BY (BY keyword is excluded to prevent suggesting after ORDER BY)
        assert_matches(false, rule, "SELECT COUNT(*) FROM users GROUP BY name ");

        // Should NOT suggest after ORDER BY is already used
        assert_matches(false, rule, "SELECT a FROM users ORDER BY name ");
    }

    #[test]
    fn group_by_rule() {
        assert_matches(true, GROUP_BY_RULE, "GROUP ");
        assert_matches(false, GROUP_BY_RULE, "GROUP BY");
        assert_matches(false, GROUP_BY_RULE, "");
        assert_matches(false, GROUP_BY_RULE, "SELECT");
        assert_matches(false, GROUP_BY_RULE, "BY");
    }

    #[test]
    fn group_by_suggest_rule() {
        let rule = GROUP_BY_SUGGEST_RULE;

        // Should suggest GROUP BY after complete SELECT/FROM/WHERE clauses
        assert_matches(true, rule, "SELECT a FROM users ");
        assert_matches(true, rule, "SELECT a FROM users WHERE id = 1 ");
        assert_matches(true, rule, "SELECT a FROM users WHERE name = 'John' ");

        // Should NOT suggest after incomplete clauses
        assert_matches(false, rule, "SELECT ");
        assert_matches(false, rule, "SELECT a");
        assert_matches(false, rule, "SELECT a, ");
        assert_matches(false, rule, "SELECT a FROM users WHERE");
        assert_matches(false, rule, "SELECT a FROM users WHERE ");
        assert_matches(false, rule, "SELECT a FROM users WHERE id");
        assert_matches(false, rule, "SELECT a FROM users WHERE id =");
        assert_matches(false, rule, "SELECT a FROM users WHERE name = 'John' AND");
        assert_matches(
            false,
            rule,
            "SELECT a FROM users WHERE name = 'John' AND age",
        );

        // Should NOT suggest after GROUP/ORDER/HAVING keywords
        assert_matches(false, rule, "SELECT a FROM users GROUP");
        assert_matches(false, rule, "SELECT a FROM users ORDER");
        assert_matches(false, rule, "SELECT a FROM users HAVING");
    }

    #[test]
    fn having_after_group_by_rule() {
        let rule = HAVING_AFTER_GROUP_BY_RULE;

        // Should suggest HAVING after GROUP BY with column
        assert_matches(true, rule, "SELECT COUNT(*) FROM users GROUP BY name ");
        assert_matches(true, rule, "SELECT a FROM users GROUP BY col1, col2 ");

        // Should NOT suggest immediately after GROUP BY keywords
        assert_matches(false, rule, "SELECT a FROM users GROUP");
        assert_matches(false, rule, "SELECT a FROM users GROUP ");
        assert_matches(false, rule, "SELECT a FROM users GROUP BY");
        assert_matches(false, rule, "SELECT a FROM users GROUP BY ");

        // Should NOT suggest after HAVING is already used
        assert_matches(
            false,
            rule,
            "SELECT a FROM users GROUP BY name HAVING COUNT(*) > 1 ",
        );

        // Should NOT suggest after ORDER BY or LIMIT
        assert_matches(
            false,
            rule,
            "SELECT a FROM users GROUP BY name ORDER BY name ",
        );
        assert_matches(false, rule, "SELECT a FROM users GROUP BY name LIMIT 10 ");
    }

    #[test]
    fn join_rule() {
        assert_matches(true, JOIN_RULE, "SELECT * FROM users JOIN ");
        assert_matches(false, JOIN_RULE, "SELECT * FROM users JOIN posts ON");
        assert_matches(false, JOIN_RULE, "");
        assert_matches(false, JOIN_RULE, "SELECT");
    }

    #[test]
    fn inner_join_rule() {
        assert_matches(true, INNER_JOIN_RULE, "INNER ");
        assert_matches(false, INNER_JOIN_RULE, "INNER JOIN");
        assert_matches(false, INNER_JOIN_RULE, "");
        assert_matches(false, INNER_JOIN_RULE, "JOIN");
    }

    #[test]
    fn left_join_rule() {
        // Should suggest both JOIN and OUTER after LEFT
        assert_matches(true, LEFT_JOIN_RULE, "LEFT ");
        assert_matches(false, LEFT_JOIN_RULE, "LEFT JOIN");
        assert_matches(false, LEFT_JOIN_RULE, "LEFT OUTER");
        assert_matches(false, LEFT_JOIN_RULE, "");
    }

    #[test]
    fn right_join_rule() {
        // Should suggest both JOIN and OUTER after RIGHT
        assert_matches(true, RIGHT_JOIN_RULE, "RIGHT ");
        assert_matches(false, RIGHT_JOIN_RULE, "RIGHT JOIN");
        assert_matches(false, RIGHT_JOIN_RULE, "RIGHT OUTER");
        assert_matches(false, RIGHT_JOIN_RULE, "");
    }

    #[test]
    fn full_join_rule() {
        // Should suggest both JOIN and OUTER after FULL
        assert_matches(true, FULL_JOIN_RULE, "FULL ");
        assert_matches(false, FULL_JOIN_RULE, "FULL JOIN");
        assert_matches(false, FULL_JOIN_RULE, "FULL OUTER");
        assert_matches(false, FULL_JOIN_RULE, "");
    }

    #[test]
    fn cross_join_rule() {
        assert_matches(true, CROSS_JOIN_RULE, "CROSS ");
        assert_matches(false, CROSS_JOIN_RULE, "CROSS JOIN");
        assert_matches(false, CROSS_JOIN_RULE, "");
    }

    #[test]
    fn insert_into_rule() {
        assert_matches(true, INSERT_INTO_RULE, "INSERT ");
        assert_matches(false, INSERT_INTO_RULE, "INSERT INTO");
        assert_matches(false, INSERT_INTO_RULE, "");
        assert_matches(false, INSERT_INTO_RULE, "INTO");
    }

    #[test]
    fn create_table_rule() {
        assert_matches(true, CREATE_TABLE_RULE, "CREATE ");
        assert_matches(false, CREATE_TABLE_RULE, "CREATE TABLE");
        assert_matches(false, CREATE_TABLE_RULE, "CREATE VIEW");
        assert_matches(false, CREATE_TABLE_RULE, "");
        assert_matches(false, CREATE_TABLE_RULE, "TABLE");
    }

    #[test]
    fn primary_key_rule() {
        assert_matches(true, PRIMARY_KEY_RULE, "PRIMARY ");
        assert_matches(false, PRIMARY_KEY_RULE, "PRIMARY KEY");
        assert_matches(false, PRIMARY_KEY_RULE, "");
        assert_matches(false, PRIMARY_KEY_RULE, "KEY");
    }

    #[test]
    fn foreign_key_rule() {
        assert_matches(true, FOREIGN_KEY_RULE, "FOREIGN ");
        assert_matches(false, FOREIGN_KEY_RULE, "FOREIGN KEY");
        assert_matches(false, FOREIGN_KEY_RULE, "");
        assert_matches(false, FOREIGN_KEY_RULE, "KEY");
    }

    #[test]
    fn set_operation_all_rule() {
        assert_matches(true, SET_OPERATION_ALL_RULE, "SELECT a FROM users UNION ");
        assert_matches(
            true,
            SET_OPERATION_ALL_RULE,
            "SELECT a FROM users INTERSECT ",
        );
        assert_matches(true, SET_OPERATION_ALL_RULE, "SELECT a FROM users EXCEPT ");
        assert_matches(
            false,
            SET_OPERATION_ALL_RULE,
            "SELECT a FROM users UNION ALL",
        );
        assert_matches(false, SET_OPERATION_ALL_RULE, "");
    }

    #[test]
    fn set_op_rule() {
        let rule = SET_OP_RULE;

        // Should suggest set operations after a complete SELECT statement
        assert_matches(true, rule, "SELECT a FROM users ");
        assert_matches(true, rule, "SELECT a FROM users WHERE id = 1 ");
        assert_matches(true, rule, "SELECT a FROM users ORDER BY name ");

        // Should NOT suggest after incomplete clauses
        assert_matches(false, rule, "SELECT a FROM users WHERE");
        assert_matches(false, rule, "SELECT a FROM users WHERE ");
        assert_matches(false, rule, "SELECT a FROM users WHERE id");
        assert_matches(false, rule, "SELECT a FROM users WHERE id =");
        assert_matches(false, rule, "SELECT a FROM users WHERE name = 'John' AND");
        assert_matches(
            false,
            rule,
            "SELECT a FROM users WHERE name = 'John' AND age",
        );

        // Should NOT suggest in the middle of SELECT list
        assert_matches(false, rule, "SELECT ");
        assert_matches(false, rule, "SELECT a");
        assert_matches(false, rule, "SELECT a, ");
    }

    #[test]
    fn join_after_from_rule() {
        let rule = JOIN_AFTER_FROM_RULE;

        // Should suggest JOIN types after table name in FROM clause
        assert_matches(true, rule, "SELECT * FROM users ");
        assert_matches(true, rule, "SELECT * FROM table1 ");
        assert_matches(true, rule, "SELECT * FROM (SELECT id FROM t) ");

        // Should suggest JOIN after a table with alias
        assert_matches(true, rule, "SELECT * FROM users u ");

        // Should NOT suggest after WHERE or other clauses
        assert_matches(false, rule, "SELECT * FROM users WHERE");
        assert_matches(false, rule, "SELECT * FROM users WHERE ");
        assert_matches(false, rule, "SELECT * ");
        assert_matches(false, rule, "");

        // Should NOT suggest if already have JOIN keyword
        assert_matches(false, rule, "SELECT * FROM users JOIN");
        assert_matches(false, rule, "SELECT * FROM users INNER");
        assert_matches(false, rule, "SELECT * FROM users LEFT");
    }

    #[test]
    fn left_outer_join_rule() {
        assert_matches(
            true,
            LEFT_OUTER_JOIN_RULE,
            "SELECT * FROM users LEFT OUTER ",
        );
        assert_matches(
            false,
            LEFT_OUTER_JOIN_RULE,
            "SELECT * FROM users LEFT OUTER JOIN",
        );
        assert_matches(false, LEFT_OUTER_JOIN_RULE, "SELECT * FROM users LEFT ");
        assert_matches(false, LEFT_OUTER_JOIN_RULE, "");
    }

    #[test]
    fn right_outer_join_rule() {
        assert_matches(
            true,
            RIGHT_OUTER_JOIN_RULE,
            "SELECT * FROM users RIGHT OUTER ",
        );
        assert_matches(
            false,
            RIGHT_OUTER_JOIN_RULE,
            "SELECT * FROM users RIGHT OUTER JOIN",
        );
        assert_matches(false, RIGHT_OUTER_JOIN_RULE, "");
    }

    #[test]
    fn full_outer_join_rule() {
        assert_matches(
            true,
            FULL_OUTER_JOIN_RULE,
            "SELECT * FROM users FULL OUTER ",
        );
        assert_matches(
            false,
            FULL_OUTER_JOIN_RULE,
            "SELECT * FROM users FULL OUTER JOIN",
        );
        assert_matches(false, FULL_OUTER_JOIN_RULE, "");
    }

    #[test]
    fn natural_join_rule() {
        // Should suggest JOIN types after NATURAL
        assert_matches(true, NATURAL_JOIN_RULE, "SELECT * FROM users NATURAL ");
        assert_matches(false, NATURAL_JOIN_RULE, "SELECT * FROM users NATURAL JOIN");
        assert_matches(
            false,
            NATURAL_JOIN_RULE,
            "SELECT * FROM users NATURAL INNER",
        );
        assert_matches(false, NATURAL_JOIN_RULE, "SELECT * FROM users NATURAL LEFT");
        assert_matches(false, NATURAL_JOIN_RULE, "");
    }

    #[test]
    fn order_by_dir_rule() {
        let rule = ORDER_BY_DIR_RULE;

        // Should suggest ASC, DESC, NULLS after ORDER BY columnname
        assert_matches(true, rule, "SELECT a FROM users ORDER BY name ");
        assert_matches(true, rule, "SELECT a FROM users ORDER BY id ");
        assert_matches(true, rule, "SELECT a, b FROM users ORDER BY a ");

        // Should NOT suggest after ASC/DESC/NULLS is already used
        assert_matches(false, rule, "SELECT a FROM users ORDER BY name ASC ");
        assert_matches(false, rule, "SELECT a FROM users ORDER BY name DESC ");
        assert_matches(false, rule, "SELECT a FROM users ORDER BY name NULLS ");

        // Should NOT suggest before BY
        assert_matches(false, rule, "SELECT a FROM users ORDER ");
        assert_matches(false, rule, "SELECT a FROM users ORDER BY ");

        // Should NOT match columns after commas (that's ORDER_BY_DIR_AFTER_COMMA_RULE's job)
        assert_matches(false, rule, "SELECT a FROM users ORDER BY name ASC, id ");
    }

    #[test]
    fn order_by_dir_after_comma_rule() {
        let rule = ORDER_BY_DIR_AFTER_COMMA_RULE;

        // Should suggest ASC, DESC, NULLS after column following comma in ORDER BY
        assert_matches(true, rule, "SELECT a FROM users ORDER BY name ASC, id ");
        assert_matches(true, rule, "SELECT a FROM users ORDER BY a, b ");
        assert_matches(true, rule, "SELECT a FROM users ORDER BY a DESC, b ASC, c ");

        // Should NOT suggest after first column (that's ORDER_BY_DIR_RULE's job)
        assert_matches(false, rule, "SELECT a FROM users ORDER BY name ");

        // Should NOT suggest after ASC/DESC/NULLS following the comma
        assert_matches(
            false,
            rule,
            "SELECT a FROM users ORDER BY name ASC, id ASC ",
        );
        assert_matches(false, rule, "SELECT a FROM users ORDER BY name, id DESC ");

        // Should NOT suggest after commas in SELECT list or WHERE clause
        assert_matches(false, rule, "SELECT a, b ");
        assert_matches(false, rule, "SELECT a, b FROM users");
        assert_matches(false, rule, "SELECT a FROM users WHERE id IN (1, 2 ");
    }

    #[test]
    fn subquery_tests() {
        // Test that LIMIT_RULE works in subqueries
        assert_matches(true, LIMIT_RULE, "SELECT * FROM (SELECT a FROM users ");
        assert_matches(
            true,
            LIMIT_RULE,
            "SELECT * FROM (SELECT a FROM users WHERE id = 1 ",
        );

        // Test that ORDER_BY_SUGGEST_RULE works in subqueries
        assert_matches(
            true,
            ORDER_BY_SUGGEST_RULE,
            "SELECT * FROM (SELECT a FROM users ",
        );
        assert_matches(
            true,
            ORDER_BY_SUGGEST_RULE,
            "SELECT * FROM (SELECT a FROM users WHERE id = 1 ",
        );

        // Test that ORDER_BY_DIR_RULE works in subqueries
        assert_matches(
            true,
            ORDER_BY_DIR_RULE,
            "SELECT * FROM (SELECT a FROM users ORDER BY name ",
        );
        assert_matches(
            false,
            ORDER_BY_DIR_RULE,
            "SELECT * FROM (SELECT a FROM users ORDER BY name ASC ",
        );

        // Test that ORDER_BY_DIR_AFTER_COMMA_RULE works in subqueries
        assert_matches(
            true,
            ORDER_BY_DIR_AFTER_COMMA_RULE,
            "SELECT * FROM (SELECT a FROM users ORDER BY name ASC, id ",
        );

        // Test that GROUP_BY_SUGGEST_RULE works in subqueries
        assert_matches(
            true,
            GROUP_BY_SUGGEST_RULE,
            "SELECT * FROM (SELECT COUNT(*) FROM users ",
        );

        // Test that SET_OP_RULE works in subqueries
        assert_matches(true, SET_OP_RULE, "SELECT * FROM (SELECT a FROM users ");
        assert_matches(
            true,
            SET_OP_RULE,
            "SELECT * FROM (SELECT a FROM users ORDER BY name ",
        );

        // Test that JOIN_AFTER_FROM_RULE works in subqueries
        assert_matches(
            true,
            JOIN_AFTER_FROM_RULE,
            "SELECT * FROM (SELECT * FROM users ",
        );

        // Test that outer query rules match after closing paren of subquery
        // (The closing paren acts like a table name for the outer query)
        assert_matches(true, LIMIT_RULE, "SELECT * FROM (SELECT a FROM users) ");
        assert_matches(
            true,
            ORDER_BY_SUGGEST_RULE,
            "SELECT * FROM (SELECT a FROM users) ",
        );
        assert_matches(
            true,
            JOIN_AFTER_FROM_RULE,
            "SELECT * FROM (SELECT a FROM users) ",
        );
        assert_matches(true, SET_OP_RULE, "SELECT * FROM (SELECT a FROM users) ");

        // ORDER_BY_DIR_RULE SHOULD match after closing paren if there's ORDER BY before it
        // This handles cases like: ORDER BY (expression) or ORDER BY (subquery_column)
        assert_matches(
            true,
            ORDER_BY_DIR_RULE,
            "SELECT * FROM (SELECT a FROM users ORDER BY name) ",
        );

        // Test nested ORDER BY - inner query with ORDER BY
        assert_matches(
            true,
            ORDER_BY_DIR_RULE,
            "SELECT * FROM t ORDER BY (SELECT name FROM users ",
        );
    }

    #[test]
    fn case_when_rule() {
        let rule = CASE_WHEN_RULE;

        // Should suggest WHEN after CASE
        assert_matches(true, rule, "SELECT CASE ");
        assert_matches(true, rule, "SELECT a, CASE ");
        assert_matches(true, rule, "SELECT a FROM users WHERE status = CASE ");

        // Should NOT suggest after WHEN is already used
        assert_matches(false, rule, "SELECT CASE WHEN ");
        assert_matches(false, rule, "SELECT CASE WHEN a = 1 ");
    }

    #[test]
    fn when_then_rule() {
        let rule = WHEN_THEN_RULE;

        // Should suggest THEN after WHEN with a value
        assert_matches(true, rule, "SELECT CASE WHEN a = 1 ");
        assert_matches(true, rule, "SELECT CASE WHEN status = 'active' ");
        assert_matches(true, rule, "SELECT CASE WHEN (a > 0) ");
        // Single column is valid for boolean columns
        assert_matches(true, rule, "SELECT CASE WHEN is_active ");

        // Should NOT suggest before value is provided
        assert_matches(false, rule, "SELECT CASE WHEN ");
        // Should NOT suggest in the middle of an expression
        assert_matches(false, rule, "SELECT CASE WHEN a =");

        // Should NOT suggest after THEN is already used
        assert_matches(false, rule, "SELECT CASE WHEN a = 1 THEN ");
        assert_matches(false, rule, "SELECT CASE WHEN a = 1 THEN result ");
    }

    #[test]
    fn then_follow_rule() {
        let rule = THEN_FOLLOW_RULE;

        // Should suggest WHEN, ELSE, END after THEN with a value
        assert_matches(true, rule, "SELECT CASE WHEN a = 1 THEN result ");
        assert_matches(true, rule, "SELECT CASE WHEN a = 1 THEN 'yes' ");
        assert_matches(true, rule, "SELECT CASE WHEN a = 1 THEN (SELECT x FROM t) ");

        // Should NOT suggest before value is provided
        assert_matches(false, rule, "SELECT CASE WHEN a = 1 THEN ");

        // Should work with multiple WHEN clauses
        assert_matches(
            true,
            rule,
            "SELECT CASE WHEN a = 1 THEN 'one' WHEN a = 2 THEN 'two' ",
        );
    }

    #[test]
    fn else_end_rule() {
        let rule = ELSE_END_RULE;

        // Should suggest END after ELSE with a value
        assert_matches(true, rule, "SELECT CASE WHEN a = 1 THEN 'yes' ELSE 'no' ");
        assert_matches(true, rule, "SELECT CASE WHEN a = 1 THEN result ELSE other ");
        assert_matches(
            true,
            rule,
            "SELECT CASE WHEN a = 1 THEN (SELECT x FROM t) ELSE (SELECT y FROM t) ",
        );

        // Should NOT suggest before value is provided
        assert_matches(false, rule, "SELECT CASE WHEN a = 1 THEN result ELSE ");

        // Should NOT suggest after END is already used
        assert_matches(
            false,
            rule,
            "SELECT CASE WHEN a = 1 THEN 'yes' ELSE 'no' END ",
        );
    }

    #[test]
    fn case_statement_full_flow() {
        // Test complete CASE statement flow
        // Simple CASE with single WHEN
        assert_matches(true, CASE_WHEN_RULE, "SELECT CASE ");
        assert_matches(true, WHEN_THEN_RULE, "SELECT CASE WHEN status = 'active' ");
        assert_matches(
            true,
            THEN_FOLLOW_RULE,
            "SELECT CASE WHEN status = 'active' THEN 1 ",
        );
        assert_matches(
            true,
            ELSE_END_RULE,
            "SELECT CASE WHEN status = 'active' THEN 1 ELSE 0 ",
        );

        // CASE with multiple WHEN clauses
        assert_matches(true, THEN_FOLLOW_RULE, "SELECT CASE WHEN a = 1 THEN 'one' ");
        assert_matches(
            true,
            WHEN_THEN_RULE,
            "SELECT CASE WHEN a = 1 THEN 'one' WHEN a = 2 ",
        );
        assert_matches(
            true,
            THEN_FOLLOW_RULE,
            "SELECT CASE WHEN a = 1 THEN 'one' WHEN a = 2 THEN 'two' ",
        );

        // CASE in different contexts
        assert_matches(true, CASE_WHEN_RULE, "SELECT a, CASE ");
        assert_matches(true, CASE_WHEN_RULE, "SELECT a FROM users WHERE id = CASE ");
        assert_matches(true, CASE_WHEN_RULE, "SELECT a FROM users ORDER BY CASE ");
    }

    fn assert_matches(expected: bool, rule: Rule, sql: &str) {
        let tokens = ansi_tokens(sql);
        let kinds = tokens.iter().map(|t| t.kind).collect::<Vec<TokenKind>>();
        let kinds = &kinds[0..kinds.len().saturating_sub(1)]; // ignore the last token (EOF)
        let result = rule.0.scan_from(kinds, Dir::Bwd, kinds.len()).0;

        assert_eq!(
            result, expected,
            "\n\tevaluated to ({}) expected ({})\n\tinput: {:?}\n\trule {:?}",
            result, expected, sql, rule,
        );
    }
}
