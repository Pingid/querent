use smol_str::SmolStr;

use crate::complete::Completer;
use crate::complete::candidate::Candidate;
use crate::complete::candidate::CandidateBuilder;
use crate::complete::candidate::CandidateSet;
use crate::complete::context::Context;
use crate::dialect::rule::Next;
use crate::lex::Keyword;

pub struct KeywordProvider;
impl<'a> Completer<'a> for KeywordProvider {
    fn complete(&mut self, ctx: &mut Context<'a>, b: &mut CandidateSet<'a>) {
        for n in ctx.spec().resolve_follow_rules(&ctx.cursor().preceding) {
            b.push(n.to_candidate());
        }
    }
}

impl Next {
    fn to_candidate<'a>(self) -> Candidate<'a> {
        let (label, is_snippet) = self.label(true);
        let keyword = match self {
            Next::Kw(kw) => Some(kw),
            _ => None,
        };
        let mut builder = CandidateBuilder::keyword(label, keyword);
        if is_snippet {
            builder = builder.snippet();
        }
        builder.build()
    }

    fn label(self, capitilize: bool) -> (SmolStr, bool) {
        match self {
            Next::Kw(kw) => (fmt_kw(&kw, capitilize), false),
            Next::KwSeq(kws) => (
                kws.iter()
                    .map(|kw| fmt_kw(kw, true))
                    .collect::<Vec<_>>()
                    .join(" ".into())
                    .into(),
                false,
            ),
            Next::Seq(seq) => {
                let (labels, is_snippet) = seq
                    .iter()
                    .map(|next| next.label(capitilize))
                    .unzip::<_, _, Vec<_>, Vec<_>>();
                (
                    labels.join(" ".into()).into(),
                    is_snippet.iter().any(|b| *b),
                )
            }
            Next::Query => (SmolStr::from("($1)"), true),
        }
    }
}

fn fmt_kw(kw: &Keyword, capitilize: bool) -> SmolStr {
    let kw = format!("{:?}", kw);
    match capitilize {
        true => kw.to_ascii_uppercase().into(),
        false => kw.into(),
    }
}
