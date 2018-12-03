use super::cite_context::*;
use super::{Proc, IR};
use crate::output::OutputFormat;
use crate::style::element::{Element, Formatting};

/// Tests whether the given variables (Appendix IV - Variables) contain numeric content. Content is
/// considered numeric if it solely consists of numbers. Numbers may have prefixes and suffixes
/// (“D2”, “2b”, “L2d”), and may be separated by a comma, hyphen, or ampersand, with or without
/// spaces (“2, 3”, “2-4”, “2 & 4”). For example, “2nd” tests “true” whereas “second” and “2nd
/// edition” test “false”.
pub fn convert_numeric<'a>(_value: &'a str) -> Result<i32, &'a str> {
    Ok(0)
}

pub fn sequence<'c, 's, 'r, O>(
    ctx: &CiteContext<'c, 'r, O>,
    f: &Formatting,
    delim: &str,
    els: &'c [Element],
) -> IR<'c, O>
where
    O: OutputFormat,
{
    let fmt = ctx.format;
    let mut dedup = vec![];
    let mut dups = vec![];
    for el in els.iter() {
        let pr = el.intermediate(ctx);
        if let IR::Rendered(Some(r)) = pr {
            dups.push(r);
        } else if let IR::Rendered(None) = pr {
        } else {
            if !dups.is_empty() {
                let r = IR::Rendered(Some(fmt.group(&dups, delim, &f)));
                dedup.push(r);
                dups.clear();
            }
            dedup.push(pr);
        }
    }
    if !dups.is_empty() {
        let r = IR::Rendered(Some(fmt.group(&dups, delim, &f)));
        dedup.push(r);
        dups.clear();
    }
    if dedup.len() == 1 {
        return dedup.into_iter().nth(0).unwrap();
    }
    if dedup.len() == 0 {
        return IR::Rendered(None);
    }
    IR::Seq(dedup)
}
