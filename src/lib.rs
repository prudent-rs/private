#![doc = include_str!("../README.md")]

use core::convert::TryFrom;
use dis::ext::core::ContentIntoDisplayExt;
use dis::prelude_ext::{alloc::*, proc_macro2_diagnostics::*};
use dis::{MacroDeepResult, MacroDiagnosticResult, MacroStreamResult};

//use mce_lib::public::OwnedStringSlice;

use proc_macro::TokenStream as ProcTokenStream;
use proc_macro_rules::rules;
use proc_macro2::{Span, TokenStream, TokenTree};

use quote::{quote, quote_spanned};
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::LazyLock;
use syn::{Expr, Ident, Type};

mod file;

/// Thanks to build.rs.
const OUT_DIR: &str = env!("OUT_DIR");

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum IdentNameConvention {
    CamelCase,
    LowerCase,
    UpperCase,
}
impl IdentNameConvention {
    const fn uses_underscore(&self) -> bool {
        matches!(*self, Self::LowerCase | Self::UpperCase)
    }

    /*fn can_apply_in_place(&self, s: &str, is_last_part: bool) -> bool {
        s.is_ascii() && !self.uses_underscore()
    }*/

    /// Apply formatting, including appending an underscore if this formatting involves it (and if
    /// `is_last_part` is `false`).
    fn apply(&self, mut s: String, is_last_part: bool) -> String {
        if let Some(new) = self.apply_or_new(&mut s, is_last_part) {
            new
        } else {
            s
        }
    }

    /// Apply formatting, including appending an underscore if this formatting involves it (and if
    /// `is_last_part` is `false`). Potentially modify the given slice, ad/or return [Some] of a new
    /// [String].
    /// - If returns [Some], use the return value. The given slice `s` _may_ be modified, but _not_
    ///   complete - so don't use it.
    /// - If returns [None], use the givene slice `s` - which may be modified (if due).
    fn apply_or_new(self, s: &mut str, is_last_part: bool) -> Option<String> {
        match self {
            IdentNameConvention::LowerCase | IdentNameConvention::UpperCase => {
                if s.is_ascii() && is_last_part {
                    match self {
                        IdentNameConvention::LowerCase => s.make_ascii_lowercase(),
                        IdentNameConvention::UpperCase => s.make_ascii_uppercase(),
                        IdentNameConvention::CamelCase => unreachable!(),
                    }
                    return None;
                } else {
                    let mut s = match self {
                        IdentNameConvention::LowerCase => s.to_lowercase(),
                        IdentNameConvention::UpperCase => s.to_uppercase(),
                        IdentNameConvention::CamelCase => unreachable!(),
                    };
                    if !is_last_part {
                        s.push('_');
                    }
                    return Some(s);
                }
            }
            IdentNameConvention::CamelCase => {
                if s.is_ascii() {
                    s.make_ascii_lowercase();
                    if s.len() > 0 {
                        let first = &mut s[0..1];
                        first.make_ascii_uppercase();
                    }
                    return None;
                } else {
                    let mut new = String::with_capacity(s.len());
                    let mut chars = s.chars();
                    if let Some(first) = chars.next() {
                        new.extend(first.to_uppercase());
                    }
                    new.extend(chars.map(|c| c.to_uppercase()).flatten());
                    return Some(new);
                }
            }
        }
    }

    /// Token that indicates this name_convention, as expected by [at_direct].
    fn macro_input_token(&self, span: Span) -> Ident {
        let ident = match *self {
            Self::LowerCase => "lower_case",
            Self::UpperCase => "UPPER_CASE",
            Self::CamelCase => "CamelCase",
        };
        Ident::new(ident, span)
    }
}
impl TryFrom<&str> for IdentNameConvention {
    type Error = String;

    fn try_from(id: &str) -> Result<Self, Self::Error> {
        let mut flags = [false, false, false, false];
        // 0-based indexes to bools in flags[]:
        const HAS_INNER_UNDERSCORE: usize = 0;
        const HAS_LOWER: usize = 1;
        const HAS_UPPER: usize = 2;
        // _not_ conclusive/exhaustive - even if CANNOT_BE_CAMEL==false, id still may _not_ be in
        // CamelCase.
        const CANNOT_BE_CAMEL: usize = 3;

        //let how_many_true = |can_be: &[bool]| -> usize { can_be.iter().map(|b| if *b { 1 } else { 0 }).sum() };

        let mut expecting_first_letter = true;
        for c in id.chars() {
            if c == '_' {
                // Ignoring leadins _ __ ___ etc.
                if !expecting_first_letter {
                    // Treating _ __ ___ etc. as an inner underscore
                    flags[HAS_INNER_UNDERSCORE] = true;
                }
                continue;
            }
            if c.is_alphabetic() {
                if c.is_lowercase() && !c.is_uppercase() {
                    if expecting_first_letter {
                        flags[CANNOT_BE_CAMEL] = true;
                    }
                    flags[HAS_LOWER] = true;
                } else if c.is_uppercase() && !c.is_lowercase() {
                    flags[HAS_UPPER] = true;
                }
                expecting_first_letter = false;
            }
        }
        const COULDNT_DETECT_FOR: &str = "Naming convention couldn't be detected for ";

        match (flags[HAS_LOWER], flags[HAS_UPPER]) {
            (true, true) => {
                if flags[CANNOT_BE_CAMEL] {
                    let leading_underscore_clause = if flags[HAS_INNER_UNDERSCORE] {
                        " (after the leading underscore(s))."
                    } else {
                        ""
                    };
                    Err(format!(
                        "{COULDNT_DETECT_FOR}{id}. It contains both lowercase and uppercase, but \
                        it doesn't start with uppercase{leading_underscore_clause}."
                    ))
                } else if flags[HAS_INNER_UNDERSCORE] {
                    Err(format!(
                        "{COULDNT_DETECT_FOR}{id}. It contains both lowercase and uppercase, but \
                        it also contains an inner underscore."
                    ))
                } else {
                    Ok(IdentNameConvention::CamelCase)
                }
            }
            (true, false) => Ok(IdentNameConvention::LowerCase),
            (false, true) => {
                if flags[HAS_INNER_UNDERSCORE] {
                    Ok(IdentNameConvention::UpperCase)
                } else {
                    Err(format!(
                        "{COULDNT_DETECT_FOR}{id}. It contains uppercase, no lowercase, and no inner underscore, but it could pass as either UPPERCASE or CamelCase (in Rust)."
                    ))
                }
            },
            (false, false) => Err(format!(
                "{COULDNT_DETECT_FOR}{id}. It doesn't contain either lowercase or uppercase."
            )),
        }
    }
}

static OUT_DIR_LOWER_CASE: LazyLock<String> = LazyLock::new(|| {
    let s = OUT_DIR.to_owned();
    IdentNameConvention::LowerCase.apply(s, false)
});
static OUT_DIR_UPPER_CASE: LazyLock<String> = LazyLock::new(|| {
    let s = OUT_DIR.to_owned();
    IdentNameConvention::UpperCase.apply(s, false)
});
static OUT_DIR_CAMEL_CASE: LazyLock<String> = LazyLock::new(|| {
    let s = OUT_DIR.to_owned();
    IdentNameConvention::CamelCase.apply(s, false)
});
fn out_dir(name_convention: IdentNameConvention) -> &'static str {
    match name_convention {
        IdentNameConvention::LowerCase => &*OUT_DIR_LOWER_CASE,
        IdentNameConvention::UpperCase => &*OUT_DIR_UPPER_CASE,
        IdentNameConvention::CamelCase => &*OUT_DIR_CAMEL_CASE,
    }
}

static RND_PART_LOWER_CASE: LazyLock<String> = LazyLock::new(|| {
    let s = file::existing_rnd_part(OUT_DIR)
        .unwrap()
        .expect("Expecting a file generated by build.rs.");
    IdentNameConvention::LowerCase.apply(s, true)
});
static RND_PART_UPPER_CASE: LazyLock<String> = LazyLock::new(|| {
    let s = (*RND_PART_LOWER_CASE).clone();
    IdentNameConvention::UpperCase.apply(s, true)
});
static RND_PART_CAMEL_CASE: LazyLock<String> = LazyLock::new(|| {
    let s = (*RND_PART_LOWER_CASE).clone();
    IdentNameConvention::CamelCase.apply(s, true)
});
fn rnd_part(name_convention: IdentNameConvention) -> &'static str {
    match name_convention {
        IdentNameConvention::LowerCase => &*RND_PART_LOWER_CASE,
        IdentNameConvention::UpperCase => &*RND_PART_UPPER_CASE,
        IdentNameConvention::CamelCase => &*RND_PART_CAMEL_CASE,
    }
}

fn restricted_part(name_convention: IdentNameConvention) -> &'static str {
    match name_convention {
        IdentNameConvention::LowerCase => "restricted_",
        IdentNameConvention::UpperCase => "RESTRICTED_",
        IdentNameConvention::CamelCase => "Restricted",
    }
}

fn underscore_if_used(name_convention: IdentNameConvention) -> &'static str {
    if name_convention.uses_underscore() {
        "_"
    } else {
        ""
    }
}

struct LocalFilePath {
    local_path: PathBuf,
    name_convention: IdentNameConvention,
}
impl LocalFilePath {
    /// Create an instance. Convert any ASCII character sto lowercase/uppercase as indicated by
    /// `make_uppercase`.
    pub fn new(local_path: PathBuf, name_convention: IdentNameConvention) -> Self {
        Self {
            local_path,
            name_convention,
        }
    }
}

// Can't implement
// - `impl From<&LocalFilePath> for Option<&str>`, or
// - `impl Into<Option<&str>> for &LocalFilePath`
//
// because their lifetimes would have to be independent.
impl LocalFilePath {
    /// Get subpath, excluding any part shared with [OUT_DIR], and sanitize to be
    /// [Ident]-compatible.
    ///
    /// Do _not_ use for a last part of an identifier, because if [LocalFilePath::name_convention] part
    /// of `self` has [IdentNameConvention::uses_underscore] `true`, then this appends an underscore
    /// (which would then become a trailing underscore).
    ///
    // Just a subset of
    // https://rust-lang.github.io/rfcs/2457-non-ascii-idents.html#reference-level-explanation
    // (which refers to https://www.unicode.org/reports/tr31/).
    fn sanitize_as_ident_multi_part(&self) -> MacroDeepResult<String> {
        return if let Some(path) = self.local_path.to_str() {
            /*
            let s = if s.is_ascii() {
                // LocalFilePath::new() already converted any ASCII characters to
                // lowercase/uppercase as appropriate.
                owned_s = None;
                s
            } else {
                owned_s = Some(if self.name_convention {
                    s.to_uppercase()
                } else {
                    s.to_lowercase()
                });
                owned_s.as_ref().unwrap()
            };

            let out_dir = if self.name_convention {
                &(*OUT_DIR_UPPER_CASE)[..]
            } else {
                &(*OUT_DIR_LOWER_CASE)[..]
            };
            */

            let out_dir = out_dir(self.name_convention);

            let out_dir_chars_opt = out_dir
                .chars()
                .map(|c| Some(c))
                .chain(core::iter::repeat(None));
            let path_chars = path.chars();
            let mut pairs = path_chars.zip(out_dir_chars_opt);

            let mut num_same_chars_skipped = 0;
            loop {
                if let Some((p, Some(o))) = pairs.next()
                    && o == p
                {
                    num_same_chars_skipped += 1;
                    continue;
                } else {
                    break;
                }
            }

            let max_part_len = path.len() - num_same_chars_skipped;
            let mut joined_parts = String::with_capacity(max_part_len);
            {
                let mut part = String::with_capacity(max_part_len);

                let mut first_part_first_char = true;
                for (p, _) in pairs {
                    if first_part_first_char {
                        if p.is_alphabetic() {
                            part.push(p);
                            first_part_first_char = false;
                        }
                    } else {
                        if p.is_alphanumeric() {
                            part.push(p);
                        } else {
                            if part.len() > 0 {
                                part = self.name_convention.apply(part, false);
                                joined_parts.extend(part.chars());
                                part.clear();
                            }
                        }
                    }
                }
                if part.len() > 0 {
                    let part = self.name_convention.apply(part, false);
                    joined_parts.extend(part.chars());
                }
            }
            Ok(joined_parts)
        } else {
            Err(format!(
                "Source code file path contains non-UTF-8 byte(s). Lossy path: {}",
                self.local_path.to_string_lossy()
            )
            .into_dis()
            .dis_as_macro_deep_diagnostic())
        };
    }
    /*let len = s.len().min(out_dir.len());
        let mut i = 0;
        while i < len {
            if s.as_bytes()[i] != out_dir.as_bytes()[i] {
                break;
            }
            i += 1;
        }
        // @TODO proper error
        assert!(
            i < s.len(),
            "Source of the macro crate that uses crate 'restricted' is at {s}, which should not under of OUT_DIR: {OUT_DIR}"
        );
        //OwnedStringSlice::new_starting_from(s, i)
        &s[i..]
    } else {
        return Err(format!(
            "Source code file path contains non-UTF-8 byte(s). Lossy path: {}",
            self.local_path.to_string_lossy()
        )
        .into_dis()
        .as_macro_deep_diagnostic());
    };

    let mut first_char = true;

    let filtered = path.chars().filter_map(|c| {
        if first_char {
            if c.is_alphabetic() {
                first_char = false;
                Some(c)
            } else {
                // Let's _not_ have the first character an underscore
                None
            }
        } else {
            if c.is_alphanumeric() {
                Some(c)
            } else {
                Some('_')
            }
        }
    });
    let mut s = String::with_capacity(path.len());
    s.extend(filtered);
    Ok(s)*/
}

fn local_file_path(
    span: Span,
    name_convention: IdentNameConvention,
) -> MacroDiagnosticResult<LocalFilePath> {
    if let Some(local_path) = span.local_file() {
        Ok(LocalFilePath::new(local_path, name_convention))
    } else {
        Err(format!(
            "Source file path unknown for code that invokes crate `restricted`. Span: {:?}",
            span
        )
        .dis_to_error_at(span))
    }
}

/// Generate a full name. We do _not_ modify the part based on `ident_given_name` - so
/// `ident_given_name` already has to follow `name_convention`. However, we exclude any leading or
/// trailing underscores from `ident_given_name` - we do insert an underscore on each side of
/// `ident_given_name` if `name_convention` uses it.
///
/// Compatible with Rust non-ASCII identifiers
/// (https://rust-lang.github.io/rfcs/2457-non-ascii-idents.html).
fn restricted_full_name(
    ident_short_name: &Ident,
    name_convention: IdentNameConvention,
    //@TODO prefix_with_underscore: bool
) -> MacroDiagnosticResult<Ident> {
    let span = ident_short_name.span();

    let file_path = local_file_path(span, name_convention)?;
    let file_multi_part = file_path
        .sanitize_as_ident_multi_part()
        .dis_span_err(span)?;

    let restricted_part = restricted_part(name_convention);

    let underscore_if_used = underscore_if_used(name_convention);
    let rnd_part = rnd_part(name_convention);

    // Intentionally putting the random part at the end, to make errors/warnings more
    // obvious/searchable.

    let full_name = format!(
        "{ident_short_name}{underscore_if_used}{file_multi_part}{restricted_part}{rnd_part}"
    );

    Ok(Ident::new(&full_name, span))
}
// --------------

#[proc_macro]
pub fn def_const(input: ProcTokenStream) -> ProcTokenStream {
    match def_const_or_static_grammar(input.into(), ItemChoice::Const, false) {
        Ok(output) => output.into(),
        Err(diag) => panic!("{:?}", diag), //diag.emit_as_expr_tokens().into(),
    }
}
#[proc_macro]
pub fn def_const_direct(input: ProcTokenStream) -> ProcTokenStream {
    match def_const_or_static_grammar(input.into(), ItemChoice::Const, true) {
        Ok(output) => output.into(),
        Err(diag) => panic!("{:?}", diag), //diag.emit_as_expr_tokens().into(),
    }
}
#[proc_macro]
pub fn def_static(input: ProcTokenStream) -> ProcTokenStream {
    match def_const_or_static_grammar(input.into(), ItemChoice::Static, false) {
        Ok(output) => output.into(),
        Err(diag) => panic!("{:?}", diag), //diag.emit_as_expr_tokens().into(),
    }
}
#[proc_macro]
pub fn def_static_direct(input: ProcTokenStream) -> ProcTokenStream {
    match def_const_or_static_grammar(input.into(), ItemChoice::Static, true) {
        Ok(output) => output.into(),
        Err(diag) => panic!("{:?}", diag), //diag.emit_as_expr_tokens().into(),
    }
}
// --------------

#[proc_macro]
pub fn def_let(input: ProcTokenStream) -> ProcTokenStream {
    match def_let_or_mut_grammar(input.into(), ItemChoice::Let, false) {
        Ok(output) => output.into(),
        Err(diag) => panic!("{:?}", diag), //diag.emit_as_expr_tokens().into(),
    }
}
#[proc_macro]
pub fn def_let_direct(input: ProcTokenStream) -> ProcTokenStream {
    match def_let_or_mut_grammar(input.into(), ItemChoice::Let, true) {
        Ok(output) => output.into(),
        Err(diag) => panic!("{:?}", diag), //diag.emit_as_expr_tokens().into(),
    }
}
#[proc_macro]
pub fn def_mut(input: ProcTokenStream) -> ProcTokenStream {
    match def_let_or_mut_grammar(input.into(), ItemChoice::Mut, false) {
        Ok(output) => output.into(),
        Err(diag) => panic!("{:?}", diag), //diag.emit_as_expr_tokens().into(),
    }
}
#[proc_macro]
pub fn def_mut_direct(input: ProcTokenStream) -> ProcTokenStream {
    match def_let_or_mut_grammar(input.into(), ItemChoice::Mut, true) {
        Ok(output) => output.into(),
        Err(diag) => panic!("{:?}", diag), //diag.emit_as_expr_tokens().into(),
    }
}
// --------------

#[proc_macro]
pub fn def_use(input: ProcTokenStream) -> ProcTokenStream {
    match def_use_grammar(input.into(), false) {
        Ok(output) => output.into(),
        Err(diag) => panic!("{:?}", diag), //diag.emit_as_expr_tokens().into(),
    }
}
#[proc_macro]
pub fn def_use_direct(input: ProcTokenStream) -> ProcTokenStream {
    match def_use_grammar(input.into(), true) {
        Ok(output) => output.into(),
        Err(diag) => panic!("{:?}", diag), //diag.emit_as_expr_tokens().into(),
    }
}
// --------------

#[proc_macro]
pub fn at_const(input: ProcTokenStream) -> ProcTokenStream {
    match at_grammar_item_with_only_one_convention(input.into(), ItemChoice::Const) {
        Ok(output) => output.into(),
        Err(diag) => panic!("{:?}", diag), //diag.emit_as_expr_tokens().into(),
    }
}
#[proc_macro]
pub fn at_let(input: ProcTokenStream) -> ProcTokenStream {
    match at_grammar_item_with_only_one_convention(input.into(), ItemChoice::Let) {
        Ok(output) => output.into(),
        Err(diag) => panic!("{:?}", diag), //diag.emit_as_expr_tokens().into(),
    }
}
#[proc_macro]
pub fn at_mut(input: ProcTokenStream) -> ProcTokenStream {
    match at_grammar_item_with_only_one_convention(input.into(), ItemChoice::Mut) {
        Ok(output) => output.into(),
        Err(diag) => panic!("{:?}", diag), //diag.emit_as_expr_tokens().into(),
    }
}
#[proc_macro]
pub fn at_static(input: ProcTokenStream) -> ProcTokenStream {
    match at_grammar_item_with_only_one_convention(input.into(), ItemChoice::Static) {
        Ok(output) => output.into(),
        Err(diag) => panic!("{:?}", diag), //diag.emit_as_expr_tokens().into(),
    }
}

#[proc_macro]
pub fn at_use(input: ProcTokenStream) -> ProcTokenStream {
    match at_use_grammar(input.into()) {
        Ok(output) => output.into(),
        Err(diag) => panic!("{:?}", diag), //diag.emit_as_expr_tokens().into(),
    }
}

/// Primarily for `trait`, where [at_use] and "direct" macros are unusable.
#[proc_macro]
pub fn use_with(input: ProcTokenStream) -> ProcTokenStream {
    match use_with_grammar(input.into()) {
        Ok(output) => output.into(),
        Err(diag) => panic!("{:?}", diag), //diag.emit_as_expr_tokens().into(),
    }
}
// --------------

/// Internal - not for direct use. For `const`, `let`, `let mut` and `static` only.
#[proc_macro]
#[doc(hidden)]
pub fn at_direct(input: ProcTokenStream) -> ProcTokenStream {
    match at_direct_grammar(input.into()) {
        Ok(output) => output.into(),
        Err(diag) => panic!("{:?}", diag), //diag.emit_as_expr_tokens().into(),
    }
}

/// Internal - not for direct use. For `use` only.
#[proc_macro]
#[doc(hidden)]
pub fn use_direct(input: ProcTokenStream) -> ProcTokenStream {
    match use_direct_grammar(input.into()) {
        Ok(output) => output.into(),
        Err(diag) => panic!("{:?}", diag), //diag.emit_as_expr_tokens().into(),
    }
}
// --------------

fn def_const_or_static_grammar(
    input: TokenStream,
    which: ItemChoice,
    create_direct_accessor: bool,
) -> MacroStreamResult {
    assert!(matches!(which, ItemChoice::Const | ItemChoice::Static));
    rules!(input => {
        ( $short_name:ident : $ty:ty = $value:expr ) => {
            def_const_or_static_or_let_or_mut(which, short_name, Some(ty), Some(value), create_direct_accessor)
        }
    })
    .into()
}

fn def_let_or_mut_grammar(
    input: TokenStream,
    which: ItemChoice,
    create_direct_accessor: bool,
) -> MacroStreamResult {
    assert!(matches!(which, ItemChoice::Let | ItemChoice::Mut));
    rules!(input => {
        ( $short_name:ident = $value:expr ) => {
            def_const_or_static_or_let_or_mut(which, short_name, None, Some(value), create_direct_accessor)
        }
        ( $short_name:ident : $ty:ty = $value:expr ) => {
            def_const_or_static_or_let_or_mut(which, short_name, Some(ty), Some(value), create_direct_accessor)
        }
        ( $short_name:ident : $ty:ty) => {
            def_const_or_static_or_let_or_mut(which, short_name, Some(ty), None, create_direct_accessor)
        }
        ( $short_name:ident ) => {
            def_const_or_static_or_let_or_mut(which, short_name, None, None, create_direct_accessor)
        }
    })
    .into()
}
/*macro_rules! _path_then_colon {
    ($path:path)
}*/
//----------

fn def_use_grammar(input: TokenStream, create_direct_accessor: bool) -> MacroStreamResult {
    rules!(input => {
        ( $ident_short_name: ident,
          CamelCase,
          // @TODO other param: attribute(s)?
          $( $items: tt )*
        ) => {
            def_use_impl(ident_short_name, items, IdentNameConvention::CamelCase, create_direct_accessor)
        }
        ( $ident_short_name: ident,
          lower_case,
          // @TODO other param: attribute(s)?
          $( $items: tt )*
        ) => {
            def_use_impl(ident_short_name, items, IdentNameConvention::LowerCase, create_direct_accessor)
        }
        ( $ident_short_name: ident,
          UPPER_CASE,
          // @TODO other param: attribute(s)?
          $( $items: tt )*
        ) => {
            def_use_impl(ident_short_name, items, IdentNameConvention::UpperCase, create_direct_accessor)
        }
    })
    .into()
}
//----------
//----------

fn at_grammar_item_with_only_one_convention(
    input: TokenStream,
    which: ItemChoice,
) -> MacroStreamResult {
    rules!(input => {
        ( $ident_short_name:ident) => {
            at_impl(&ident_short_name, which.name_convention().unwrap())
        }
    })
}

fn at_use_grammar(input: TokenStream) -> MacroStreamResult {
    rules!(input => {
        ( $ident_short_name:ident, CamelCase) => {

            at_use_impl(&ident_short_name, IdentNameConvention::CamelCase)
        }
        ( $ident_short_name:ident, UPPER_CASE) => {

            at_use_impl(&ident_short_name, IdentNameConvention::UpperCase)
        }
        ( $ident_short_name:ident, lower_case) => {

            at_use_impl(&ident_short_name, IdentNameConvention::LowerCase)
        }
    })
}

fn use_with_grammar(input: TokenStream) -> MacroStreamResult {
    rules!(input => {
        ( $ident_short_name:ident, CamelCase, $accessor_module:ident, $( $items: tt )*
        ) => {

            use_with_impl(&ident_short_name, IdentNameConvention::CamelCase, &accessor_module, items)
        }
        ( $ident_short_name:ident, UPPER_CASE, $accessor_module:ident, $( $items: tt )*
        ) => {

            use_with_impl(&ident_short_name, IdentNameConvention::UpperCase, &accessor_module, items)
        }
        ( $ident_short_name:ident, lower_case, $accessor_module:ident, $( $items: tt )*
        ) => {

            use_with_impl(&ident_short_name, IdentNameConvention::LowerCase, &accessor_module, items)
        }
    })
}
//----------

fn at_direct_grammar(input: TokenStream) -> MacroStreamResult {
    rules!(input => {
        ( $ident_short_name:ident, $ident_full_name:ident, $alleged_macro_provider_scope_tokens:tt, lower_case) => {

            at_direct_grammar_for_convention(ident_short_name, ident_full_name, alleged_macro_provider_scope_tokens, IdentNameConvention::LowerCase)
        }
        ( $ident_short_name:ident, $ident_full_name:ident, $alleged_macro_provider_scope_tokens:tt, UPPER_CASE) => {

            at_direct_grammar_for_convention(ident_short_name, ident_full_name, alleged_macro_provider_scope_tokens, IdentNameConvention::UpperCase)
        }
        ( $ident_short_name:ident, $ident_full_name:ident, $alleged_macro_provider_scope_tokens:tt, CamelCase) => {

            at_direct_grammar_for_convention(ident_short_name, ident_full_name, alleged_macro_provider_scope_tokens, IdentNameConvention::CamelCase)
        }
    })
}

/// Verify that access to `ident_short_name` as `alleged_ident_full_name` is compatible/allowed from
/// scope of `alleged_macro_provider_scope_tokens`.
fn verify_alleged_access(
    ident_short_name: &Ident,
    alleged_ident_full_name: &Ident,
    alleged_macro_provider_scope_tokens: &TokenTree,
    name_convention: IdentNameConvention,
) -> MacroDiagnosticResult<()> {
    let ident_full_name_based_on_actual_macro_provider_span =
        restricted_full_name(&ident_short_name, name_convention)?;

    if &ident_full_name_based_on_actual_macro_provider_span != alleged_ident_full_name {
        let alleged_macro_provider_span = alleged_macro_provider_scope_tokens.span();
        return Err(format!(
            "Identifier {ident_short_name} is not accessible in the given span \
                        {alleged_macro_provider_span:?}."
        )
        .dis_to_error_at(alleged_macro_provider_span));
    }
    Ok(())
}

fn use_direct_grammar(input: TokenStream) -> MacroStreamResult {
    rules!(input => {
        ( $ident_short_name:ident, $ident_full_name:ident, $alleged_macro_provider_scope_tokens:tt, lower_case) => {

            use_direct_grammar_for_convention(ident_short_name, ident_full_name, alleged_macro_provider_scope_tokens, IdentNameConvention::LowerCase)
        }
        ( $ident_short_name:ident, $ident_full_name:ident, $alleged_macro_provider_scope_tokens:tt, UPPER_CASE) => {

            use_direct_grammar_for_convention(ident_short_name, ident_full_name, alleged_macro_provider_scope_tokens, IdentNameConvention::UpperCase)
        }
        ( $ident_short_name:ident, $ident_full_name:ident, $alleged_macro_provider_scope_tokens:tt, CamelCase) => {

            use_direct_grammar_for_convention(ident_short_name, ident_full_name, alleged_macro_provider_scope_tokens, IdentNameConvention::CamelCase)
        }
    })
}
//----------

fn at_direct_grammar_for_convention(
    ident_short_name: Ident,
    alleged_ident_full_name: Ident,
    alleged_macro_provider_scope_tokens: TokenTree,
    name_convention: IdentNameConvention,
) -> MacroStreamResult {
    let alleged_macro_provider_span = alleged_macro_provider_scope_tokens.span();

    verify_alleged_access(
        &ident_short_name,
        &alleged_ident_full_name,
        &alleged_macro_provider_scope_tokens,
        name_convention,
    )?;

    Ok(quote_spanned! {alleged_macro_provider_span=>
        #alleged_ident_full_name
    })
}

fn use_direct_grammar_for_convention(
    ident_short_name: Ident,
    alleged_ident_full_name: Ident,
    alleged_macro_provider_scope_tokens: TokenTree,
    name_convention: IdentNameConvention,
) -> MacroStreamResult {
    let alleged_macro_provider_span = alleged_macro_provider_scope_tokens.span();

    verify_alleged_access(
        &ident_short_name,
        &alleged_ident_full_name,
        &alleged_macro_provider_scope_tokens,
        name_convention,
    )?;

    let outer_mod = use_outer_mod_name(&ident_short_name);

    Ok(quote_spanned! {alleged_macro_provider_span=>
        #outer_mod::#alleged_ident_full_name
    })
}
//----------

/// Access choice for a var/value. For now (and hopefully forever) we ignore `static mut`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ItemChoice {
    Const, // this covers "fn", because a function can be assigned to a constant
    Let,
    Mut,
    Static,
}
impl ItemChoice {
    /// One of: `const`, `fn`, `static`, `trait`, `type`.
    pub fn keywords(&self) -> &'static str {
        match self {
            Self::Const => "const",
            Self::Let => "let",
            Self::Mut => "let mut",
            Self::Static => "static",
        }
    }
    /// Defined only for [`ItemChoice::Const`], [`ItemChoice::Let`], [`ItemChoice::Mut`] and
    /// [`ItemChoice::Static`].
    pub fn requires_type_and_value(&self) -> bool {
        assert!(matches!(
            self,
            &Self::Const | &Self::Let | &Self::Mut | &Self::Static
        ));
        matches!(self, &Self::Const | &Self::Static)
        //self == &Self::Const || self == &Self::Static
    }

    /// Returning [Some] means to use that [IdentNameConvention]. But, returning [None] means that
    /// the [IdentNameConvention] depends on how this [ItemChoice] is used.
    // @TODO if we don't need ItemItemChoice::Use, then remove Option<> here and just return
    // IdentNameConvention
    pub fn name_convention(&self) -> Option<IdentNameConvention> {
        match self {
            Self::Const | Self::Static => Some(IdentNameConvention::UpperCase),
            Self::Let | Self::Mut => Some(IdentNameConvention::LowerCase),
        }
    }
}

fn def_const_or_static_or_let_or_mut(
    which: ItemChoice,
    ident_short_name: Ident,
    ty: Option<Type>,
    value: Option<Expr>,
    create_direct_accessor: bool,
) -> MacroStreamResult {
    assert!(
        matches!(
            which,
            ItemChoice::Const | ItemChoice::Let | ItemChoice::Mut | ItemChoice::Static
        ),
        "Given item choice {which:?} is not one acceptable."
    );

    assert!(
        !which.requires_type_and_value() || ty.is_some() && value.is_some(),
        "Given item choice {which:?} requires both the type and the value."
    );

    let name_convention = which.name_convention().unwrap();
    let ident_full_name = restricted_full_name(&ident_short_name, name_convention)?;

    let span = ident_short_name.span();

    let const_static_part = TokenStream::from_str(which.keywords())
        .dis_map_err_to()
        .dis_map_macro_err()
        .dis_span_err(span)?;

    let type_part = if let Some(ty) = ty {
        quote_spanned! {span=>
            : #ty
        }
    } else {
        TokenStream::new()
    };
    let value_part = if let Some(value) = value {
        quote_spanned! {span=>
            = #value
        }
    } else {
        TokenStream::new()
    };

    let doc = format!("(restricted) {} {ident_short_name}", which.keywords());

    let convention_token = name_convention.macro_input_token(span);
    let direct_part = if create_direct_accessor {
        // #[doc = #doc] works to generate tooltip/mouseover with rust-analyzer:
        quote_spanned! {span=>

            #[doc = #doc]
            #[doc(hidden)]
            macro_rules! #ident_short_name {

                ($macro_provider_scope_tokens:tt) => {

                    ::restricted_enforce::at_direct!(#ident_short_name, #ident_full_name, $macro_provider_scope_tokens, #convention_token)
                }
            }
        }
    } else {
        TokenStream::new()
    };

    Ok(quote_spanned! {span=>
        #[doc = #doc]
        #const_static_part #ident_full_name #type_part #value_part;

        #direct_part
    })
}

fn use_outer_mod_name(ident_short_name: &Ident) -> Ident {
    let span = ident_short_name.span();
    // Based on
    // https://github.com/search?q=%2F%5B%5Ea-zA-Z_%5Dmod+%2Brestricted_outer_%5Ba-zA-Z_0-9%5D%2B%2F+language%3ARust&type=code
    // (which is GitHub search for `/[^a-zA-Z_]mod +restricted_outer_[a-zA-Z_0-9]+/ language:Rust`)
    // there is no Rust module with name starting with `restricted_outer_`.
    Ident::new(
        &format!(
            "restricted_outer_{}",
            IdentNameConvention::LowerCase.apply(ident_short_name.to_string(), true)
        ),
        span,
    )
}

fn def_use_impl(
    ident_short_name: Ident,
    items: Vec<TokenTree>,
    name_convention: IdentNameConvention,
    create_direct_accessor: bool,
) -> MacroStreamResult {
    let ident_full_name = restricted_full_name(&ident_short_name, name_convention)?;

    let span = ident_short_name.span();
    let outer_mod = use_outer_mod_name(&ident_short_name);

    let doc = format!("(restricted) {ident_short_name}");

    let convention_token = name_convention.macro_input_token(span);
    let direct_part = if create_direct_accessor {
        // #[doc = #doc] works to generate tooltip/mouseover with rust-analyzer:
        quote_spanned! {span=>

            #[doc = #doc]
            #[doc(hidden)]
            macro_rules! #ident_short_name {

                ($macro_provider_scope_tokens:tt) => {

                    ::restricted_enforce::use_direct!(#ident_short_name, #ident_full_name, $macro_provider_scope_tokens, #convention_token)
                                }
            }
        }
    } else {
        TokenStream::new()
    };

    // https://github.com/search?q=%2F%5B%5Ea-zA-Z_%5Dmod+%2Brestricted_inner%2F+language%3ARust&type=code
    // (which is GitHub search for `/[^a-zA-Z_]mod +restricted_inner/ language:Rust`) returns no
    // matches.
    Ok(quote_spanned! {span=>
        mod #outer_mod {
            mod restricted_inner {
                // This will need:
                use super::super::*;

                #( #items )*
            }
            pub(crate) use restricted_inner::#ident_short_name as #ident_full_name;
        }

        #direct_part
    })
}
//------------

fn at_impl(ident_short_name: &Ident, name_convention: IdentNameConvention) -> MacroStreamResult {
    let full_name = restricted_full_name(ident_short_name, name_convention)?;

    let span = ident_short_name.span();
    Ok(quote_spanned! {span=>
        #full_name
    })
}

fn at_use_impl(
    ident_short_name: &Ident,
    name_convention: IdentNameConvention,
) -> MacroStreamResult {
    let span = ident_short_name.span();

    let outer_mod = use_outer_mod_name(&ident_short_name);
    let full_name = restricted_full_name(ident_short_name, name_convention)?;

    Ok(quote_spanned! {span=>
        #outer_mod::#full_name
    })
}

fn use_with_impl(
    ident_short_name: &Ident,
    name_convention: IdentNameConvention,
    accessor_module: &Ident,
    items: Vec<TokenTree>,
) -> MacroStreamResult {
    let span = ident_short_name.span();

    let outer_mod = use_outer_mod_name(&ident_short_name);
    let full_name = restricted_full_name(ident_short_name, name_convention)?;

    Ok(quote_spanned! {span=>
        mod #accessor_module {
            // intentionally _not_ `pub use`
            use super::#outer_mod::#full_name as #ident_short_name;

            #( #items )*
        }
        use #accessor_module::*;
    })
}

//------------

#[doc(hidden)]
#[proc_macro]
pub fn version(input: ProcTokenStream) -> ProcTokenStream {
    rules!(input.into() => {
        () => {
            const VERSION: &str = env!("CARGO_PKG_VERSION");
            quote! {
                #VERSION
            }
        }
    })
    .into()
}
