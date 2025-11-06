use super::util::FieldSetFormatter;
use super::util::fmt_list;
use super::util::some_eq;
use crate::complete::types::Completion;
use crate::complete::types::CompletionKind;

/// Trait for implementing completion matchers.
///
/// Matchers are used to verify that a set of completions meets certain criteria
/// during testing. Implementations should check the provided completions and
/// return an error if the criteria are not met.
pub trait Matcher: Send + Sync + 'static {
    fn check(&self, items: &[Completion]) -> Result<(), String>;
}

/// A collection of matchers that all must pass for a test to succeed.
///
/// This struct allows combining multiple matchers with AND semantics,
/// where all matchers must pass for the overall expectation to be met.
pub struct Expect(Vec<Box<dyn Matcher>>);
impl Expect {
    pub fn new() -> Self {
        Self(vec![])
    }
    pub fn and<M: Matcher>(mut self, m: M) -> Self {
        self.0.push(Box::new(m));
        self
    }
    pub fn check(&self, items: &[Completion]) -> Result<(), String> {
        for p in &self.0 {
            p.check(items)?;
        }
        Ok(())
    }
}

/// Matcher that expects no completions.
///
/// This matcher fails if any completions are present in the result set.
pub struct Empty;
impl Matcher for Empty {
    fn check(&self, items: &[Completion]) -> Result<(), String> {
        if items.len() > 0 {
            return Err(format!("Expected no completions, got {}", items.len()));
        }
        Ok(())
    }
}

/// Matcher that verifies the first N completions have specific labels.
///
/// This matcher checks that the completions start with the specified sequence
/// of labels in the exact order provided.
pub struct Starts(pub Vec<String>);
impl Matcher for Starts {
    fn check(&self, items: &[Completion]) -> Result<(), String> {
        let n = self.0.len();
        let head = &items[..items.len().min(n)]
            .iter()
            .map(|c| c.label.clone())
            .collect::<Vec<_>>();
        match head == &self.0 {
            true => Ok(()),
            false => Err(format!(
                "Expected to start with labels {:?}, got {:?}",
                self.0, head
            )),
        }
    }
}

/// Matcher that verifies specific completions are present.
///
/// This matcher checks that all specified candidate matches are present
/// somewhere in the completion list, regardless of order.
pub struct Contains(pub Vec<CandidateMatch>);
impl Matcher for Contains {
    fn check(&self, items: &[Completion]) -> Result<(), String> {
        for m in &self.0 {
            if !items.iter().any(|c| m.matches(c).is_ok()) {
                return missing(items, m);
            }
        }
        Ok(())
    }
}

/// Matcher that verifies completions appear in a specific order.
///
/// This matcher checks that all specified candidate matches are present
/// and appear in the same relative order as specified, though they don't
/// need to be consecutive.
pub struct InOrder(pub Vec<CandidateMatch>);
impl Matcher for InOrder {
    fn check(&self, items: &[Completion]) -> Result<(), String> {
        let mut last = None;
        for m in &self.0 {
            let idx = items.iter().position(|c| m.matches(c).is_ok());
            let Some(idx) = idx else {
                return missing(items, m);
            };
            if let Some(last) = last
                && idx < last
            {
                println!("idx: {}, last: {}, len: {}", idx, last, items.len());
                return out_of_order(items, idx, last);
            }
            last = Some(idx);
        }
        Ok(())
    }
}

fn missing(items: &[Completion], missing: &CandidateMatch) -> Result<(), String> {
    Err(format!("Missing ({})\ngot: {}", missing, fmt_list(&items),))
}

fn out_of_order(items: &[Completion], idx: usize, last: usize) -> Result<(), String> {
    Err(format!(
        "Out of order: expected ({}) after ({})\ngot: {}",
        items[idx],
        items[last],
        fmt_list(&items),
    ))
}

/// An expected completion candidate for testing.
///
/// This struct represents the expected properties of a completion item.
/// Each field is optional, allowing partial matching. Only the fields
/// that are `Some` will be checked against the actual completion.
#[derive(Debug, Default, Clone)]
pub struct CandidateMatch {
    pub label: Option<String>,
    pub kind: Option<CompletionKind>,
    pub detail: Option<String>,
    pub insert_text: Option<String>,
}

/// Macro for creating a `CandidateMatch` with specified fields.
///
/// This macro provides a convenient way to create partial candidate matches
/// for testing. Only the specified fields will be checked during matching.
///
/// # Example
/// ```ignore
/// candidate! {
///     label: "users",
///     kind: CompletionKind::Table,
///     detail: "public.users"
/// }
/// ```
#[macro_export]
macro_rules! candidate {
    ($($k:ident : $v:expr),* $(,)? ) => {
        crate::test_utils::CandidateMatch {
            $(
                $k: Some($v.to_string()),
            )*
            ..Default::default()
        }
    };
}

impl CandidateMatch {
    pub fn matches(&self, completion: &Completion) -> Result<(), String> {
        some_eq("label", self.label.as_ref(), Some(&completion.label))?;
        some_eq("kind", self.kind.as_ref(), Some(&completion.kind))?;
        some_eq("detail", self.detail.as_ref(), completion.detail.as_ref())?;
        some_eq(
            "insert_text",
            self.insert_text.as_ref(),
            Some(&completion.insert_text),
        )?;
        Ok(())
    }
}

impl From<&str> for CandidateMatch {
    fn from(s: &str) -> Self {
        Self {
            label: Some(s.to_string()),
            ..Default::default()
        }
    }
}

impl std::fmt::Display for Completion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = FieldSetFormatter::new();
        out.push("label", &self.label);
        out.push("kind", &self.kind);
        out.push_some("detail", self.detail.as_ref());
        out.push("insert_text", &self.insert_text);
        write!(f, "{{ {} }}", out.join(", "))
    }
}
impl std::fmt::Display for CandidateMatch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut out = FieldSetFormatter::new();
        out.push_some("label", self.label.as_ref());
        out.push_some("kind", self.kind.as_ref());
        out.push_some("detail", self.detail.as_ref());
        out.push_some("insert_text", self.insert_text.as_ref());
        write!(f, "{{ {} }}", out.join(", "))
    }
}
