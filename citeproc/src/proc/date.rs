use super::disamb::{AddDisambTokens, DisambToken};
use super::group::GroupVars;
use super::ir::*;
use super::ProcDatabase;
use super::{CiteContext, IrState, Proc};
use crate::input::{Date, DateOrRange};
use crate::output::OutputFormat;
use csl::style::{
    BodyDate, DatePart, DatePartForm, DateParts, DayForm, IndependentDate, LocalizedDate,
    MonthForm, YearForm,
};

impl<'c, O> Proc<'c, O> for BodyDate
where
    O: OutputFormat,
{
    fn intermediate(
        &self,
        db: &impl ProcDatabase,
        state: &mut IrState,
        ctx: &CiteContext<'c, O>,
    ) -> IrSum<O>
    where
        O: OutputFormat,
    {
        // TODO: wrap BodyDate in a YearSuffixHook::Date() under certain conditions
        match *self {
            BodyDate::Indep(ref idate) => idate.intermediate(db, state, ctx),
            BodyDate::Local(ref ldate) => ldate.intermediate(db, state, ctx),
        }
    }
}

impl<'c, O> Proc<'c, O> for LocalizedDate
where
    O: OutputFormat,
{
    fn intermediate(
        &self,
        db: &impl ProcDatabase,
        state: &mut IrState,
        ctx: &CiteContext<'c, O>,
    ) -> IrSum<O>
    where
        O: OutputFormat,
    {
        let fmt = &ctx.format;
        let locale = db.default_locale();
        // TODO: handle missing
        let locale_date = locale.dates.get(&self.form).unwrap();
        // TODO: render date ranges
        // TODO: TextCase
        let date = ctx.reference.date.get(&self.variable).and_then(|r| {
            mask_range(r, &self.date_parts, state);
            r.single_or_first()
        });
        let content = date.map(|val| {
            let each: Vec<_> = locale_date
                .date_parts
                .iter()
                .filter(|dp| dp_matches(dp, self.parts_selector.clone()))
                .filter_map(|dp| dp_render(dp, db, state, ctx, &val))
                .collect();
            let delim = &locale_date.delimiter.0;
            fmt.affixed(fmt.group(each, delim, self.formatting), &self.affixes)
        });
        let gv = GroupVars::rendered_if(content.is_some());
        (IR::Rendered(content), gv)
    }
}

impl<'c, O> Proc<'c, O> for IndependentDate
where
    O: OutputFormat,
{
    fn intermediate(
        &self,
        db: &impl ProcDatabase,
        state: &mut IrState,
        ctx: &CiteContext<'c, O>,
    ) -> IrSum<O>
    where
        O: OutputFormat,
    {
        let fmt = &ctx.format;
        let content = ctx
            .reference
            .date
            .get(&self.variable)
            // TODO: render date ranges
            .and_then(|r| {
                mask_range(r, &self.date_parts, state);
                r.single_or_first()
            })
            .map(|val| {
                let each: Vec<_> = self
                    .date_parts
                    .iter()
                    .filter_map(|dp| dp_render(dp, db, state, ctx, &val))
                    .collect();
                let delim = &self.delimiter.0;
                fmt.affixed(fmt.group(each, delim, self.formatting), &self.affixes)
            });
        let gv = GroupVars::rendered_if(content.is_some());
        (IR::Rendered(content), gv)
    }
}

type DatePartAcc = (bool, bool, bool);

fn dp_fold(mut a: DatePartAcc, form: DatePartForm) -> DatePartAcc {
    match form {
        DatePartForm::Year(..) => a.0 = true,
        DatePartForm::Month(..) => a.1 = true,
        DatePartForm::Day(..) => a.2 = true,
    }
    a
}

fn mask(d: Date, date_parts: &[DatePart]) -> Date {
    let a = date_parts
        .iter()
        .map(|dp| dp.form)
        .fold((false, false, false), dp_fold);
    Date {
        year: if a.0 { d.year } else { 0 },
        month: if a.1 { d.month } else { 0 },
        day: if a.2 { d.day } else { 0 },
    }
}

fn mask_range(r: &DateOrRange, date_parts: &[DatePart], state: &mut IrState) {
    match *r {
        DateOrRange::Single(d) => {
            mask(d, date_parts).add_tokens(&mut state.tokens);
        }
        DateOrRange::Range(d1, d2) => {
            mask(d1, date_parts).add_tokens(&mut state.tokens);
            mask(d2, date_parts).add_tokens(&mut state.tokens);
        }
        DateOrRange::Literal(ref lit) => {
            state.tokens.insert(DisambToken::Str(lit.as_str().into()));
        }
    }
}

fn dp_matches(part: &DatePart, selector: DateParts) -> bool {
    match part.form {
        DatePartForm::Day(_) => selector == DateParts::YearMonthDay,
        DatePartForm::Month(..) => selector != DateParts::Year,
        DatePartForm::Year(_) => true,
    }
}

fn dp_render<'c, O: OutputFormat>(
    part: &DatePart,
    db: &impl ProcDatabase,
    _state: &mut IrState,
    ctx: &CiteContext<'c, O>,
    date: &Date,
) -> Option<O::Build> {
    let locale = db.default_locale();
    let string = match part.form {
        DatePartForm::Year(form) => match form {
            YearForm::Long => Some(format!("{}", date.year)),
            YearForm::Short => Some(format!("{:02}", date.year % 100)),
        },
        DatePartForm::Month(form, _strip_periods) => match form {
            MonthForm::Numeric => {
                if date.month == 0 || date.month > 12 {
                    None
                } else {
                    Some(format!("{}", date.month))
                }
            }
            MonthForm::NumericLeadingZeros => {
                if date.month == 0 || date.month > 12 {
                    None
                } else {
                    Some(format!("{:02}", date.month))
                }
            }
            _ => {
                // TODO: support seasons
                if date.month == 0 || date.month > 12 {
                    return None;
                }
                use csl::terms::*;
                let term_form = match form {
                    MonthForm::Long => TermForm::Long,
                    MonthForm::Short => TermForm::Short,
                    _ => TermForm::Long,
                };
                let sel = GenderedTermSelector::Month(
                    MonthTerm::from_u32(date.month).expect("TODO: support seasons"),
                    term_form,
                );
                Some(
                    locale
                        .gendered_terms
                        .get(&sel)
                        .map(|gt| gt.0.singular().to_string())
                        .unwrap_or_else(|| {
                            let fallback = if term_form == TermForm::Short {
                                MONTHS_SHORT
                            } else {
                                MONTHS_LONG
                            };
                            fallback[date.month as usize].to_string()
                        }),
                )
            }
        },
        DatePartForm::Day(form) => match form {
            DayForm::Numeric => {
                if date.day == 0 {
                    None
                } else {
                    Some(format!("{}", date.day))
                }
            }
            DayForm::NumericLeadingZeros => {
                if date.day == 0 {
                    None
                } else {
                    Some(format!("{:02}", date.day))
                }
            }
            // TODO: implement ordinals
            DayForm::Ordinal => {
                if date.day == 0 {
                    None
                } else {
                    Some(format!("{}ORD", date.day))
                }
            }
        },
    };
    string.map(|s| ctx.format.affixed_text(s, part.formatting, &part.affixes))
}

const MONTHS_SHORT: &[&str] = &[
    "undefined",
    "Jan",
    "Feb",
    "Mar",
    "Apr",
    "May",
    "Jun",
    "Jul",
    "Aug",
    "Sep",
    "Oct",
    "Nov",
    "Dec",
];

const MONTHS_LONG: &[&str] = &[
    "undefined",
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];
