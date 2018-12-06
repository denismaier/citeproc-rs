use super::cite_context::*;
use super::ir::*;
use super::Proc;
use crate::input::{ Name, PersonName };
use crate::output::OutputFormat;
use crate::style::element::{
    Formatting, Name as NameEl, Names, EtAl, NameLabel,
};

// TODOS
// [ ] "editor translator" && editor==translator
// [ ] inherit name options from cs:style, cs:citation, and cs:bibliography
//     http://docs.citationstyles.org/en/stable/specification.html#inheritable-name-options

impl NameEl {
    fn inherit_from(parent: &Self) -> Self {
    }
    fn render<'c, 'r, O: OutputFormat>(
        &self,
        ctx: &mut CiteContext<'c, 'r, O>,
        var: &NameVariable<'r>,
    ) -> O::Build {
        let names = 
        let out = if let Some(names) = ctx.reference.names.get(var) {

        } else {
            None
        }
        let string = match self.form {
            DatePartForm::Year(ref form) => match form {
                YearForm::Long => format!("{}", date.year),
                YearForm::Short => format!("{:02}", date.year % 100),
            },
            DatePartForm::Month(ref form) => match form {
                // TODO: locale getter for months
                MonthForm::Long => format!("{}", MONTHS_LONG[date.month as usize]),
                MonthForm::Short => format!("{}", MONTHS_SHORT[date.month as usize]),
                MonthForm::Numeric => format!("{}", date.month),
                MonthForm::NumericLeadingZeros => format!("{:02}", date.month),
            },
            DatePartForm::Day(ref form) => match form {
                DayForm::Numeric => format!("{}", date.day),
                DayForm::NumericLeadingZeros => format!("{:02}", date.day),
                // TODO: implement ordinals
                DayForm::Ordinal => format!("{:02}", date.day),
            },
        };
        ctx.format.affixed(&string, &self.formatting, &self.affixes)
    }
}

impl<'c, 's: 'c> Proc<'c, 's> for IndependentDate {
    #[cfg_attr(feature = "flame_it", flame("Date"))]
    fn intermediate<'r, O>(&'s self, ctx: &mut CiteContext<'c, 'r, O>) -> IR<'c, O>
    where
        O: OutputFormat,
    {
        let fmt = ctx.format;
        // TODO: support locale-defined dates with Date,
        // and use an IndependentDate for unlocalized.
        let content = ctx
            .reference
            .date
            .get(&self.variable)
            .and_then(|d| d.single())
            .map(|val| {
                let each: Vec<_> = self
                    .date_parts
                    .iter()
                    .map(|dp| dp.render(ctx, &val))
                    .collect();
                let delim = &self.delimiter.0;
                fmt.group(
                    &[
                        fmt.plain(&self.affixes.prefix),
                        fmt.group(&each, delim, &self.formatting),
                        fmt.plain(&self.affixes.suffix),
                    ],
                    "",
                    &Formatting::default(),
                )

                // let string = format!("{}-{}-{}", val.year, val.month, val.day);
                // fmt.affixed(&string, &self.formatting, &self.affixes)
            });
        IR::Rendered(content)
    }
}