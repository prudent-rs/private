use core::convert::TryFrom;
use proc_macro2::Span;
use syn::Ident;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentNameConvention {
    CamelCase,
    LowerCase,
    UpperCase,
}
impl IdentNameConvention {
    pub const fn uses_underscore(&self) -> bool {
        matches!(*self, Self::LowerCase | Self::UpperCase)
    }

    /*fn can_apply_in_place(&self, s: &str, is_last_part: bool) -> bool {
        s.is_ascii() && !self.uses_underscore()
    }*/

    /// Apply formatting, including appending an underscore if this formatting involves it (and if
    /// `is_last_part` is `false`).
    pub fn apply(&self, mut s: String, is_last_part: bool) -> String {
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
    pub fn apply_or_new(self, s: &mut str, is_last_part: bool) -> Option<String> {
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
    pub fn macro_input_token(&self, span: Span) -> Ident {
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
        let mut has_leading_underscore = false;
        let mut flags = [false, false, false, false];
        // 0-based indexes to bools in flags[]:
        const HAS_INNER_UNDERSCORE: usize = 0;
        const HAS_LOWER: usize = 1;
        const HAS_UPPER: usize = 2;
        // _not_ conclusive/exhaustive - even if CANNOT_BE_CAMEL==false, id still may _not_ be in
        // CamelCase.
        const CANNOT_BE_CAMEL: usize = 3;

        let mut expecting_first_letter = true;
        for c in id.chars() {
            if c == '_' {
                // Ignoring leadins _ __ ___ etc.
                if expecting_first_letter {
                    has_leading_underscore = true;
                } else {
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
                    let leading_underscore_clause = if has_leading_underscore {
                        " (after the leading underscore(s))."
                    } else {
                        ""
                    };
                    let inner_underscore_clause = if flags[HAS_INNER_UNDERSCORE] {
                        " And it also has inner underscores(s)."
                    } else {
                        ""
                    };
                    Err(format!(
                        "{COULDNT_DETECT_FOR}{id}. It contains both lowercase and uppercase, but \
                        it doesn't start with uppercase{leading_underscore_clause}.\
                        {inner_underscore_clause}"
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
                        "{COULDNT_DETECT_FOR}{id}. It contains uppercase, no lowercase, and no inner underscore, but it could pass as either UPPERCASE or Camel (in Rust)."
                    ))
                }
            }
            (false, false) => Err(format!(
                "{COULDNT_DETECT_FOR}{id}. It doesn't contain either lowercase or uppercase."
            )),
        }
    }
}

#[cfg(test)]
mod test_parsing {
    use super::IdentNameConvention;
    use core::convert::TryFrom;

    #[test]
    fn ok_camel() {
        assert_eq!(
            TryFrom::try_from("GoodStruct"),
            Ok(IdentNameConvention::CamelCase)
        );
    }
    #[test]
    fn ok_camel_leading_underscore() {
        assert_eq!(
            TryFrom::try_from("_GoodStruct"),
            Ok(IdentNameConvention::CamelCase)
        );
    }
    #[test]
    fn ok_camel_digits() {
        assert_eq!(
            TryFrom::try_from("GoodStruct12"),
            Ok(IdentNameConvention::CamelCase)
        );
    }

    #[test]
    fn no_camel_all_letters_uppercase() {
        let result = IdentNameConvention::try_from("GOOD");
        assert!(matches!(result, Err(_)));
        assert!(result.unwrap_err().contains("either UPPERCASE or Camel"));
    }
    #[test]
    fn no_camel_underscore() {
        let result = IdentNameConvention::try_from("Good_Struct");
        assert!(matches!(result, Err(_)));
        assert!(result.unwrap_err().contains(
            "both lowercase and uppercase, but \
                        it also contains an inner underscore"
        ));
    }
    #[test]
    fn no_camel_first_letter_lowercase() {
        let result = IdentNameConvention::try_from("goodStruct");
        assert!(matches!(result, Err(_)));
        assert!(result.unwrap_err().contains(
            "both lowercase and uppercase, but \
                        it doesn't start with uppercase"
        ));
    }
}
