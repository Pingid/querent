use smol_str::SmolStr;

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

    /// Convert to Any for downcasting in format_expected
    fn as_any(&self) -> &dyn std::any::Any;
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

    /// Formats the expected completions for display in error messages
    pub fn format_expected(&self) -> String {
        if self.0.is_empty() {
            return "(no expectations set)\n".to_string();
        }

        self.0
            .iter()
            .map(|matcher| {
                // Try to downcast to known matcher types for better formatting
                if let Some(starts) = matcher.as_any().downcast_ref::<Starts>() {
                    format!("Starts with: {:?}\n", starts.0)
                } else if let Some(contains) = matcher.as_any().downcast_ref::<Contains>() {
                    format!("Contains: {} items\n", contains.0.len())
                } else if let Some(in_order) = matcher.as_any().downcast_ref::<InOrder>() {
                    format!("In order: {} items\n", in_order.0.len())
                } else if let Some(_) = matcher.as_any().downcast_ref::<Empty>() {
                    "Empty (no completions expected)\n".to_string()
                } else {
                    "(custom matcher)\n".to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("")
    }
}

/// Matcher that expects no completions.
///
/// This matcher fails if any completions are present in the result set.
pub struct Empty(pub Option<Vec<CompletionKind>>);
impl Matcher for Empty {
    fn check(&self, items: &[Completion]) -> Result<(), String> {
        if self.0.is_none() && items.len() > 0 {
            return Err(format!(
                "Expected no completions, got {}:\n{}",
                items.len(),
                fmt_list(items)
            ));
        }
        if let Some(kinds) = &self.0 {
            for kind in kinds {
                if items.iter().any(|item| item.kind == *kind) {
                    return Err(format!("Expected no completions of kind {:?}", kind));
                }
            }
        }
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
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

    fn as_any(&self) -> &dyn std::any::Any {
        self
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

    fn as_any(&self) -> &dyn std::any::Any {
        self
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
                return out_of_order(items, idx, last);
            }
            last = Some(idx);
        }
        Ok(())
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

// pub struct

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
    pub label: Option<SmolStr>,
    pub kind: Option<CompletionKind>,
    pub detail: Option<SmolStr>,
    pub insert_text: Option<SmolStr>,
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
            label: Some(s.into()),
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
