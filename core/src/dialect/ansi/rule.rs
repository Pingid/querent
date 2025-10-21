use crate::dialect::RuleSet;
use crate::dialect::follow::{If, Rule, Then};
use crate::lex::{Keyword, OpTag, TokenKind};

const SIMPLE_VALUE: If = If::Any(&[
    If::Kind(TokenKind::Identifier),
    If::Kind(TokenKind::RightParen),
    If::Kind(TokenKind::Number),
    If::Kind(TokenKind::Str),
]);

#[cfg(test)]
use crate::dialect::follow::Direction;

pub const ANSI_RULE_SET: RuleSet = RuleSet(&[
    START_RULE,
    SELECT_RULE,
    FROM_RULE,
    WHERE_RULE,
    GROUP_BY_RULE,
    ORDER_BY_RULE,
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
    SET_OPERATION_ALL_RULE,
    SET_OP_RULE,
    LIMIT_FOLLOW_RULE,
]);

const START_RULE: Rule = Rule(
    If::Any(&[If::End, If::Kind(TokenKind::Semicolon)]),
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
);

const SELECT_RULE: Rule = Rule(
    If::Kw(Keyword::Select),
    &[Then::Kw(Keyword::Distinct), Then::Kw(Keyword::All)],
);

const FROM_RULE: Rule = Rule(
    If::Match(&[
        If::Kw(Keyword::Select),
        If::While(&If::Not(&If::Any(&[
            If::Kw(Keyword::From),
            If::Kw(Keyword::Select),
        ]))),
        If::Any(&[
            SIMPLE_VALUE,
            If::Match(&[
                If::Any(&[If::Kw(Keyword::Select), If::Kind(TokenKind::Comma)]),
                If::Op(OpTag::Mul),
            ]),
        ]),
    ]),
    &[Then::Kw(Keyword::From)],
);

const WHERE_RULE: Rule = Rule(
    If::Match(&[
        If::Kw(Keyword::From),
        If::While(&If::Not(&If::Any(&[
            If::Kw(Keyword::From),
            If::Kw(Keyword::Where),
        ]))),
        SIMPLE_VALUE,
    ]),
    &[Then::Kw(Keyword::Where)],
);

const LIMIT_RULE: Rule = Rule(
    If::Match(&[
        If::Not(&If::Until(&If::Any(&[
            If::Kw(Keyword::Limit),
            If::Kw(Keyword::Union),
            If::Kw(Keyword::Intersect),
            If::Kw(Keyword::Except),
            If::Kw(Keyword::Offset),
            If::Kw(Keyword::Fetch),
        ]))),
        SIMPLE_VALUE,
    ]),
    &[Then::Kw(Keyword::Limit)],
);

const SUBQUERY_RULE: Rule = Rule(If::Kind(TokenKind::LeftParen), &[Then::Kw(Keyword::Select)]);

const WITH_CTE_RULE: Rule = Rule(If::Kw(Keyword::With), &[Then::Kw(Keyword::Recursive)]);

const ORDER_BY_RULE: Rule = Rule(If::Kw(Keyword::Order), &[Then::Kw(Keyword::By)]);

const GROUP_BY_RULE: Rule = Rule(If::Kw(Keyword::Group), &[Then::Kw(Keyword::By)]);

const JOIN_RULE: Rule = Rule(
    If::Kw(Keyword::Join),
    &[Then::Kw(Keyword::On), Then::Kw(Keyword::Using)],
);

const INNER_JOIN_RULE: Rule = Rule(If::Kw(Keyword::Inner), &[Then::Kw(Keyword::Join)]);

const LEFT_JOIN_RULE: Rule = Rule(
    If::Kw(Keyword::Left),
    &[
        Then::CombinedKw(&[Keyword::Left, Keyword::Join]),
        Then::CombinedKw(&[Keyword::Left, Keyword::Outer, Keyword::Join]),
    ],
);

const RIGHT_JOIN_RULE: Rule = Rule(
    If::Kw(Keyword::Right),
    &[
        Then::CombinedKw(&[Keyword::Right, Keyword::Join]),
        Then::CombinedKw(&[Keyword::Right, Keyword::Outer, Keyword::Join]),
    ],
);

const FULL_JOIN_RULE: Rule = Rule(
    If::Kw(Keyword::Full),
    &[
        Then::CombinedKw(&[Keyword::Full, Keyword::Join]),
        Then::CombinedKw(&[Keyword::Full, Keyword::Outer, Keyword::Join]),
    ],
);

const CROSS_JOIN_RULE: Rule = Rule(If::Kw(Keyword::Cross), &[Then::Kw(Keyword::Join)]);

// Suggest JOIN keywords after a table in FROM clause
const JOIN_AFTER_FROM_RULE: Rule = Rule(
    If::Match(&[
        If::Kw(Keyword::From),
        If::While(&If::Not(&If::Any(&[
            If::Kw(Keyword::From),
            If::Kw(Keyword::Where),
            If::Kw(Keyword::Join),
            If::Kw(Keyword::Inner),
            If::Kw(Keyword::Left),
            If::Kw(Keyword::Right),
            If::Kw(Keyword::Full),
            If::Kw(Keyword::Cross),
            If::Kw(Keyword::Natural),
            If::Kw(Keyword::Group),
            If::Kw(Keyword::Order),
            If::Kw(Keyword::Limit),
            If::Kw(Keyword::Union),
            If::Kw(Keyword::Intersect),
            If::Kw(Keyword::Except),
            If::Kw(Keyword::Offset),
            If::Kw(Keyword::Fetch),
        ]))),
        If::Any(&[
            If::Kind(TokenKind::Identifier),
            If::Kind(TokenKind::RightParen),
        ]),
    ]),
    &[
        Then::CombinedKw(&[Keyword::Inner, Keyword::Join]),
        Then::CombinedKw(&[Keyword::Left, Keyword::Join]),
        Then::CombinedKw(&[Keyword::Right, Keyword::Join]),
        Then::CombinedKw(&[Keyword::Full, Keyword::Join]),
        Then::CombinedKw(&[Keyword::Cross, Keyword::Join]),
        Then::CombinedKw(&[Keyword::Natural, Keyword::Join]),
    ],
);

// OUTER can follow LEFT, RIGHT, or FULL
const LEFT_OUTER_JOIN_RULE: Rule = Rule(
    If::Match(&[If::Kw(Keyword::Left), If::Kw(Keyword::Outer)]),
    &[Then::Kw(Keyword::Join)],
);

const RIGHT_OUTER_JOIN_RULE: Rule = Rule(
    If::Match(&[If::Kw(Keyword::Right), If::Kw(Keyword::Outer)]),
    &[Then::Kw(Keyword::Join)],
);

const FULL_OUTER_JOIN_RULE: Rule = Rule(
    If::Match(&[If::Kw(Keyword::Full), If::Kw(Keyword::Outer)]),
    &[Then::Kw(Keyword::Join)],
);

// NATURAL can combine with join types
const NATURAL_JOIN_RULE: Rule = Rule(
    If::Kw(Keyword::Natural),
    &[
        Then::Kw(Keyword::Join),
        Then::CombinedKw(&[Keyword::Natural, Keyword::Left, Keyword::Join]),
        Then::CombinedKw(&[Keyword::Natural, Keyword::Right, Keyword::Join]),
        Then::CombinedKw(&[Keyword::Natural, Keyword::Full, Keyword::Join]),
        Then::CombinedKw(&[Keyword::Natural, Keyword::Inner, Keyword::Join]),
    ],
);

const HAVING_RULE: Rule = Rule(If::Kw(Keyword::Group), &[Then::CombinedKw(&[Keyword::By])]);

const HAVING_AFTER_GROUP_BY_RULE: Rule = Rule(
    If::Match(&[
        If::Kw(Keyword::Group),
        If::Kw(Keyword::By),
        If::While(&If::Not(&If::Any(&[
            If::Kw(Keyword::Order),
            If::Kw(Keyword::Limit),
            If::Kw(Keyword::Having),
            If::Kw(Keyword::Union),
            If::Kw(Keyword::Intersect),
            If::Kw(Keyword::Except),
            If::Kw(Keyword::Offset),
            If::Kw(Keyword::Fetch),
        ]))),
    ]),
    &[Then::Kw(Keyword::Having)],
);

const INSERT_INTO_RULE: Rule = Rule(If::Kw(Keyword::Insert), &[Then::Kw(Keyword::Into)]);

const CREATE_TABLE_RULE: Rule = Rule(
    If::Kw(Keyword::Create),
    &[
        Then::Kw(Keyword::Table),
        Then::Kw(Keyword::View),
        Then::Kw(Keyword::Schema),
    ],
);

const PRIMARY_KEY_RULE: Rule = Rule(
    If::Kw(Keyword::Primary),
    &[Then::CombinedKw(&[Keyword::Key])],
);

const FOREIGN_KEY_RULE: Rule = Rule(
    If::Kw(Keyword::Foreign),
    &[Then::CombinedKw(&[Keyword::Key])],
);

const SET_OPERATION_ALL_RULE: Rule = Rule(
    If::Any(&[
        If::Kw(Keyword::Union),
        If::Kw(Keyword::Intersect),
        If::Kw(Keyword::Except),
    ]),
    &[Then::CombinedKw(&[Keyword::All])],
);

const SET_OP_RULE: Rule = Rule(
    If::Match(&[
        If::Not(&If::Until(&If::Any(&[
            If::Kw(Keyword::Union),
            If::Kw(Keyword::Intersect),
            If::Kw(Keyword::Except),
        ]))),
        SIMPLE_VALUE,
    ]),
    &[
        Then::Kw(Keyword::Union),
        Then::Kw(Keyword::Intersect),
        Then::Kw(Keyword::Except),
    ],
);

const LIMIT_FOLLOW_RULE: Rule = Rule(
    If::Kw(Keyword::Limit),
    &[Then::Kw(Keyword::Offset), Then::Kw(Keyword::Fetch)],
);

#[cfg(test)]
mod tests {
    use crate::test_util::ansi_tokens;

    use super::*;

    #[test]
    fn start_rule() {
        assert_matches(true, START_RULE, "");
        assert_matches(false, START_RULE, "SELECT");
        assert_matches(true, START_RULE, ";");
        assert_matches(false, START_RULE, ";SE");
        assert_matches(false, START_RULE, "FROM");
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
        assert_matches(true, rule, "SELECT a FROM users WHERE name = 'John' ");
        assert_matches(true, rule, "SELECT a FROM users GROUP BY id ");
        assert_matches(
            true,
            rule,
            "SELECT a FROM users GROUP BY id HAVING count > 1 ",
        );
        assert_matches(true, rule, "SELECT a FROM users ORDER BY name ");
        assert_matches(
            false,
            rule,
            "SELECT a FROM users UNION SELECT b FROM others ",
        );
        assert_matches(false, rule, "SELECT a FROM users LIMIT ");
        assert_matches(false, rule, "SELECT a FROM users LIMIT 10");
        assert_matches(false, rule, "SELECT a FROM users OFFSET ");
        assert_matches(false, rule, "SELECT a FROM users OFFSET 5 ");
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
    fn group_by_rule() {
        assert_matches(true, GROUP_BY_RULE, "GROUP ");
        assert_matches(false, GROUP_BY_RULE, "GROUP BY");
        assert_matches(false, GROUP_BY_RULE, "");
        assert_matches(false, GROUP_BY_RULE, "SELECT");
        assert_matches(false, GROUP_BY_RULE, "BY");
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
        assert_matches(true, LEFT_JOIN_RULE, "LEFT ");
        assert_matches(false, LEFT_JOIN_RULE, "LEFT JOIN");
        assert_matches(false, LEFT_JOIN_RULE, "");
    }

    #[test]
    fn right_join_rule() {
        assert_matches(true, RIGHT_JOIN_RULE, "RIGHT ");
        assert_matches(false, RIGHT_JOIN_RULE, "RIGHT JOIN");
        assert_matches(false, RIGHT_JOIN_RULE, "");
    }

    #[test]
    fn full_join_rule() {
        assert_matches(true, FULL_JOIN_RULE, "FULL ");
        assert_matches(false, FULL_JOIN_RULE, "FULL JOIN");
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
    fn join_after_from_rule() {
        assert_matches(true, JOIN_AFTER_FROM_RULE, "SELECT * FROM users ");
        assert_matches(true, JOIN_AFTER_FROM_RULE, "SELECT * FROM table1 ");
        assert_matches(
            true,
            JOIN_AFTER_FROM_RULE,
            "SELECT * FROM (SELECT id FROM t) ",
        );
        assert_matches(false, JOIN_AFTER_FROM_RULE, "SELECT * FROM users WHERE");
        assert_matches(false, JOIN_AFTER_FROM_RULE, "SELECT * FROM users JOIN");
        assert_matches(false, JOIN_AFTER_FROM_RULE, "SELECT * FROM users LEFT");
        assert_matches(false, JOIN_AFTER_FROM_RULE, "SELECT * ");
        assert_matches(false, JOIN_AFTER_FROM_RULE, "");
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
        assert_matches(true, NATURAL_JOIN_RULE, "SELECT * FROM users NATURAL ");
        assert_matches(false, NATURAL_JOIN_RULE, "SELECT * FROM users NATURAL JOIN");
        assert_matches(
            false,
            NATURAL_JOIN_RULE,
            "SELECT * FROM users NATURAL INNER",
        );
        assert_matches(false, NATURAL_JOIN_RULE, "");
    }

    fn assert_matches(matches: bool, rule: Rule, sql: &str) {
        let tokens = ansi_tokens(sql);
        let kinds = tokens.iter().map(|t| t.kind).collect::<Vec<TokenKind>>();
        let kinds = &kinds[0..kinds.len().saturating_sub(1)]; // ignore the last token (EOF)
        let result = rule
            .0
            .match_consume(kinds, kinds.len().saturating_sub(1), Direction::Backward)
            .0;
        assert_eq!(
            result, matches,
            "SQL: {:?}, expected match: {}, got: {}",
            sql, matches, result
        );
    }
}
