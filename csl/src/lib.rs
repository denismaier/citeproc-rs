//! Describes the `<style>` element and all its children, and parses it from an XML tree.

pub use string_cache::DefaultAtom as Atom;

#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate strum_macros;

use std::sync::Arc;

pub(crate) mod attr;
pub use self::attr::GetAttribute;
pub mod error;
pub mod locale;
pub mod style;
pub mod terms;
pub mod variables;
pub mod version;

use self::attr::*;
use self::error::{CslError, InvalidCsl, NeedVarType, PartitionResults, StyleError};
use self::locale::*;
use self::style::*;
use self::terms::*;
use self::version::*;
use fnv::FnvHashMap;
use roxmltree::{Children, Node};
use semver::VersionReq;

#[derive(Default)]
pub(crate) struct ParseInfo {
    features: Features,
}

pub(crate) type FromNodeResult<T> = Result<T, CslError>;

pub(crate) trait FromNode
where
    Self: Sized,
{
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self>;
}

trait AttrChecker
where
    Self: Sized,
{
    fn filter_attribute(attr: &str) -> bool;
    fn is_on_node<'a>(node: &'a Node) -> bool {
        node.attributes()
            .iter()
            .filter(|a| Self::filter_attribute(a.name()))
            .next()
            != None
    }
    fn relevant_attrs<'a>(node: &'a Node) -> Vec<String> {
        node.attributes()
            .iter()
            .filter(|a| Self::filter_attribute(a.name()))
            .map(|a| String::from(a.name()))
            .collect()
    }
}

impl AttrChecker for Formatting {
    fn filter_attribute(attr: &str) -> bool {
        attr == "font-style"
            || attr == "font-variant"
            || attr == "font-weight"
            || attr == "text-decoration"
            || attr == "vertical-alignment"
            || attr == "strip-periods"
    }
}

impl<T> FromNode for Option<T>
where
    T: AttrChecker + FromNode,
{
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        if T::is_on_node(node) {
            Ok(Some(T::from_node(node, info)?))
        } else {
            Ok(None)
        }
    }
}

impl FromNode for Affixes {
    fn from_node(node: &Node, _info: &ParseInfo) -> FromNodeResult<Self> {
        Ok(Affixes {
            prefix: attribute_atom(node, "prefix"),
            suffix: attribute_atom(node, "suffix"),
        })
    }
}

impl FromNode for RangeDelimiter {
    fn from_node(node: &Node, _info: &ParseInfo) -> FromNodeResult<Self> {
        Ok(RangeDelimiter(attribute_atom_default(
            node,
            "range-delimiter",
            "\u{2013}".into(),
        )))
    }
}

impl AttrChecker for RangeDelimiter {
    fn filter_attribute(attr: &str) -> bool {
        attr == "range-delimiter"
    }
}

impl AttrChecker for Affixes {
    fn filter_attribute(attr: &str) -> bool {
        attr == "prefix" || attr == "suffix"
    }
}

impl FromNode for Delimiter {
    fn from_node(node: &Node, _info: &ParseInfo) -> FromNodeResult<Self> {
        Ok(Delimiter(attribute_atom(node, "delimiter")))
    }
}

impl FromNode for Formatting {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        Ok(Formatting {
            font_style: attribute_optional(node, "font-style", info)?,
            font_variant: attribute_optional(node, "font-variant", info)?,
            font_weight: attribute_optional(node, "font-weight", info)?,
            text_decoration: attribute_optional(node, "text-decoration", info)?,
            vertical_alignment: attribute_optional(node, "vertical-alignment", info)?,
            // TODO: carry options from root
            // hyperlink: String::from(""),
        })
    }
}

impl FromNode for Citation {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        // TODO: remove collect() using Peekable
        let layouts: Vec<_> = node
            .children()
            .filter(|n| n.has_tag_name("layout"))
            .collect();
        if layouts.len() != 1 {
            return Ok(Err(InvalidCsl::new(
                node,
                "<citation> must contain exactly one <layout>",
            ))?);
        }
        let layout_node = layouts[0];
        Ok(Citation {
            disambiguate_add_names: attribute_bool(node, "disambiguate-add-names", false)?,
            disambiguate_add_givenname: attribute_bool(node, "disambiguate-add-givenname", false)?,
            givenname_disambiguation_rule: attribute_optional(
                node,
                "givenname-disambiguation-rule",
                info
            )?,
            disambiguate_add_year_suffix: attribute_bool(
                node,
                "disambiguate-add-year-suffix",
                false,
            )?,
            layout: Layout::from_node(&layout_node, info)?,
            name_inheritance: Name::from_node(&node, info)?,
            names_delimiter: node
                .attribute("names-delimiter")
                .map(Atom::from)
                .map(Delimiter),
        })
    }
}

impl FromNode for Sort {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        Ok(Sort {
            keys: node
                .children()
                .filter(|n| n.has_tag_name("key"))
                .map(|node| SortKey::from_node(&node, info))
                .partition_results()?,
        })
    }
}

impl FromNode for SortKey {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        Ok(SortKey {
            sort_source: SortSource::from_node(node, info)?,
            names_min: attribute_option_int(node, "names-min")?,
            names_use_first: attribute_option_int(node, "names-min")?,
            names_use_last: attribute_option_int(node, "names-min")?,
            sort: attribute_option(node, "sort", info)?,
        })
    }
}

impl FromNode for SortSource {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        let macro_ = node.attribute("macro");
        let variable = node.attribute("variable");
        let err = "<key> must have either a `macro` or `variable` attribute";
        match (macro_, variable) {
            (Some(mac), None) => Ok(SortSource::Macro(mac.into())),
            (None, Some(_)) => Ok(SortSource::Variable(attribute_var_type(
                node,
                "variable",
                NeedVarType::Any,
                info,
            )?)),
            _ => Err(InvalidCsl::new(node, err).into()),
        }
    }
}

impl FromNode for Bibliography {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        // TODO: layouts matching locales in CSL-M mode
        // TODO: make sure that all elements are under the control of a display attribute
        //       if any of them are
        let layouts: Vec<_> = node
            .children()
            .filter(|n| n.has_tag_name("layout"))
            .collect();
        if layouts.len() != 1 {
            return Ok(Err(InvalidCsl::new(
                node,
                "<citation> must contain exactly one <layout>",
            ))?);
        }
        let layout_node = layouts[0];
        let line_spaces = attribute_int(node, "line-spaces", 1)?;
        if line_spaces < 1 {
            return Err(InvalidCsl::new(node, "line-spaces must be >= 1").into());
        }
        let entry_spacing = attribute_int(node, "entry-spacing", 1)?;
        let sorts: Vec<_> = node.children().filter(|n| n.has_tag_name("sort")).collect();
        if sorts.len() > 1 {
            return Ok(Err(InvalidCsl::new(
                node,
                "<bibliography> can only contain one <sort>",
            ))?);
        }
        let sort = if sorts.len() == 0 {
            None
        } else {
            Some(Sort::from_node(&sorts[0], info)?)
        };
        Ok(Bibliography {
            sort,
            layout: Layout::from_node(&layout_node, info)?,
            hanging_indent: attribute_bool(node, "hanging-indent", false)?,
            second_field_align: attribute_option(node, "second-field-align", info)?,
            line_spaces,
            entry_spacing,
            name_inheritance: Name::from_node(&node, info)?,
            subsequent_author_substitute: attribute_option_atom(
                node,
                "subsequent-author-substitute",
            ),
            subsequent_author_substitute_rule: attribute_optional(
                node,
                "subsequent-author-substitute-rule",
                info
            )?,
            names_delimiter: node
                .attribute("names-delimiter")
                .map(Atom::from)
                .map(Delimiter),
        })
    }
}

impl FromNode for Layout {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        let elements = node
            .children()
            .filter(|n| n.is_element())
            .map(|el| Element::from_node(&el, info))
            .partition_results()?;
        Ok(Layout {
            formatting: Option::from_node(node, info)?,
            affixes: Affixes::from_node(node, info)?,
            delimiter: Delimiter::from_node(node, info)?,
            locale: attribute_array(node, "locale", info)?,
            elements,
        })
    }
}

impl FromNode for TextTermSelector {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        use self::terms::AnyTermName::*;
        // we already know term is on there
        let t = attribute_required(node, "term", info)?;
        match t {
            Number(v) => Ok(TextTermSelector::Gendered(GenderedTermSelector::Number(
                v,
                TermForm::from_node(node, info)?,
            ))),
            Month(t) => Ok(TextTermSelector::Gendered(GenderedTermSelector::Month(
                t,
                TermForm::from_node(node, info)?,
            ))),
            Loc(t) => Ok(TextTermSelector::Gendered(GenderedTermSelector::Locator(
                t,
                TermForm::from_node(node, info)?,
            ))),
            Misc(t) => Ok(TextTermSelector::Simple(SimpleTermSelector::Misc(
                t,
                TermFormExtended::from_node(node, info)?,
            ))),
            Season(t) => Ok(TextTermSelector::Simple(SimpleTermSelector::Season(
                t,
                TermForm::from_node(node, info)?,
            ))),
            Quote(t) => Ok(TextTermSelector::Simple(SimpleTermSelector::Quote(
                t,
                TermForm::from_node(node, info)?,
            ))),
            Role(t) => Ok(TextTermSelector::Role(RoleTermSelector(
                t,
                TermFormExtended::from_node(node, info)?,
            ))),
            Ordinal(_) => {
                Err(InvalidCsl::new(node, "you cannot render an ordinal term directly").into())
            }
        }
    }
}

fn text_el(node: &Node, info: &ParseInfo) -> Result<Element, CslError> {
    let macro_ = node.attribute("macro");
    let value = node.attribute("value");
    let variable = node.attribute("variable");
    let term = node.attribute("term");
    let invalid = "<text> without a `variable`, `macro`, `term` or `value` is invalid";

    let source = match (macro_, value, variable, term) {
        (Some(mac), None, None, None) => TextSource::Macro(mac.into()),
        (None, Some(val), None, None) => TextSource::Value(val.into()),
        (None, None, Some(___), None) => TextSource::Variable(
            attribute_var_type(node, "variable", NeedVarType::TextVariable, info)?,
            attribute_optional(node, "form", info)?,
        ),
        (None, None, None, Some(___)) => TextSource::Term(
            TextTermSelector::from_node(node, info)?,
            attribute_bool(node, "plural", false)?,
        ),
        _ => return Err(InvalidCsl::new(node, invalid).into()),
    };

    let formatting = Option::from_node(node, info)?;
    let affixes = Affixes::from_node(node, info)?;
    let quotes = attribute_bool(node, "quotes", false)?;
    let strip_periods = attribute_bool(node, "strip-periods", false)?;
    let text_case = TextCase::from_node(node, info)?;
    let display = attribute_option(node, "display", info)?;

    Ok(Element::Text(
        source,
        formatting,
        affixes,
        quotes,
        strip_periods,
        text_case,
        display,
    ))
}

fn label_el(node: &Node, info: &ParseInfo) -> Result<Element, CslError> {
    Ok(Element::Label(
        attribute_var_type(node, "variable", NeedVarType::NumberVariable, info)?,
        attribute_optional(node, "form", info)?,
        Option::from_node(node, info)?,
        Affixes::from_node(node, info)?,
        attribute_bool(node, "strip-periods", false)?,
        TextCase::from_node(node, info)?,
        attribute_optional(node, "plural", info)?,
    ))
}

fn number_el(node: &Node, info: &ParseInfo) -> Result<Element, CslError> {
    Ok(Element::Number(
        attribute_var_type(node, "variable", NeedVarType::NumberVariable, info)?,
        attribute_optional(node, "form", info)?,
        Option::from_node(node, info)?,
        Affixes::from_node(node, info)?,
        attribute_optional(node, "plural", info)?,
        attribute_option(node, "display", info)?,
    ))
}

impl FromNode for Group {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        let elements = node
            .children()
            .filter(|n| n.is_element())
            .map(|el| Element::from_node(&el, info))
            .partition_results()?;
        Ok(Group {
            elements,
            formatting: Option::from_node(node, info)?,
            delimiter: Delimiter::from_node(node, info)?,
            affixes: Affixes::from_node(node, info)?,
            display: attribute_option(node, "display", info)?,
            // TODO: CSL-M only
            is_parallel: attribute_bool(node, "is-parallel", false)?,
        })
    }
}

impl FromNode for Else {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        let elements = node
            .children()
            .filter(|n| n.is_element())
            .map(|el| Element::from_node(&el, info))
            .partition_results()?;
        Ok(Else(elements))
    }
}

impl FromNode for Match {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        Ok(attribute_optional(node, "match", info)?)
    }
}

#[derive(Debug)]
enum ConditionError {
    Unconditional(InvalidCsl),
    Invalid(CslError),
}

impl ConditionError {
    fn into_inner(self) -> CslError {
        match self {
            ConditionError::Unconditional(e) => CslError(vec![e]),
            ConditionError::Invalid(e) => e,
        }
    }
}

impl From<InvalidCsl> for ConditionError {
    fn from(err: InvalidCsl) -> Self {
        ConditionError::Invalid(CslError::from(err))
    }
}

impl From<CslError> for ConditionError {
    fn from(err: CslError) -> Self {
        ConditionError::Invalid(err)
    }
}

impl From<Vec<CslError>> for ConditionError {
    fn from(err: Vec<CslError>) -> Self {
        ConditionError::Invalid(CslError::from(err))
    }
}

impl Condition {
    fn from_node_custom(node: &Node, info: &ParseInfo) -> Result<Self, ConditionError> {
        let (has_year_only, has_month_or_season, has_day) = if info.features.condition_date_parts {
            (
                attribute_array_var(node, "has-year-only", NeedVarType::CondDate, info)?,
                attribute_array_var(node, "has-month-or-season", NeedVarType::CondDate, info)?,
                attribute_array_var(node, "has-day", NeedVarType::CondDate, info)?,
            )
        } else {
            Default::default()
        };
        let cond = Condition {
            match_type: Match::from_node(node, info)?,
            jurisdiction: attribute_option_atom(node, "jurisdiction"),
            subjurisdictions: attribute_option_int(node, "subjurisdictions")?,
            context: attribute_option(node, "context", info)?,
            disambiguate: attribute_option_bool(node, "disambiguate")?,
            variable: attribute_array_var(node, "variable", NeedVarType::Any, info)?,
            position: attribute_array_var(node, "position", NeedVarType::CondPosition, info)?,
            is_plural: attribute_array_var(node, "is-plural", NeedVarType::CondIsPlural, info)?,
            csl_type: attribute_array_var(node, "type", NeedVarType::CondType, info)?,
            locator: attribute_array_var(node, "locator", NeedVarType::CondLocator, info)?,
            is_uncertain_date: attribute_array_var(
                node,
                "is-uncertain-date",
                NeedVarType::CondDate,
                info,
            )?,
            is_numeric: attribute_array_var(node, "is-numeric", NeedVarType::Any, info)?,
            has_year_only,
            has_month_or_season,
            has_day,
        };
        // technically, only a match="..." on an <if> is ignored when a <conditions> block is
        // present, but that's ok
        if cond.is_empty() {
            Err(ConditionError::Unconditional(InvalidCsl::new(
                node,
                "Unconditional <choose> branch",
            )))
        } else {
            Ok(cond)
        }
    }
}

impl FromNode for Conditions {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        let match_type = attribute_required(node, "match", info)?;
        let conds = node
            .children()
            .filter(|n| n.has_tag_name("condition"))
            .map(|el| Condition::from_node_custom(&el, info).map_err(|e| e.into_inner()))
            .partition_results()?;
        if conds.is_empty() {
            Err(InvalidCsl::new(node, "Unconditional <choose> branch").into())
        } else {
            Ok(Conditions(match_type, conds))
        }
    }
}

// TODO: need context to determine if the CSL-M syntax can be used
impl FromNode for IfThen {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        let tag = "if or else-if";

        // CSL 1.0.1 <if match="MMM" vvv ></if> equivalent to
        // CSL-M <if><conditions match="all"><condition match="MMM" vvv /></conditions></if>

        // these are 'inline' ones directly on an if / if-else node
        let own_conditions: Result<Conditions, ConditionError> =
            Condition::from_node_custom(node, info).map(|c| Conditions(Match::All, vec![c]));

        // these are child nodes
        let sub_conditions: Result<Option<Conditions>, CslError> = if info.features.conditions {
            // TODO: only accept <conditions> in head position
            max1_child(tag, "conditions", node.children(), info)
        } else {
            if let Some(invalid) = node
                .children()
                .filter(|n| n.has_tag_name("conditions"))
                .nth(0)
            {
                Err(InvalidCsl::new(
                    &invalid,
                    "You must opt-in to the `conditions` feature to use <conditions>",
                )
                .into())
            } else {
                Ok(None)
            }
        };

        use self::ConditionError::*;

        let conditions: Conditions = (match (own_conditions, sub_conditions) {
            // just an if block
            (Ok(own), Ok(None)) => Ok(own),
            // just an if block, that failed
            (Err(e), Ok(None)) => Err(e.into_inner()),
            // no conds on if block, but error in <conditions>
            (Err(Unconditional(_)), Err(esub)) => Err(esub),
            // no conds on if block, but <conditions> present
            (Err(Unconditional(_)), Ok(Some(sub))) => Ok(sub),
            // if block has conditions, and <conditions> was also present
            (Err(Invalid(_)), Ok(Some(_)))
            | (Err(Invalid(_)), Err(_))
            | (Ok(_), Ok(Some(_)))
            | (Ok(_), Err(_)) => Ok(Err(InvalidCsl::new(
                node,
                &format!(
                    "{} can only have its own conditions OR a <conditions> block",
                    tag
                ),
            ))?),
        })?;
        let elements = node
            .children()
            .filter(|n| n.is_element() && !n.has_tag_name("conditions"))
            .map(|el| Element::from_node(&el, info))
            .partition_results()?;
        Ok(IfThen(conditions, elements))
    }
}

fn choose_el(node: &Node, info: &ParseInfo) -> Result<Element, CslError> {
    let mut if_block: Option<IfThen> = None;
    let mut elseifs = vec![];
    let mut else_block = Else(vec![]);
    let mut seen_if = false;
    let mut seen_else = false;

    let unrecognised = |el, tag| {
        if tag == "if" || tag == "else-if" || tag == "else" {
            return Ok(Err(InvalidCsl::new(
                el,
                &format!(
                    "<choose> elements out of order; found <{}> in wrong position",
                    tag
                ),
            ))?);
        }
        Ok(Err(InvalidCsl::new(
            el,
            &format!("Unrecognised element {} in <choose>", tag),
        ))?)
    };

    for el in node.children().filter(|n| n.is_element()) {
        // TODO: figure out why doing this without a clone causes 'borrowed value does not
        // live long enough' problems.
        let tn = el.tag_name();
        let tag = tn.name().to_owned();
        if !seen_if {
            if tag == "if" {
                seen_if = true;
                if_block = Some(IfThen::from_node(&el, info)?);
            } else {
                return Err(InvalidCsl::new(
                    &el,
                    "<choose> blocks must begin with an <if>",
                ))?;
            }
        } else if !seen_else {
            if tag == "else-if" {
                elseifs.push(IfThen::from_node(&el, info)?);
            } else if tag == "else" {
                seen_else = true;
                else_block = Else::from_node(&el, info)?;
            } else {
                return unrecognised(&el, tag);
            }
        } else {
            return unrecognised(&el, tag);
        }
    }

    let _if = if_block.ok_or_else(|| InvalidCsl::new(node, "<choose> blocks must have an <if>"))?;

    Ok(Element::Choose(Arc::new(Choose(_if, elseifs, else_block))))
}

fn max1_child<T: FromNode>(
    parent_tag: &str,
    child_tag: &str,
    els: Children,
    info: &ParseInfo,
) -> Result<Option<T>, CslError> {
    // TODO: remove the allocation here, with a cloned iterator / peekable
    let subst_els: Vec<_> = els.filter(|n| n.has_tag_name(child_tag)).collect();
    if subst_els.len() > 1 {
        return Err(InvalidCsl::new(
            &subst_els[1],
            &format!(
                "There can only be one <{}> in a <{}> block.",
                child_tag, parent_tag
            ),
        ))?;
    }
    let substs = subst_els
        .iter()
        .map(|el| T::from_node(&el, info))
        .partition_results()?;
    let substitute = substs.into_iter().nth(0);
    Ok(substitute)
}

impl AttrChecker for TextCase {
    fn filter_attribute(attr: &str) -> bool {
        attr == "text-case"
    }
}

impl FromNode for TextCase {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        Ok(attribute_optional(node, "text-case", info)?)
    }
}

fn disallow_default<T: Default + FromNode + AttrChecker>(
    node: &Node,
    disallow: bool,
    info: &ParseInfo,
) -> Result<T, CslError> {
    if disallow {
        if T::is_on_node(node) {
            Err(InvalidCsl::new(
                node,
                &format!(
                    "Disallowed attribute on node: {:?}",
                    T::relevant_attrs(node)
                ),
            ))?
        } else {
            Ok(T::default())
        }
    } else {
        T::from_node(node, info)
    }
}

impl DatePart {
    fn from_node_dp(node: &Node, full: bool, info: &ParseInfo) -> FromNodeResult<Self> {
        let name: DatePartName = attribute_required(node, "name", info)?;
        let form = match name {
            DatePartName::Year => DatePartForm::Year(attribute_optional(node, "form", info)?),
            DatePartName::Month => DatePartForm::Month(
                attribute_optional(node, "form", info)?,
                attribute_bool(node, "strip-periods", false)?,
            ),
            DatePartName::Day => DatePartForm::Day(attribute_optional(node, "form", info)?),
        };
        Ok(DatePart {
            form,
            // affixes not allowed in a locale date
            affixes: disallow_default(node, !full, info)?,
            formatting: Option::from_node(node, info)?,
            text_case: TextCase::from_node(node, info)?,
            range_delimiter: RangeDelimiter::from_node(node, info)?,
        })
    }
}

impl FromNode for IndependentDate {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        let elements = node
            .children()
            .filter(|n| n.is_element() && n.has_tag_name("date-part"))
            .map(|el| DatePart::from_node_dp(&el, true, info))
            .partition_results()?;
        Ok(IndependentDate {
            variable: attribute_var_type(node, "variable", NeedVarType::Date, info)?,
            date_parts: elements,
            text_case: TextCase::from_node(node, info)?,
            affixes: Affixes::from_node(node, info)?,
            formatting: Option::from_node(node, info)?,
            display: attribute_option(node, "display", info)?,
            delimiter: Delimiter::from_node(node, info)?,
        })
    }
}

impl FromNode for LocalizedDate {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        let elements = node
            .children()
            .filter(|n| n.is_element() && n.has_tag_name("date-part"))
            // no affixes if you're calling a locale date
            .map(|el| DatePart::from_node_dp(&el, false, info))
            .partition_results()?;
        Ok(LocalizedDate {
            variable: attribute_var_type(node, "variable", NeedVarType::Date, info)?,
            parts_selector: attribute_optional(node, "date-parts", info)?,
            date_parts: elements,
            form: attribute_required(node, "form", info)?,
            affixes: Affixes::from_node(node, info)?,
            formatting: Option::from_node(node, info)?,
            display: attribute_option(node, "display", info)?,
            text_case: TextCase::from_node(node, info)?,
        })
    }
}

impl FromNode for BodyDate {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        if node.has_attribute("form") {
            Ok(BodyDate::Local(LocalizedDate::from_node(node, info)?))
        } else {
            Ok(BodyDate::Indep(IndependentDate::from_node(node, info)?))
        }
    }
}

impl FromNode for Element {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        match node.tag_name().name() {
            "text" => Ok(text_el(node, info)?),
            "label" => Ok(label_el(node, info)?),
            "group" => Ok(Element::Group(Group::from_node(node, info)?)),
            "number" => Ok(number_el(node, info)?),
            "names" => Ok(Element::Names(Arc::new(Names::from_node(node, info)?))),
            "choose" => Ok(choose_el(node, info)?),
            "date" => Ok(Element::Date(Arc::new(BodyDate::from_node(node, info)?))),
            _ => Err(InvalidCsl::new(node, "Unrecognised node."))?,
        }
    }
}

fn get_toplevel<'a, 'd: 'a>(
    root: &Node<'a, 'd>,
    nodename: &'static str,
) -> Result<Node<'a, 'd>, CslError> {
    // TODO: remove collect()
    let matches: Vec<_> = root
        .children()
        .filter(|n| n.has_tag_name(nodename))
        .collect();
    if matches.len() > 1 {
        Ok(Err(InvalidCsl::new(
            &root,
            &format!("Cannot have more than one <{}>", nodename),
        ))?)
    } else {
        // move matches into its first item
        Ok(matches
            .into_iter()
            .nth(0)
            .ok_or_else(|| InvalidCsl::new(&root, &format!("Must have one <{}>", nodename)))?)
    }
}

impl FromNode for MacroMap {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        // TODO: remove collect()
        let elements: Result<Vec<_>, _> = node
            .children()
            .filter(|n| n.is_element())
            .map(|el| Element::from_node(&el, info))
            .collect();
        let name = match node.attribute("name") {
            Some(n) => n,
            None => {
                return Ok(Err(InvalidCsl::new(
                    node,
                    "Macro must have a 'name' attribute.",
                ))?);
            }
        };
        Ok(MacroMap {
            name: name.into(),
            elements: elements?,
        })
    }
}

impl FromNode for Names {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        let name = max1_child("names", "name", node.children(), info)?;
        let institution = max1_child("names", "institution", node.children(), info)?;
        let et_al = max1_child("names", "et-al", node.children(), info)?;
        let label = max1_child("names", "label", node.children(), info)?;
        let with = max1_child("names", "with", node.children(), info)?;
        let substitute = max1_child("names", "substitute", node.children(), info)?;
        Ok(Names {
            variables: attribute_array_var(node, "variable", NeedVarType::Name, info)?,
            name,
            institution,
            with,
            et_al,
            label,
            substitute,
            affixes: Affixes::from_node(node, info)?,
            formatting: Option::from_node(node, info)?,
            display: attribute_option(node, "display", info)?,
            delimiter: node.attribute("delimiter").map(Atom::from).map(Delimiter),
        })
    }
}

impl FromNode for Institution {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        use crate::style::InstitutionUseFirst::*;
        let uf = node.attribute("use-first");
        let suf = node.attribute("substitute-use-first");
        let invalid = "<institution> may only use one of `use-first` or `substitute-use-first`";
        let use_first = match (uf, suf) {
            (Some(_), None) => Some(Normal(attribute_int(node, "use-first", 1)?)),
            (None, Some(_)) => Some(Substitute(attribute_int(node, "substitute-use-first", 1)?)),
            (None, None) => None,
            _ => return Err(InvalidCsl::new(node, invalid).into()),
        };

        let institution_parts = node
            .children()
            .filter(|n| n.is_element() && n.has_tag_name("institution-part"))
            .map(|el| InstitutionPart::from_node(&el, info))
            .partition_results()?;

        Ok(Institution {
            and: attribute_option(node, "and", info)?,
            delimiter: node.attribute("delimiter").map(Atom::from).map(Delimiter),
            use_first,
            use_last: attribute_option_int(node, "use-last")?,
            reverse_order: attribute_bool(node, "reverse-order", false)?,
            parts_selector: attribute_optional(node, "institution-parts", info)?,
            institution_parts,
        })
    }
}

impl FromNode for InstitutionPart {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        Ok(InstitutionPart {
            name: InstitutionPartName::from_node(node, info)?,
            formatting: Option::from_node(node, info)?,
            affixes: Affixes::from_node(node, info)?,
            strip_periods: attribute_bool(node, "strip-periods", false)?,
        })
    }
}

impl FromNode for InstitutionPartName {
    fn from_node(node: &Node, _info: &ParseInfo) -> FromNodeResult<Self> {
        match node.attribute("name") {
            Some("long") => Ok(InstitutionPartName::Long(attribute_bool(
                node, "if-short", false,
            )?)),
            Some("short") => Ok(InstitutionPartName::Short),
            Some(ref val) => Err(InvalidCsl::attr_val(node, "name", val).into()),
            None => Err(InvalidCsl::missing(node, "name").into()),
        }
    }
}

impl FromNode for Name {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        // for inheriting from cs:style/cs:citation/cs:bibliography
        let mut delim_attr = "delimiter";
        let mut form_attr = "form";
        let mut name_part_given = None;
        let mut name_part_family = None;
        if node.tag_name().name() != "name" {
            delim_attr = "name-delimiter";
            form_attr = "name-form";
        } else {
            let parts = |val| {
                node.children()
                    .filter(move |el| {
                        el.is_element()
                            && el.has_tag_name("name-part")
                            && el.attribute("name") == Some(val)
                    })
                    .map(|el| NamePart::from_node(&el, info))
                    .filter_map(|np| np.ok())
            };
            name_part_given = parts("given").nth(0);
            name_part_family = parts("family").nth(0);
        }
        Ok(Name {
            and: attribute_option(node, "and", info)?,
            delimiter: node.attribute(delim_attr).map(Atom::from).map(Delimiter),
            delimiter_precedes_et_al: attribute_option(node, "delimiter-precedes-et-al", info)?,
            delimiter_precedes_last: attribute_option(node, "delimiter-precedes-last", info)?,
            et_al_min: attribute_option_int(node, "et-al-min")?,
            et_al_use_last: attribute_option_bool(node, "et-al-use-last")?,
            et_al_use_first: attribute_option_int(node, "et-al-use-first")?,
            et_al_subsequent_min: attribute_option_int(node, "et-al-subsequent-min")?,
            et_al_subsequent_use_first: attribute_option_int(node, "et-al-subsequent-use-first")?,
            form: attribute_option(node, form_attr, info)?,
            initialize: attribute_option_bool(node, "initialize")?,
            initialize_with: attribute_option_atom(node, "initialize-with"),
            name_as_sort_order: attribute_option(node, "name-as-sort-order", info)?,
            sort_separator: attribute_option_atom(node, "sort-separator"),
            formatting: Option::from_node(node, info)?,
            affixes: Affixes::from_node(node, info)?,
            name_part_given,
            name_part_family,
        })
    }
}

impl FromNode for NameEtAl {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        Ok(NameEtAl {
            term: attribute_string(node, "term"),
            formatting: Option::from_node(node, info)?,
        })
    }
}

impl FromNode for NameWith {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        Ok(NameWith {
            formatting: Option::from_node(node, info)?,
            affixes: Affixes::from_node(node, info)?,
        })
    }
}

impl FromNode for NamePart {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        Ok(NamePart {
            name: attribute_required(node, "name", info)?,
            text_case: TextCase::from_node(node, info)?,
            formatting: Option::from_node(node, info)?,
            affixes: Affixes::from_node(node, info)?,
        })
    }
}

impl FromNode for NameLabel {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        Ok(NameLabel {
            form: attribute_optional(node, "form", info)?,
            formatting: Option::from_node(node, info)?,
            delimiter: Delimiter::from_node(node, info)?,
            plural: attribute_optional(node, "plural", info)?,
            strip_periods: attribute_bool(node, "strip-periods", false)?,
        })
    }
}

impl FromNode for Substitute {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        let els = node
            .children()
            .filter(|n| n.is_element() && n.has_tag_name("name"))
            .map(|el| Element::from_node(&el, info))
            .partition_results()?;
        Ok(Substitute(els))
    }
}

struct TextContent(Option<String>);

impl FromNode for TextContent {
    fn from_node(node: &Node, _info: &ParseInfo) -> FromNodeResult<Self> {
        let opt_s = node.text().map(String::from);
        Ok(TextContent(opt_s))
    }
}

impl FromNode for TermPlurality {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        let always: Option<String> = TextContent::from_node(node, info)?
            .0
            .map(|s| s.trim().into());
        let single: Option<TextContent> = max1_child("term", "single", node.children(), info)?;
        let multiple: Option<TextContent> = max1_child("term", "multiple", node.children(), info)?;
        let msg = "<term> must contain either only text content or both <single> and <multiple>";
        match (always, single, multiple) {
            // empty term is valid
            (None, None, None) => Ok(TermPlurality::Invariant("".into())),
            // <term>plain text content</term>
            (Some(a), None, None) => Ok(TermPlurality::Invariant(a)),
            // <term> ANYTHING <single> s </single> <multiple> m </multiple></term>
            (_, Some(s), Some(m)) => Ok(TermPlurality::Pluralized {
                single: s.0.unwrap_or_else(|| "".into()),
                multiple: m.0.unwrap_or_else(|| "".into()),
            }),
            // had one of <single> or <multiple>, but not the other
            _ => Ok(Err(InvalidCsl::new(node, msg))?),
        }
    }
}

impl FromNode for OrdinalMatch {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        Ok(attribute_optional(node, "match", info)?)
    }
}

impl FromNode for TermFormExtended {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        Ok(attribute_optional(node, "form", info)?)
    }
}

impl FromNode for TermForm {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        Ok(attribute_optional(node, "form", info)?)
    }
}

impl FromNode for CslVersionReq {
    fn from_node(node: &Node, info: &ParseInfo) -> FromNodeResult<Self> {
        let version = attribute_string(node, "version");
        let variant: CslVariant;
        let req = if version.ends_with("mlz1") {
            variant = CslVariant::CslM;
            VersionReq::parse(version.trim_end_matches("mlz1")).map_err(|_| {
                InvalidCsl::new(
                    node,
                    &format!(
r#"unsupported "1.1mlz1"-style version string (use variant="csl-m" version="1.x", for example)"#),
                )
            })?
        } else {
            // TODO: bootstrap attribute_optional with a dummy CslVariant::Csl
            variant = attribute_optional(node, "variant", info)?;
            VersionReq::parse(&version).map_err(|_| {
                InvalidCsl::new(
                    node,
                    &format!("could not parse version string \"{}\"", &version),
                )
            })?
        };
        let supported = match &variant {
            CslVariant::Csl => &*COMPILED_VERSION,
            CslVariant::CslM => &*COMPILED_VERSION_M,
        };
        if !req.matches(supported) {
            return Err(InvalidCsl::new(
                    node,
                    &format!(
                        "Unsupported version for variant {:?}: \"{}\". This engine supports {} and later.",
                            variant,
                            req,
                            supported)).into());
        }
        Ok(CslVersionReq(variant, req))
    }
}

impl FromNode for Features {
    fn from_node(node: &Node, _info: &ParseInfo) -> FromNodeResult<Self> {
        let input = node
            .children()
            .filter(|n| n.is_element() && n.has_tag_name("feature"))
            .filter_map(|el| el.attribute("name"));
        read_features(input).map_err(|s| {
            InvalidCsl::new(node, &format!("Unrecognised feature flag `{}`", s)).into()
        })
    }
}

impl FromNode for Style {
    fn from_node(node: &Node, default_info: &ParseInfo) -> FromNodeResult<Self> {
        let version_req = CslVersionReq::from_node(node, default_info)?;
        // let info_node = get_toplevel(&doc, "info")?;
        let mut macros = FnvHashMap::default();
        let mut locale_overrides = FnvHashMap::default();
        let mut errors: Vec<CslError> = Vec::new();

        // Check features first, so we know when parsing the rest which are enabled
        let feat_matches: Vec<_> = node
            .children()
            .filter(|n| n.has_tag_name("features"))
            .collect();
        let feat_node = if feat_matches.len() > 1 {
            Ok(Err(InvalidCsl::new(
                &node,
                "Cannot have more than one <features> section",
            ))?)
        } else {
            // move matches into its first item
            Ok(feat_matches.into_iter().nth(0))
        };

        let features = match feat_node {
            Ok(Some(node)) => match Features::from_node(&node, default_info) {
                Ok(bib) => Some(bib),
                Err(err) => {
                    errors.push(err);
                    None
                }
            },
            Ok(None) => None,
            Err(e) => {
                errors.push(e);
                None
            }
        }
        .unwrap_or_else(Features::new);
        // Create our own info struct, ignoring the one passed in.
        let info = ParseInfo {
            features: features.clone(),
        };

        let locales_res = node
            .children()
            .filter(|n| n.is_element() && n.has_tag_name("locale"))
            .map(|el| Locale::from_node(&el, &info))
            .partition_results();
        match locales_res {
            Ok(locales) => {
                for loc in locales {
                    locale_overrides.insert(loc.lang.clone(), loc);
                }
            }
            Err(mut errs) => {
                errors.append(&mut errs);
            }
        }
        // TODO: output errors from macros, locales as well as citation and bibliography, if there are errors in
        // all
        let macro_res = node
            .children()
            .filter(|n| n.is_element() && n.has_tag_name("macro"))
            .map(|el| MacroMap::from_node(&el, &info))
            .partition_results();
        match macro_res {
            Ok(macro_maps) => {
                for mac in macro_maps {
                    macros.insert(mac.name, mac.elements);
                }
            }
            Err(mut errs) => {
                errors.append(&mut errs);
            }
        }
        let citation = match Citation::from_node(&get_toplevel(&node, "citation")?, &info) {
            Ok(cit) => Ok(cit),
            Err(err) => {
                errors.push(err);
                Err(CslError(Vec::new()))
            }
        };

        let matches: Vec<_> = node
            .children()
            .filter(|n| n.has_tag_name("bibliography"))
            .collect();

        let bib_node = if matches.len() > 1 {
            Ok(Err(InvalidCsl::new(
                &node,
                "Cannot have more than one <bibliography>",
            ))?)
        } else {
            // move matches into its first item
            Ok(matches.into_iter().nth(0))
        };

        let bibliography = match bib_node {
            Ok(Some(node)) => match Bibliography::from_node(&node, &info) {
                Ok(bib) => Some(bib),
                Err(err) => {
                    errors.push(err);
                    None
                }
            },
            Ok(None) => None,
            Err(e) => {
                errors.push(e);
                None
            }
        };

        if errors.len() > 0 {
            return Err(errors.into());
        }

        Ok(Style {
            macros,
            version_req,
            locale_overrides,
            default_locale: attribute_optional(node, "default-locale", &info)?,
            citation: citation?,
            features,
            bibliography,
            info: Info {},
            class: attribute_required(node, "class", &info)?,
            name_inheritance: Name::from_node(&node, &info)?,
            page_range_format: attribute_option(node, "page-range-format", &info)?,
            demote_non_dropping_particle: attribute_optional(node, "demote-non-dropping-particle", &info)?,
            initialize_with_hyphen: attribute_bool(node, "initialize-with-hyphen", true)?,
            names_delimiter: node
                .attribute("names-delimiter")
                .map(Atom::from)
                .map(Delimiter),
        })
    }
}

use roxmltree::Document;
use std::str::FromStr;
impl FromStr for Style {
    type Err = StyleError;
    fn from_str(xml: &str) -> Result<Self, Self::Err> {
        let doc = Document::parse(&xml)?;
        let info = ParseInfo::default();
        let style = Style::from_node(&doc.root_element(), &info)?;
        Ok(style)
    }
}
