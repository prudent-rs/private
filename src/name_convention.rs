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
        let mut has_leading_underscore = false; // only used as extra info in error messages

        let mut has_inner_underscore = false;
        let mut has_lower = false;
        let mut has_upper = false;
        // _not_ conclusive/exhaustive - even if cannot_be_camel==false, id still may _not_ be in
        // CamelCase.
        let mut cannot_be_camel = false;

        let mut expecting_first_alphanumeric = true;

        let mut expecting_first_letter = true;
        for c in id.chars() {
            if c == '_' {
                if expecting_first_alphanumeric {
                    has_leading_underscore = true; // leading _ __ ___ etc.
                } else {
                    // Treating _ __ ___ etc. as an inner underscore
                    has_inner_underscore = true;
                }
                continue;
            }
            if c.is_alphanumeric() {
                if c.is_alphabetic() {
                    if c.is_lowercase() && !c.is_uppercase() {
                        has_lower = true;
                        if expecting_first_letter {
                            cannot_be_camel = true;
                        }
                    } else if c.is_uppercase() && !c.is_lowercase() {
                        has_upper = true;
                    }
                    expecting_first_letter = false;
                }
                expecting_first_alphanumeric = false;
            }
        }

        const COULDNT_DETECT_FOR: &str = "Naming convention couldn't be detected for ";
        match (has_lower, has_upper) {
            (true, true) => {
                if cannot_be_camel {
                    let leading_underscore_clause = if has_leading_underscore {
                        " (after the leading underscore(s))."
                    } else {
                        ""
                    };
                    let inner_underscore_clause = if has_inner_underscore {
                        " And it also has inner underscores(s)."
                    } else {
                        ""
                    };
                    Err(format!(
                        "{COULDNT_DETECT_FOR}{id}. It contains both lowercase and uppercase, but \
                        it doesn't start with uppercase{leading_underscore_clause}.\
                        {inner_underscore_clause}"
                    ))
                } else if has_inner_underscore {
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
                if has_inner_underscore {
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
    use syn::Ident;

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
    fn no_camel_first_letter_lowercase() {
        for id in ["goodStruct", "_goodStruct"] {
            let result = IdentNameConvention::try_from(id);
            assert!(matches!(result, Err(_)));
            assert!(result.unwrap_err().contains(
                "both lowercase and uppercase, but \
                            it doesn't start with uppercase"
            ));
        }
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
    fn no_camel_all_letters_uppercase() {
        let result = IdentNameConvention::try_from("GOOD");
        assert!(matches!(result, Err(_)));
        assert!(result.unwrap_err().contains("either UPPERCASE or Camel"));
    }

    #[test]
    fn no_lowercase_no_uppercase_underscore_and_numerical_only() {
        const ERR_PART: &str = "doesn't contain either lowercase or uppercase";
        let span = proc_macro2::Span::mixed_site();

        let _ = Ident::new("_1", span); // _1 **is** an acceptable ident
        assert!(
            IdentNameConvention::try_from("_1")
                .unwrap_err()
                .contains(ERR_PART)
        );

        // _½ (U+00BD) is refused by Ident::new
        //
        // ٢ is an Arabic-Indic digit (U+0662)
        let _ = Ident::new("_٢", span); // _1 **is** an acceptable ident
        assert!(
            IdentNameConvention::try_from("_٢")
                .unwrap_err()
                .contains(ERR_PART)
        );

        // Roman numeral Ⅻ = U+216C __is__ an (uppercase) letter!
    }
}
