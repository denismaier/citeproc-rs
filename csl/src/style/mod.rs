use super::terms::{TermForm, TermFormExtended, TextTermSelector};
use crate::error::*;
use crate::locale::{Lang, Locale};
use crate::terms::LocatorType;
use crate::variables::*;
use crate::version::{CslVersionReq, Features};
use crate::Atom;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

type TermPlural = bool;
type StripPeriods = bool;
type Quotes = bool;

#[derive(Debug, Eq, Clone, PartialEq)]
pub enum TextSource {
    Macro(Atom),
    Value(Atom),
    Variable(StandardVariable, VariableForm),
    Term(TextTermSelector, TermPlural),
}

#[derive(Debug, Eq, Clone, PartialEq)]
pub enum Element {
    /// <cs:text>
    Text(
        TextSource,
        Option<Formatting>,
        Affixes,
        Quotes,
        StripPeriods,
        TextCase,
        Option<DisplayMode>,
    ),
    /// <cs:label>
    Label(
        NumberVariable,
        TermForm,
        Option<Formatting>,
        Affixes,
        StripPeriods,
        TextCase,
        Plural,
    ),
    /// <cs:number>
    Number(
        NumberVariable,
        NumericForm,
        Option<Formatting>,
        Affixes,
        TextCase,
        Option<DisplayMode>,
    ),
    /// <cs:group>
    Group(Group),
    /// <cs:choose>
    /// Arc because the IR needs a reference to one, cloning deep trees is costly, and IR has
    /// to be in a Salsa db that doesn't really support lifetimes.
    Choose(Arc<Choose>),
    /// <cs:names>
    Names(Arc<Names>),
    /// <cs:date>
    Date(Arc<BodyDate>),
}

#[derive(Debug, Eq, Clone, PartialEq)]
pub struct Group {
    pub formatting: Option<Formatting>,
    pub delimiter: Delimiter,
    pub affixes: Affixes,
    pub elements: Vec<Element>,
    pub display: Option<DisplayMode>,
    /// CSL-M only
    pub is_parallel: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BodyDate {
    Indep(IndependentDate),
    Local(LocalizedDate),
}

/// e.g. for <text variable="title" form="short" />
#[derive(AsRefStr, EnumString, EnumProperty, Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[strum(serialize_all = "kebab_case")]
pub enum VariableForm {
    Long,
    Short,
}

impl Default for VariableForm {
    fn default() -> Self {
        VariableForm::Long
    }
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[strum(serialize_all = "kebab_case")]
pub enum NumericForm {
    Numeric,
    Ordinal,
    Roman,
    LongOrdinal,
}

impl Default for NumericForm {
    fn default() -> Self {
        NumericForm::Numeric
    }
}

#[derive(Clone, PartialEq, Eq)]
pub struct Affixes {
    pub prefix: Atom,
    pub suffix: Atom,
}

impl Default for Affixes {
    fn default() -> Self {
        Affixes {
            prefix: "".into(),
            suffix: "".into(),
        }
    }
}

#[derive(Eq, Copy, Clone, PartialEq)]
pub struct Formatting {
    pub font_style: FontStyle,
    pub font_variant: FontVariant,
    pub font_weight: FontWeight,
    pub vertical_alignment: VerticalAlignment,
    pub text_decoration: TextDecoration,
    // TODO: put this somewhere else, like directly on text nodes?
    // pub hyperlink: String,
}

impl Formatting {
    pub fn bold() -> Self {
        let mut f = Formatting::default();
        f.font_weight = FontWeight::Bold;
        f
    }
    pub fn italic() -> Self {
        let mut f = Formatting::default();
        f.font_style = FontStyle::Italic;
        f
    }
}

impl Default for Formatting {
    fn default() -> Self {
        Formatting {
            font_style: FontStyle::default(),
            font_variant: FontVariant::default(),
            font_weight: FontWeight::default(),
            text_decoration: TextDecoration::default(),
            vertical_alignment: VerticalAlignment::default(),
        }
    }
}

impl fmt::Debug for Affixes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Affixes {{ ")?;
        if !self.prefix.is_empty() {
            write!(f, "prefix: {:?}, ", self.prefix)?;
        }
        if !self.suffix.is_empty() {
            write!(f, "suffix: {:?}, ", self.suffix)?;
        }
        write!(f, "}}")
    }
}

impl fmt::Debug for Formatting {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let default = Formatting::default();
        write!(f, "Formatting {{ ")?;
        if self.font_style != default.font_style {
            write!(f, "font_style: {:?}, ", self.font_style)?;
        }
        if self.font_variant != default.font_variant {
            write!(f, "font_variant: {:?}, ", self.font_variant)?;
        }
        if self.font_weight != default.font_weight {
            write!(f, "font_weight: {:?}, ", self.font_weight)?;
        }
        if self.text_decoration != default.text_decoration {
            write!(f, "text_decoration: {:?}, ", self.text_decoration)?;
        }
        if self.vertical_alignment != default.vertical_alignment {
            write!(f, "vertical_alignment: {:?}, ", self.vertical_alignment)?;
        }
        write!(f, "}}")
    }
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[strum(serialize_all = "kebab_case")]
pub enum DisplayMode {
    Block,
    LeftMargin,
    RightInline,
    Indent,
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[strum(serialize_all = "kebab_case")]
pub enum TextCase {
    None,
    Lowercase,
    Uppercase,
    CapitalizeFirst,
    CapitalizeAll,
    Sentence,
    Title,
}

impl Default for TextCase {
    fn default() -> Self {
        TextCase::None
    }
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[strum(serialize_all = "kebab_case")]
pub enum FontStyle {
    Normal,
    Italic,
    Oblique,
}

impl Default for FontStyle {
    fn default() -> Self {
        FontStyle::Normal
    }
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[strum(serialize_all = "kebab_case")]
pub enum FontVariant {
    Normal,
    SmallCaps,
}

impl Default for FontVariant {
    fn default() -> Self {
        FontVariant::Normal
    }
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[strum(serialize_all = "kebab_case")]
pub enum FontWeight {
    Normal,
    Bold,
    Light,
}

impl Default for FontWeight {
    fn default() -> Self {
        FontWeight::Normal
    }
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[strum(serialize_all = "kebab_case")]
pub enum TextDecoration {
    None,
    Underline,
}

impl Default for TextDecoration {
    fn default() -> Self {
        TextDecoration::None
    }
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum VerticalAlignment {
    #[strum(serialize = "baseline")]
    Baseline,
    #[strum(serialize = "sup")]
    Superscript,
    #[strum(serialize = "sub")]
    Subscript,
}

impl Default for VerticalAlignment {
    fn default() -> Self {
        VerticalAlignment::Baseline
    }
}

#[derive(Default, Debug, Eq, Clone, PartialEq)]
pub struct Delimiter(pub Atom);

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq)]
#[strum(serialize_all = "kebab_case")]
pub enum Plural {
    Contextual,
    Always,
    Never,
}

impl Default for Plural {
    fn default() -> Self {
        Plural::Contextual
    }
}

/// [spec][]
///
/// [spec]: https://docs.citationstyles.org/en/stable/specification.html#choose
#[derive(Debug, Eq, Clone, PartialEq)]
pub struct Condition {
    pub match_type: Match,

    /// TODO: apparently CSL-M has disambiguate="check-ambiguity-and-backreference" as an
    /// option here. Frank alone knows what that means.
    /// https://github.com/Juris-M/citeproc-js/blob/30ceaf50a0ef86517a9a8cd46362e450133c7f91/src/attributes.js#L17-L46
    pub disambiguate: Option<bool>,

    /// It doesn't make much sense to test non-numeric variables, but the spec definitely says you
    /// can do it.
    pub is_numeric: Vec<AnyVariable>,
    pub variable: Vec<AnyVariable>,
    pub position: Vec<Position>,
    pub csl_type: Vec<CslType>,
    pub locator: Vec<LocatorType>,
    pub is_uncertain_date: Vec<DateVariable>,

    // TODO: do not populate in plain CSL mode
    pub jurisdiction: Option<Atom>,
    pub subjurisdictions: Option<u32>,

    /// https://citeproc-js.readthedocs.io/en/latest/csl-m/index.html#has-year-only-extension
    pub has_year_only: Vec<DateVariable>,
    /// https://citeproc-js.readthedocs.io/en/latest/csl-m/index.html#has-day-extension
    pub has_day: Vec<DateVariable>,
    /// https://citeproc-js.readthedocs.io/en/latest/csl-m/index.html#has-to-month-or-season-extension
    /// Original CSL-M is "has-to-month-or-season" which makes no sense.
    pub has_month_or_season: Vec<DateVariable>,
    pub context: Option<Context>,

    // undocumented CSL-M features
    // are there are more of these lurking in the citeproc-js codebase?

    // https://github.com/Juris-M/citeproc-js/blob/30ceaf50a0ef86517a9a8cd46362e450133c7f91/src/attributes.js#L599-L627
    pub is_plural: Vec<NameVariable>,
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq)]
#[strum(serialize_all = "kebab_case")]
pub enum Context {
    Citation,
    Bibliography,
}

impl Condition {
    pub fn is_empty(&self) -> bool {
        self.disambiguate.is_none()
            && self.is_numeric.is_empty()
            && self.variable.is_empty()
            && self.position.is_empty()
            && self.csl_type.is_empty()
            && self.locator.is_empty()
            && self.is_uncertain_date.is_empty()
            && self.has_year_only.is_empty()
            && self.has_day.is_empty()
            && self.has_month_or_season.is_empty()
            && self.jurisdiction.is_none()
            && self.subjurisdictions.is_none()
            && self.is_plural.is_empty()
            && self.context.is_none()
    }
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Clone, PartialEq, Eq)]
#[strum(serialize_all = "kebab_case")]
pub enum Match {
    Any,
    All,
    None,
    /// CSL-M only
    #[strum(props(csl = "0", cslM = "1"))]
    Nand,
}

impl Default for Match {
    fn default() -> Self {
        Match::Any
    }
}

#[derive(Debug, Eq, Clone, PartialEq)]
// in CSL 1.0.1, conditions.len() == 1
pub struct IfThen(pub Conditions, pub Vec<Element>);

#[derive(Debug, Eq, Clone, PartialEq)]
pub struct Conditions(pub Match, pub Vec<Condition>);

#[derive(Debug, Eq, Clone, PartialEq)]
pub struct Else(pub Vec<Element>);

#[derive(Debug, Eq, Clone, PartialEq)]
pub struct Choose(pub IfThen, pub Vec<IfThen>, pub Else);

#[derive(Debug, Eq, Clone, PartialEq)]
pub struct Names {
    // inheritable.
    pub delimiter: Option<Delimiter>,
    // non-inheritable
    pub variables: Vec<NameVariable>,
    pub name: Option<Name>,
    pub institution: Option<Institution>,
    pub label: Option<NameLabel>,
    pub et_al: Option<NameEtAl>,
    pub with: Option<NameWith>,
    pub substitute: Option<Substitute>,
    pub formatting: Option<Formatting>,
    pub display: Option<DisplayMode>,
    pub affixes: Affixes,
}

/// The available inheritable attributes for cs:name are and, delimiter-precedes-et-al,
/// delimiter-precedes-last, et-al-min, et-al-use-first, et-al-use-last, et-al-subsequent-min,
/// et-al-subsequent-use-first, initialize, initialize-with, name-as-sort-order and sort-separator.
/// The attributes name-form and name-delimiter correspond to the form and delimiter attributes on
/// cs:name. Similarly, names-delimiter corresponds to the delimiter attribute on cs:names.

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq)]
#[strum(serialize_all = "kebab_case")]
pub enum NameAnd {
    Text,
    Symbol,
}

/// It is not entirely clear which attributes `<cs:with>` supports.
#[derive(Debug, Eq, Clone, PartialEq, Default)]
pub struct NameWith {
    pub formatting: Option<Formatting>,
    pub affixes: Affixes,
}

#[derive(Debug, Eq, Clone, PartialEq, Default)]
pub struct Institution {
    pub and: Option<NameAnd>,
    pub delimiter: Option<Delimiter>,
    pub use_first: Option<InstitutionUseFirst>,
    /// This is different from the `*_use_last` on a Name, which is a boolean to activate `one,
    /// two,... last`.
    ///
    /// Instead, it plucks institution segments from the end in the same way use_first pulls from
    /// the start.
    pub use_last: Option<u32>,
    /// default is false
    pub reverse_order: bool,
    pub parts_selector: InstitutionParts,
    pub institution_parts: Vec<InstitutionPart>,
    // Not clearly part of the spec, but may be necessary.
    // pub formatting: Option<Formatting>,
    // pub affixes: Affixes,

    // TODO: suppress-min
}

#[derive(Debug, Eq, Clone, PartialEq, Default)]
pub struct InstitutionPart {
    pub name: InstitutionPartName,
    pub formatting: Option<Formatting>,
    pub affixes: Affixes,
    // TODO: is this better achieved using initialize-with?
    pub strip_periods: StripPeriods,
}

type IfShort = bool;

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq)]
#[strum(serialize_all = "kebab_case")]
pub enum InstitutionPartName {
    Long(IfShort),
    Short,
}

impl Default for InstitutionPartName {
    fn default() -> Self {
        InstitutionPartName::Long(false)
    }
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq)]
#[strum(serialize_all = "kebab_case")]
pub enum InstitutionParts {
    Long,
    Short,
    ShortLong,
    LongShort,
}

impl Default for InstitutionParts {
    fn default() -> Self {
        InstitutionParts::Long
    }
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq)]
#[strum(serialize_all = "kebab_case")]
pub enum InstitutionUseFirst {
    /// Set with `use-first="1"`
    Normal(u32),
    /// Set with `substitute-use-first="1"`
    ///
    /// The substitute-use-first attribute includes the leading (smallest) subunit if and only if
    /// no personal names are associated with the organization.
    Substitute(u32),
}

#[derive(Debug, Eq, Clone, PartialEq, Default)]
pub struct Name {
    pub and: Option<NameAnd>,
    pub delimiter: Option<Delimiter>,
    pub delimiter_precedes_et_al: Option<DelimiterPrecedes>,
    pub delimiter_precedes_last: Option<DelimiterPrecedes>,
    pub et_al_min: Option<u32>,
    pub et_al_use_first: Option<u32>,
    pub et_al_use_last: Option<bool>, // default is false
    pub et_al_subsequent_min: Option<u32>,
    pub et_al_subsequent_use_first: Option<u32>,
    pub form: Option<NameForm>,
    pub initialize: Option<bool>, // default is true
    pub initialize_with: Option<Atom>,
    pub name_as_sort_order: Option<NameAsSortOrder>,
    pub sort_separator: Option<Atom>,
    pub formatting: Option<Formatting>,
    pub affixes: Affixes,
    pub name_part_given: Option<NamePart>,
    pub name_part_family: Option<NamePart>,
}

impl Name {
    /// All properties on a Name may be inherited from elsewhere. Therefore while the
    /// `Default::default()` implementation will give you lots of `None`s, you need to define what
    /// those Nones should default to absent a parent giving a concrete definition.
    ///
    /// This follows how [citeproc-js][defaults] sets the defaults, because this is not specified
    /// in the spec(s).
    ///
    /// [defaults]: https://github.com/Juris-M/citeproc-js/blob/30ceaf50a0ef86517a9a8cd46362e450133c7f91/src/state.js#L103-L121
    pub fn root_default() -> Self {
        Name {
            and: None,
            delimiter: Some(Delimiter(",".into())),
            delimiter_precedes_et_al: Some(DelimiterPrecedes::Contextual),
            delimiter_precedes_last: Some(DelimiterPrecedes::Contextual),
            et_al_min: Some(0),
            et_al_use_first: Some(1),
            et_al_use_last: Some(false),
            et_al_subsequent_min: None, // must fall back to et_al_min
            et_al_subsequent_use_first: None, // must fall back to et_al_use_first
            // https://github.com/Juris-M/citeproc-js/blob/30ceaf50a0ef86517a9a8cd46362e450133c7f91/src/util_names_render.js#L710
            form: Some(NameForm::Long),
            initialize: Some(true),
            // https://github.com/Juris-M/citeproc-js/blob/30ceaf50a0ef86517a9a8cd46362e450133c7f91/src/util_names_render.js#L739
            initialize_with: None,
            name_as_sort_order: None,
            sort_separator: Some(", ".into()),
            // these four aren't inherited
            formatting: None,
            affixes: Default::default(),
            name_part_given: None,
            name_part_family: None,
        }
    }

    /// Takes an upstream Name definition, and merges it with a more local one that will
    /// override any fields set.
    ///
    /// Currently, also, it is not possible to override properties that don't accept a
    /// "none"/"default" option back to their default after setting it on a parent element.
    /// Like, once you set "name-as-sort-order", you cannot go back to Firstname Lastname.
    ///
    pub fn merge(&self, overrider: &Self) -> Self {
        Name {
            and: overrider.and.clone().or(self.and.clone()),
            delimiter: overrider.delimiter.clone().or(self.delimiter.clone()),
            delimiter_precedes_et_al: overrider
                .delimiter_precedes_et_al
                .or(self.delimiter_precedes_et_al),
            delimiter_precedes_last: overrider
                .delimiter_precedes_last
                .or(self.delimiter_precedes_last),
            et_al_min: overrider.et_al_min.or(self.et_al_min),
            et_al_use_first: overrider.et_al_use_first.or(self.et_al_use_first),
            et_al_use_last: overrider.et_al_use_last.or(self.et_al_use_last),
            et_al_subsequent_min: overrider.et_al_subsequent_min.or(self.et_al_subsequent_min),
            et_al_subsequent_use_first: overrider
                .et_al_subsequent_use_first
                .or(self.et_al_subsequent_use_first),
            form: overrider.form.or(self.form),
            initialize: overrider.initialize.or(self.initialize.clone()),
            initialize_with: overrider
                .initialize_with
                .clone()
                .or(self.initialize_with.clone()),
            name_as_sort_order: overrider.name_as_sort_order.or(self.name_as_sort_order),
            sort_separator: overrider
                .sort_separator
                .clone()
                .or(self.sort_separator.clone()),

            // these four aren't inherited
            formatting: overrider.formatting.clone(),
            affixes: overrider.affixes.clone(),
            name_part_given: overrider.name_part_given.clone(),
            name_part_family: overrider.name_part_family.clone(),
        }
    }
}

#[derive(Debug, Eq, Clone, PartialEq)]
pub struct NameLabel {
    pub form: TermFormExtended,
    pub formatting: Option<Formatting>,
    pub delimiter: Delimiter,
    pub plural: Plural,
    pub strip_periods: StripPeriods,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NameEtAl {
    // TODO: only accept "et-al" or "and others"
    pub term: String,
    pub formatting: Option<Formatting>,
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq)]
#[strum(serialize_all = "kebab_case")]
pub enum DemoteNonDroppingParticle {
    Never,
    SortOnly,
    DisplayAndSort,
}

impl Default for DemoteNonDroppingParticle {
    fn default() -> Self {
        DemoteNonDroppingParticle::DisplayAndSort
    }
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq)]
#[strum(serialize_all = "kebab_case")]
pub enum DelimiterPrecedes {
    Contextual,
    AfterInvertedName,
    Always,
    Never,
}

impl Default for DelimiterPrecedes {
    fn default() -> Self {
        DelimiterPrecedes::Contextual
    }
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq)]
#[strum(serialize_all = "kebab_case")]
pub enum NameForm {
    Long,
    Short,
    Count,
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq)]
#[strum(serialize_all = "kebab_case")]
pub enum NameAsSortOrder {
    First,
    All,
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq)]
#[strum(serialize_all = "kebab_case")]
pub enum NamePartName {
    Given,
    Family,
}

#[derive(Debug, Eq, Clone, PartialEq)]
pub struct NamePart {
    pub name: NamePartName,
    pub affixes: Affixes,
    pub text_case: TextCase,
    pub formatting: Option<Formatting>,
}

#[derive(Debug, Eq, Clone, PartialEq)]
pub struct Substitute(pub Vec<Element>);

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq)]
#[strum(serialize_all = "kebab_case")]
pub enum GivenNameDisambiguationRule {
    AllNames,
    AllNamesWithInitials,
    PrimaryName,
    PrimaryNameWithInitials,
    ByCite,
}

impl Default for GivenNameDisambiguationRule {
    fn default() -> Self {
        GivenNameDisambiguationRule::ByCite
    }
}

#[derive(Debug, Eq, Clone, PartialEq)]
pub struct Citation {
    pub disambiguate_add_names: bool,
    pub disambiguate_add_givenname: bool,
    pub givenname_disambiguation_rule: GivenNameDisambiguationRule,
    pub disambiguate_add_year_suffix: bool,
    pub layout: Layout,
    pub name_inheritance: Name,
    pub names_delimiter: Option<Delimiter>,
}

impl Default for Citation {
    fn default() -> Self {
        Citation {
            disambiguate_add_names: false,
            disambiguate_add_givenname: false,
            givenname_disambiguation_rule: Default::default(),
            disambiguate_add_year_suffix: false,
            layout: Default::default(),
            name_inheritance: Default::default(),
            names_delimiter: None,
        }
    }
}

#[derive(Debug, Eq, Clone, PartialEq)]
pub struct Bibliography {
    pub sort: Option<Sort>,
    pub layout: Layout,
    pub hanging_indent: bool, // default is false
    pub second_field_align: Option<SecondFieldAlign>,
    pub line_spaces: u32,   // >= 1 only. default is 1
    pub entry_spacing: u32, // >= 0. default is 1
    pub name_inheritance: Name,
    pub subsequent_author_substitute: Option<Atom>,
    pub subsequent_author_substitute_rule: SubstituteAuthorSubstituteRule,
    pub names_delimiter: Option<Delimiter>,
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Clone, PartialEq, Eq)]
#[strum(serialize_all = "kebab_case")]
pub enum SecondFieldAlign {
    Flush,
    Margin,
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Clone, PartialEq, Eq)]
#[strum(serialize_all = "kebab_case")]
pub enum SubstituteAuthorSubstituteRule {
    CompleteAll,
    CompleteEach,
    PartialEach,
    PartialFirst,
}

impl Default for SubstituteAuthorSubstituteRule {
    fn default() -> Self {
        SubstituteAuthorSubstituteRule::CompleteAll
    }
}

#[derive(Debug, Eq, Clone, PartialEq)]
pub struct Sort {
    pub keys: Vec<SortKey>,
}

#[derive(Debug, Eq, Clone, PartialEq)]
pub struct SortKey {
    pub sort_source: SortSource,
    pub names_min: Option<u32>,
    pub names_use_first: Option<u32>,
    pub names_use_last: Option<u32>,
    pub sort: Option<SortDirection>,
}

/// You must sort on either a variable or a macro
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortSource {
    Variable(AnyVariable),
    Macro(Atom),
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Clone, PartialEq, Eq, Hash)]
#[strum(serialize_all = "kebab_case")]
pub enum SortDirection {
    Ascending,
    Descending,
}

impl Default for SortDirection {
    fn default() -> Self {
        SortDirection::Ascending
    }
}

// TODO: Multiple layouts in CSL-M with locale="en es de" etc
#[derive(Default, Debug, Eq, Clone, PartialEq)]
pub struct Layout {
    pub affixes: Affixes,
    pub formatting: Option<Formatting>,
    // TODO: only allow delimiter inside <citation>
    pub delimiter: Delimiter,
    pub elements: Vec<Element>,
    pub locale: Vec<Lang>,
}

// Not actually part of a style tree, just a useful place to implement FromNode.
#[derive(Debug, Eq, Clone, PartialEq)]
pub struct MacroMap {
    pub name: Atom,
    pub elements: Vec<Element>,
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq)]
#[strum(serialize_all = "kebab_case")]
pub enum StyleClass {
    InText,
    Note,
}

impl Default for StyleClass {
    fn default() -> Self {
        StyleClass::Note
    }
}

use fnv::FnvHashMap;

#[derive(Default, Debug, Eq, Clone, PartialEq)]
pub struct Info {}

#[derive(Debug, Eq, Clone, PartialEq)]
pub struct Style {
    pub class: StyleClass,
    pub macros: FnvHashMap<Atom, Vec<Element>>,
    pub citation: Citation,
    pub bibliography: Option<Bibliography>,
    pub info: Info,
    pub features: Features,
    pub name_inheritance: Name,
    pub names_delimiter: Option<Delimiter>,
    /// `None` is the 'override everything' locale.
    pub locale_overrides: FnvHashMap<Option<Lang>, Locale>,
    pub default_locale: Lang,
    pub version_req: CslVersionReq,
    pub page_range_format: Option<PageRangeFormat>,
    pub demote_non_dropping_particle: DemoteNonDroppingParticle,
    pub initialize_with_hyphen: bool, // default is true
}

impl Default for Style {
    fn default() -> Self {
        Style {
            class: Default::default(),
            macros: Default::default(),
            citation: Default::default(),
            features: Default::default(),
            bibliography: Default::default(),
            info: Default::default(),
            name_inheritance: Default::default(),
            names_delimiter: None,
            locale_overrides: Default::default(),
            default_locale: Default::default(),
            version_req: CslVersionReq::current_csl(),
            page_range_format: None,
            demote_non_dropping_particle: Default::default(),
            initialize_with_hyphen: true,
        }
    }
}

#[derive(Debug, Eq, Clone, PartialEq)]
pub struct RangeDelimiter(pub Atom);

impl Default for RangeDelimiter {
    fn default() -> Self {
        RangeDelimiter("\u{2013}".into())
    }
}

impl std::convert::AsRef<str> for RangeDelimiter {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl FromStr for RangeDelimiter {
    type Err = UnknownAttributeValue;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(RangeDelimiter(s.into()))
    }
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq)]
#[strum(serialize_all = "kebab_case")]
pub enum DateParts {
    YearMonthDay,
    YearMonth,
    Year,
}

impl Default for DateParts {
    fn default() -> Self {
        DateParts::YearMonthDay
    }
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq)]
#[strum(serialize_all = "kebab_case")]
/// Strictly used for parsing day/month/year
pub(crate) enum DatePartName {
    Day,
    Month,
    Year,
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq)]
#[strum(serialize_all = "kebab_case")]
pub enum DayForm {
    Numeric,
    NumericLeadingZeros,
    Ordinal,
}
impl Default for DayForm {
    fn default() -> Self {
        DayForm::Numeric
    }
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq)]
#[strum(serialize_all = "kebab_case")]
pub enum MonthForm {
    Long,
    Short,
    Numeric,
    NumericLeadingZeros,
}
impl Default for MonthForm {
    fn default() -> Self {
        MonthForm::Long
    }
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq)]
#[strum(serialize_all = "kebab_case")]
pub enum YearForm {
    Long,
    Short,
}
impl Default for YearForm {
    fn default() -> Self {
        YearForm::Long
    }
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[strum(serialize_all = "kebab_case")]
pub enum DateForm {
    Text,
    Numeric,
}

#[derive(Debug, Display, Eq, Copy, Clone, PartialEq)]
pub enum DatePartForm {
    Day(DayForm),
    Month(MonthForm, StripPeriods),
    Year(YearForm),
}

#[derive(Debug, Eq, Clone, PartialEq)]
pub struct DatePart {
    pub form: DatePartForm,
    pub affixes: Affixes,
    pub formatting: Option<Formatting>,
    pub text_case: TextCase,
    pub range_delimiter: RangeDelimiter,
}

/// A date element that fully defines its own output.
/// It is 'independent' of any localization.
#[derive(Debug, Eq, Clone, PartialEq)]
pub struct IndependentDate {
    pub variable: DateVariable,
    // TODO: limit each <date-part name="XXX"> to one per?
    pub date_parts: Vec<DatePart>,
    pub delimiter: Delimiter,
    pub affixes: Affixes,
    pub formatting: Option<Formatting>,
    pub display: Option<DisplayMode>,
    pub text_case: TextCase,
}

/// A date element in the main body of a style that refers to a `LocaleDate`
#[derive(Debug, Eq, Clone, PartialEq)]
pub struct LocalizedDate {
    pub variable: DateVariable,
    pub parts_selector: DateParts,
    pub date_parts: Vec<DatePart>,
    pub form: DateForm,
    pub affixes: Affixes,
    pub formatting: Option<Formatting>,
    pub display: Option<DisplayMode>,
    pub text_case: TextCase,
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Copy, Clone, PartialEq, Eq, Hash)]
#[strum(serialize_all = "kebab_case")]
pub enum Position {
    First,
    Ibid,
    IbidWithLocator,
    Subsequent,
    NearNote,

    /// CSL-M only
    ///
    /// It [would
    /// appear](https://github.com/Juris-M/citeproc-js/blob/30ceaf50a0ef86517a9a8cd46362e450133c7f91/src/attributes.js#L165-L172)
    /// this means `subsequent && NOT near-note`, but it is not defined in any specification.
    #[strum(props(csl = "0", cslM = "1"))]
    FarNote,
}

impl Position {
    /// > "Whenever position=”ibid-with-locator” tests true, position=”ibid” also tests true.
    /// And whenever position=”ibid” or position=”near-note” test true, position=”subsequent”
    /// also tests true."
    ///
    /// [Spec](http://docs.citationstyles.org/en/stable/specification.html#choose)
    pub fn matches(self, in_cond: Self) -> bool {
        use self::Position::*;
        match (self, in_cond) {
            (IbidWithLocator, Ibid) => true,
            (IbidWithLocator, Subsequent) => true,
            (Ibid, Subsequent) => true,
            (FarNote, Subsequent) => true,
            (NearNote, Subsequent) => true,
            (x, y) => x == y,
        }
    }
}

/// [Spec](https://docs.citationstyles.org/en/stable/specification.html#appendix-v-page-range-formats)
#[derive(AsRefStr, EnumProperty, EnumString, Debug, Clone, PartialEq, Eq, Hash)]
#[strum(serialize_all = "kebab_case")]
pub enum PageRangeFormat {
    Chicago,
    Expanded,
    Minimal,
    MinimalTwo,
}

#[derive(AsRefStr, EnumProperty, EnumString, Debug, Clone, PartialEq, Eq, Hash)]
#[strum(serialize_all = "kebab_case")]
pub enum CslType {
    Article,
    ArticleMagazine,
    ArticleNewspaper,
    ArticleJournal,
    Bill,
    Book,
    Broadcast,
    Chapter,
    Dataset,
    Entry,
    EntryDictionary,
    EntryEncyclopedia,
    Figure,
    Graphic,
    Interview,
    Legislation,
    #[strum(serialize = "legal_case")]
    LegalCase,
    Manuscript,
    Map,
    #[strum(serialize = "motion_picture")]
    MotionPicture,
    #[strum(serialize = "musical_score")]
    MusicalScore,
    Pamphlet,
    PaperConference,
    Patent,
    Post,
    PostWeblog,
    #[strum(serialize = "personal_communication")]
    PersonalCommunication,
    Report,
    Review,
    ReviewBook,
    Song,
    Speech,
    Thesis,
    Treaty,
    Webpage,

    /// CSL-M only
    #[strum(props(csl = "0", cslM = "1"))]
    Classic,
    /// CSL-M only
    #[strum(props(csl = "0", cslM = "1"))]
    Gazette,
    /// CSL-M only
    #[strum(props(csl = "0", cslM = "1"))]
    Hearing,
    /// CSL-M only
    #[strum(props(csl = "0", cslM = "1"))]
    Regulation,
    /// CSL-M only
    #[strum(props(csl = "0", cslM = "1"))]
    Video,
}
